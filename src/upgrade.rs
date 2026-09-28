use anyhow::{anyhow, bail, Context, Result};
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tar::Archive;
use tempfile::NamedTempFile;
use zip::ZipArchive;

use crate::version::APP_VERSION;

const DEFAULT_OWNER: &str = "sachahjkl";
const DEFAULT_REPOSITORY: &str = "jav";
const DEFAULT_ASSET_NAME: &str = "release.json";

#[derive(Debug, Clone)]
pub struct UpdateOptions {
    pub owner: String,
    pub repository: String,
    pub include_prerelease: bool,
    pub asset_name: String,
}

impl UpdateOptions {
    pub fn from_env() -> Self {
        Self {
            owner: env::var("JAV_UPGRADE_OWNER").unwrap_or_else(|_| DEFAULT_OWNER.to_string()),
            repository: env::var("JAV_UPGRADE_REPOSITORY")
                .unwrap_or_else(|_| DEFAULT_REPOSITORY.to_string()),
            include_prerelease: env::var("JAV_UPGRADE_PRERELEASE")
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
                .unwrap_or(false),
            asset_name: env::var("JAV_UPGRADE_ASSET")
                .unwrap_or_else(|_| DEFAULT_ASSET_NAME.to_string()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHubRelease {
    pub assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    pub commit: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    pub rid: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    pub sha256: String,
    pub url: String,
}

pub fn detect_rid() -> Result<&'static str> {
    match (env::consts::OS, env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux-x64"),
        ("windows", "x86_64") => Ok("win-x64"),
        _ => bail!(
            "automatic upgrade is unsupported on {}-{}",
            env::consts::OS,
            env::consts::ARCH
        ),
    }
}

pub fn current_executable() -> Result<PathBuf> {
    env::current_exe().context("failed to resolve current executable path")
}

pub fn ensure_supported_host(executable: &Path) -> Result<()> {
    let display = executable.display().to_string();
    if display.contains("/nix/store/") || display.contains("\\nix\\store\\") {
        bail!(
            "auto-upgrade is unavailable for a Nix-managed install; use `nix run --refresh` or `nix profile upgrade`"
        );
    }

    Ok(())
}

pub fn fetch_latest_release(client: &Client, options: &UpdateOptions) -> Result<GitHubRelease> {
    let url = if options.include_prerelease {
        format!(
            "https://api.github.com/repos/{}/{}/releases",
            options.owner, options.repository
        )
    } else {
        format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            options.owner, options.repository
        )
    };

    let response = client
        .get(url)
        .send()
        .context("failed to query GitHub releases")?;
    let response = response
        .error_for_status()
        .context("GitHub releases request failed")?;

    if options.include_prerelease {
        let releases: Vec<GitHubRelease> = response
            .json()
            .context("invalid GitHub releases response")?;
        releases
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no GitHub releases found"))
    } else {
        response.json().context("invalid GitHub release response")
    }
}

pub fn download_manifest(
    client: &Client,
    release: &GitHubRelease,
    asset_name: &str,
) -> Result<ReleaseManifest> {
    let asset = release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .ok_or_else(|| anyhow!("release asset not found: {asset_name}"))?;

    client
        .get(&asset.browser_download_url)
        .send()
        .context("failed to download release manifest")?
        .error_for_status()
        .context("release manifest request failed")?
        .json()
        .context("invalid release manifest")
}

pub fn build_client() -> Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static("jav-upgrade/1.0"));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );

    Client::builder()
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(120))
        .build()
        .context("failed to build HTTP client")
}

pub fn verify_checksum(bytes: &[u8], expected: &str) -> Result<()> {
    if expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("invalid SHA256 in release manifest");
    }
    let hash: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if !hash.eq_ignore_ascii_case(expected) {
        bail!("invalid SHA256: expected {expected}, got {hash}");
    }
    Ok(())
}

pub fn is_newer_version(current: &str, available: &str) -> Result<bool> {
    let current = semver::Version::parse(current).context("invalid current version")?;
    let available = semver::Version::parse(available).context("invalid release version")?;
    Ok(available.cmp_precedence(&current).is_gt())
}

pub fn prepare_replacement(
    current: &Path,
    asset_name: &str,
    archive_bytes: &[u8],
) -> Result<NamedTempFile> {
    let directory = current
        .parent()
        .context("executable has no parent directory")?;
    let mut file = tempfile::Builder::new()
        .prefix(".jav-upgrade-")
        .tempfile_in(directory)
        .context("failed to create replacement beside executable")?;
    if asset_name.ends_with(".zip") {
        extract_zip_binary(archive_bytes, file.as_file_mut())?;
    } else if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        extract_tar_gz_binary(archive_bytes, file.as_file_mut())?;
    } else {
        file.write_all(archive_bytes)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(current)?.permissions().mode() & 0o777;
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(mode))?;
    }
    file.as_file().sync_all()?;
    Ok(file)
}

