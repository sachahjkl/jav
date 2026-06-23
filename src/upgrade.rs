use anyhow::{anyhow, bail, Context, Result};
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, USER_AGENT};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use tar::Archive;
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
        .build()
        .context("failed to build HTTP client")
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    let digest = hasher.finalize();
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn prepare_replacement(asset_name: &str, archive_bytes: &[u8]) -> Result<PathBuf> {
    if asset_name.ends_with(".zip") {
        extract_zip_binary(archive_bytes)
    } else if asset_name.ends_with(".tar.gz") || asset_name.ends_with(".tgz") {
        extract_tar_gz_binary(archive_bytes)
    } else {
        let path = unique_temp_path(asset_name);
        fs::write(&path, archive_bytes)?;
        Ok(path)
    }
}

fn extract_zip_binary(archive_bytes: &[u8]) -> Result<PathBuf> {
    let reader = Cursor::new(archive_bytes);
    let mut archive = ZipArchive::new(reader).context("invalid zip release asset")?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = Path::new(entry.name())
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name.eq_ignore_ascii_case("jav.exe") || name == "jav" {
            let path = unique_temp_path(name);
            let mut file = File::create(&path)?;
            io::copy(&mut entry, &mut file)?;
            return Ok(path);
        }
    }

    bail!("archive is missing jav executable")
}

fn extract_tar_gz_binary(archive_bytes: &[u8]) -> Result<PathBuf> {
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
        if name == "jav" || name.eq_ignore_ascii_case("jav.exe") {
            let temp = unique_temp_path(name);
            let mut file = File::create(&temp)?;
            io::copy(&mut entry, &mut file)?;
            return Ok(temp);
        }
    }

    bail!("archive is missing jav executable")
}

fn unique_temp_path(name: &str) -> PathBuf {
    let file_name = format!("jav-upgrade-{}-{name}", std::process::id());
    env::temp_dir().join(file_name)
}

pub fn replace_executable(current: &Path, replacement: &Path) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let backup = current.with_extension("exe.bak");
        let script = current.with_extension("upgrade.cmd");
        let script_body =
            windows_replacement_script(replacement, current, &backup, std::process::id());
        fs::write(&script, script_body)
            .with_context(|| format!("failed to write {}", script.display()))?;
        std::process::Command::new("cmd")
            .args(["/C", script.to_string_lossy().as_ref()])
            .spawn()
            .context("failed to launch Windows replacement script")?;
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    {
        fs::copy(replacement, current)
            .with_context(|| format!("failed to replace {}", current.display()))?;
        let metadata = fs::metadata(current)?;
        let mut permissions = metadata.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o755);
            fs::set_permissions(current, permissions)?;
        }
        fs::remove_file(replacement).ok();
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
        "@echo off\r\nsetlocal\r\nset \"NEW={}\"\r\nset \"TARGET={}\"\r\nset \"BACKUP={}\"\r\nset \"PID={}\"\r\n:wait\r\ntasklist /FI \"PID eq %PID%\" 2>nul | find \"%PID%\" >nul\r\nif not errorlevel 1 (\r\n  timeout /t 1 /nobreak >nul\r\n  goto wait\r\n)\r\nif not exist \"%NEW%\" exit /b 1\r\nif exist \"%BACKUP%\" del /f /q \"%BACKUP%\" >nul 2>nul\r\nif exist \"%TARGET%\" move /Y \"%TARGET%\" \"%BACKUP%\" >nul\r\ncopy /Y \"%NEW%\" \"%TARGET%\" >nul\r\nif errorlevel 1 (\r\n  if exist \"%BACKUP%\" move /Y \"%BACKUP%\" \"%TARGET%\" >nul\r\n  exit /b 1\r\n)\r\ndel /f /q \"%NEW%\" >nul 2>nul\r\ndel /f /q \"%BACKUP%\" >nul 2>nul\r\ndel /f /q \"%~f0\" >nul 2>nul\r\n",
        replacement.display(),
        current.display(),
        backup.display(),
        pid
    )
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