fn extract_zip_binary(archive_bytes: &[u8], file: &mut fs::File) -> Result<()> {
    let reader = Cursor::new(archive_bytes);
    let mut archive = ZipArchive::new(reader).context("invalid zip release asset")?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = Path::new(entry.name())
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if (name.eq_ignore_ascii_case("jav.exe") || name == "jav") && entry.is_file() {
            io::copy(&mut entry, file)?;
            return Ok(());
        }
    }

    bail!("archive is missing jav executable")
}

fn extract_tar_gz_binary(archive_bytes: &[u8], file: &mut fs::File) -> Result<()> {
    let reader = Cursor::new(archive_bytes);
    let decoder = GzDecoder::new(reader);
    let mut archive = Archive::new(decoder);

    for entry in archive.entries().context("invalid tar.gz release asset")? {
        let mut entry = entry?;
        let path = entry.path()?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if (name == "jav" || name.eq_ignore_ascii_case("jav.exe"))
            && entry.header().entry_type().is_file()
        {
            io::copy(&mut entry, file)?;
            return Ok(());
        }
    }

    bail!("archive is missing jav executable")
}

pub fn replace_executable(current: &Path, replacement: NamedTempFile) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let directory = tempfile::Builder::new()
            .prefix(".jav-upgrade-script-")
            .tempdir_in(
                current
                    .parent()
                    .context("executable has no parent directory")?,
            )?;
        let backup = directory.path().join("backup.exe");
        let script = directory.path().join("upgrade.cmd");
        let replacement = replacement.into_temp_path();
        let script_body =
            windows_replacement_script(&replacement, current, &backup, std::process::id());
        fs::write(&script, script_body)
            .with_context(|| format!("failed to write {}", script.display()))?;
        std::process::Command::new("cmd")
            .args(["/C", script.to_string_lossy().as_ref()])
            .spawn()
            .context("failed to launch Windows replacement script")?;
        replacement.keep()?;
        let _ = directory.keep();
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        replacement
            .persist(current)
            .with_context(|| format!("failed to replace {}", current.display()))?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn windows_replacement_script(
    replacement: &Path,
    current: &Path,
    backup: &Path,
    pid: u32,
) -> String {
    format!(
        r#"@echo off
setlocal
set "NEW={}"
set "TARGET={}"
set "BACKUP={}"
set "PID={}"
:wait
tasklist /FI "PID eq %PID%" 2>nul | find "%PID%" >nul
if not errorlevel 1 (
  timeout /t 1 /nobreak >nul
  goto wait
)
if not exist "%NEW%" goto cleanup
move /Y "%TARGET%" "%BACKUP%" >nul
if errorlevel 1 goto cleanup
move /Y "%NEW%" "%TARGET%" >nul
if errorlevel 1 (
  move /Y "%BACKUP%" "%TARGET%" >nul
  if errorlevel 1 exit /b 1
)
:cleanup
del /f /q "%NEW%" >nul 2>nul
del /f /q "%BACKUP%" >nul 2>nul
del /f /q "%~f0" >nul 2>nul & rmdir "%~dp0" >nul 2>nul
"#,
        replacement.display(),
        current.display(),
        backup.display(),
        pid
    )
    .replace('\n', "\r\n")
}

pub fn release_summary(manifest: &ReleaseManifest) -> Vec<String> {
    let mut lines = vec![format!(
        "latest version {}+{}",
        manifest.version, manifest.commit
    )];
    for asset in &manifest.assets {
        lines.push(format!(
            "{} {} {}",
            asset.rid, asset.file_name, asset.sha256
        ));
    }
    lines
}

pub fn current_version() -> &'static str {
    APP_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nix_store_installations_are_rejected() {
        let error = ensure_supported_host(Path::new("/nix/store/hash-jav/bin/jav")).unwrap_err();
        assert!(error.to_string().contains("Nix-managed install"));
        assert!(ensure_supported_host(Path::new("/home/user/.local/bin/jav")).is_ok());
    }

    #[test]
    fn only_newer_versions_are_selected() {
        for (current, available, expected) in [
            ("2026.624.1", "2026.624.2", true),
            ("2026.624.1", "2026.624.1", false),
            ("2026.624.1", "2026.623.9", false),
            ("1.9.0", "1.10.0", true),
            ("1.0.0", "1.0.0-rc.1", false),
            ("1.0.0-rc.1", "1.0.0", true),
            ("1.0.0+old", "1.0.0+new", false),
        ] {
            assert_eq!(is_newer_version(current, available).unwrap(), expected);
        }
        assert!(is_newer_version("1.0.0", "invalid").is_err());
        assert!(is_newer_version("invalid", "1.0.0").is_err());
    }

    #[test]
    fn checksum_rejects_corruption_and_invalid_manifest_values() {
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert!(verify_checksum(b"abc", expected).is_ok());
        assert!(verify_checksum(b"abc", &expected.to_uppercase()).is_ok());
        assert!(verify_checksum(b"abd", expected).is_err());
        assert!(verify_checksum(b"abc", "").is_err());
        assert!(verify_checksum(b"abc", &"z".repeat(64)).is_err());
    }

    #[test]
    fn replacement_files_are_unique_and_cleaned_up() {
        let directory = tempfile::tempdir().unwrap();
        let current = directory.path().join("jav");
        fs::write(&current, b"old").unwrap();
        let first = prepare_replacement(&current, "../../jav", b"first").unwrap();
        let second = prepare_replacement(&current, "../../jav", b"second").unwrap();
        assert_ne!(first.path(), second.path());
        assert_eq!(first.path().parent(), Some(directory.path()));
        assert_eq!(fs::read(first.path()).unwrap(), b"first");
        let first_path = first.path().to_owned();
        let second_path = second.path().to_owned();
        drop(first);
        drop(second);
        assert!(!first_path.exists());
        assert!(!second_path.exists());
        assert!(prepare_replacement(&current, "jav.tar.gz", b"invalid").is_err());
        assert!(prepare_replacement(&current, "jav.zip", b"invalid").is_err());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
        assert_eq!(fs::read(current).unwrap(), b"old");
    }

    #[test]
    fn extracts_binary_from_supported_archives() {
        let directory = tempfile::tempdir().unwrap();
        let current = directory.path().join("jav");
        fs::write(&current, b"old").unwrap();
        let payload = b"replacement executable";

        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(payload.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, "bin/jav", payload.as_slice())
            .unwrap();
        let tar_bytes = tar.into_inner().unwrap().finish().unwrap();

        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("bin/jav.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(payload).unwrap();
        let zip_bytes = zip.finish().unwrap().into_inner();

        for (name, bytes) in [("release.tar.gz", tar_bytes), ("release.zip", zip_bytes)] {
            let replacement = prepare_replacement(&current, name, &bytes).unwrap();
            assert_eq!(fs::read(replacement.path()).unwrap(), payload);
        }
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn archive_without_binary_leaves_no_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let current = directory.path().join("jav");
        fs::write(&current, b"old").unwrap();
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("README", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"readme").unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let error = prepare_replacement(&current, "release.zip", &bytes).unwrap_err();
        assert!(error.to_string().contains("missing jav executable"));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn replacement_preserves_access_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let current = directory.path().join("jav");
        fs::write(&current, b"old").unwrap();
        fs::set_permissions(&current, fs::Permissions::from_mode(0o750)).unwrap();
        let replacement = prepare_replacement(&current, "jav", b"new").unwrap();
        replace_executable(&current, replacement).unwrap();
        assert_eq!(
            fs::metadata(&current).unwrap().permissions().mode() & 0o7777,
            0o750
        );
        assert_eq!(fs::read(&current).unwrap(), b"new");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn failed_replacement_cleans_up_prepared_file() {
        let directory = tempfile::tempdir().unwrap();
        let current = directory.path().join("jav");
        fs::write(&current, b"old").unwrap();
        let replacement = prepare_replacement(&current, "jav", b"new").unwrap();
        let temporary = replacement.path().to_owned();
        let target = directory.path().join("directory");
        fs::create_dir(&target).unwrap();
        assert!(replace_executable(&target, replacement).is_err());
        assert!(!temporary.exists());
        assert_eq!(fs::read(current).unwrap(), b"old");
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "child process fixture for active executable replacement"]
    fn active_executable_fixture() {
        let Some(ready) = env::var_os("JAV_TEST_UPGRADE_READY") else {
            return;
        };
        fs::write(ready, b"ready").unwrap();
        std::thread::sleep(Duration::from_secs(60));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn replaces_active_linux_executable_atomically() {
        use std::os::unix::fs::MetadataExt;
        use std::process::{Child, Command, Stdio};
        use std::time::Instant;

        struct RunningFixture(Child);
        impl Drop for RunningFixture {
            fn drop(&mut self) {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }

        let directory = tempfile::tempdir().unwrap();
        let current = directory.path().join("jav");
        fs::copy(env::current_exe().unwrap(), &current).unwrap();
        let original_inode = fs::metadata(&current).unwrap().ino();
        let ready = directory.path().join("ready");
        let mut fixture = RunningFixture(
            Command::new(&current)
                .args([
                    "--exact",
                    "upgrade::tests::active_executable_fixture",
                    "--ignored",
                ])
                .env("JAV_TEST_UPGRADE_READY", &ready)
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            assert!(
                fixture.0.try_wait().unwrap().is_none(),
                "fixture exited before ready"
            );
            assert!(Instant::now() < deadline, "fixture did not become ready");
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(fs::OpenOptions::new().write(true).open(&current).is_err());
        let replacement = prepare_replacement(&current, "jav", b"new executable").unwrap();
        let temporary = replacement.path().to_owned();
        replace_executable(&current, replacement).unwrap();
        assert_eq!(fs::read(&current).unwrap(), b"new executable");
        assert_ne!(fs::metadata(&current).unwrap().ino(), original_inode);
        assert_eq!(
            fs::metadata(format!("/proc/{}/exe", fixture.0.id()))
                .unwrap()
                .ino(),
            original_inode
        );
        assert!(fixture.0.try_wait().unwrap().is_none());
        assert!(!temporary.exists());
    }
}
