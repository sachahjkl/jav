use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::hash::{DefaultHasher, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
#[cfg(unix)]
use std::time::Instant;

use crate::cli::RunArgs;

pub fn run(args: RunArgs, verbose: bool) -> Result<()> {
    let stopped = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&stopped);
    ctrlc::set_handler(move || signal.store(true, Ordering::SeqCst))
        .context("cannot install Ctrl-C handler")?;
    let mut previous = snapshot()?;
    let mut process = RunningProcess::start(&args, verbose)?;
    eprintln!("Watching sources and build configuration. Press Ctrl-C to stop.");
    while !stopped.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(250));
        process.report_exit()?;
        let current = snapshot()?;
        if current != previous {
            // Wait for one stable sample before restarting after a group of writes.
            std::thread::sleep(Duration::from_millis(250));
            let settled = snapshot()?;
            if settled != current {
                continue;
            }
            previous = settled;
            process.stop()?;
            if stopped.load(Ordering::SeqCst) {
                break;
            }
            eprintln!("Project inputs changed; restarting.");
            process = RunningProcess::start(&args, verbose)?;
        }
    }
    process.stop()
}

struct RunningProcess {
    child: Child,
    stopped: bool,
    reported: bool,
}

impl RunningProcess {
    fn start(args: &RunArgs, verbose: bool) -> Result<Self> {
        let mut command = Command::new(std::env::current_exe()?);
        if verbose {
            command.arg("--verbose");
        }
        command.args(["run", "--configuration", args.configuration.as_str()]);
        if args.no_build {
            command.arg("--no-build");
        }
        if let Some(class) = &args.main_class {
            command.args(["--main-class", class]);
        }
        if !args.args.is_empty() {
            command.arg("--").args(&args.args);
        }
        // Keep Gradle within the watched process group instead of using a persistent daemon.
        let mut gradle_options = std::env::var_os("GRADLE_OPTS").unwrap_or_default();
        gradle_options.push(" -Dorg.gradle.daemon=false");
        command.env("GRADLE_OPTS", gradle_options);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command.spawn().context("cannot start watched jav run")?;
        Ok(Self {
            child,
            stopped: false,
            reported: false,
        })
    }

    fn report_exit(&mut self) -> Result<()> {
        if !self.reported {
            if let Some(status) = self.child.try_wait()? {
                eprintln!("Application exited with {status}; waiting for input changes.");
                self.reported = true;
            }
        }
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        if self.stopped {
            return Ok(());
        }
        #[cfg(unix)]
        {
            let group = i32::try_from(self.child.id()).context("invalid child process ID")?;
            signal_group(group, libc::SIGTERM)?;
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                self.child.try_wait()?;
                // Signal zero checks whether descendants still occupy the child's group.
                if unsafe { libc::kill(-group, 0) } == -1
                    && std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
                {
                    break;
                }
                if Instant::now() >= deadline {
                    signal_group(group, libc::SIGKILL)?;
                    break;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        #[cfg(windows)]
        {
            let status = Command::new("taskkill")
                .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
                .status()
                .context("cannot stop watched process tree")?;
            if !status.success() && self.child.try_wait()?.is_none() {
                anyhow::bail!("cannot stop watched process tree: {status}");
            }
        }
        #[cfg(not(any(unix, windows)))]
        self.child.kill()?;
        self.child
            .wait()
            .context("cannot wait for watched process")?;
        self.stopped = true;
        Ok(())
    }
}

impl Drop for RunningProcess {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("Cannot stop watched process: {error:#}");
        }
    }
}

#[cfg(unix)]
fn signal_group(group: i32, signal: i32) -> Result<()> {
    // The child starts a new process group whose ID is the child's positive process ID.
    let result = unsafe { libc::kill(-group, signal) };
    if result == -1 {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::ESRCH) {
            return Err(error).context("cannot signal watched process group");
        }
    }
    Ok(())
}

fn snapshot() -> Result<BTreeMap<PathBuf, u64>> {
    let mut files = BTreeMap::new();
    collect(Path::new("."), &mut files)?;
    Ok(files)
}

fn is_input(path: &Path) -> bool {
    if path.components().any(|part| part.as_os_str() == "src") {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name.to_ascii_lowercase().starts_with("readme") {
        return false;
    }
    path.components()
        .any(|part| matches!(part.as_os_str().to_str(), Some("gradle" | ".mvn")))
        || matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("java" | "kt" | "groovy" | "gradle")
        )
        || name.ends_with(".gradle.kts")
        || matches!(
            name,
            "pom.xml"
                | "gradle.properties"
                | "jav.toml"
                | "gradlew"
                | "gradlew.bat"
                | "mvnw"
                | "mvnw.cmd"
        )
}

fn collect(path: &Path, files: &mut BTreeMap<PathBuf, u64>) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| format!("cannot inspect {}", path.display()))
        }
    };
    if metadata.is_dir() {
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(error).with_context(|| format!("cannot read {}", path.display()))
            }
        };
        for entry in entries {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let in_sources = path.components().any(|part| part.as_os_str() == "src");
            if matches!(name.as_ref(), ".gradle" | ".git")
                || (!in_sources && matches!(name.as_ref(), "build" | "target" | "out"))
                || name.starts_with(".jav-build-")
            {
                continue;
            }
            collect(&entry.path(), files)?;
        }
    } else if metadata.is_file() && is_input(path) {
        let content = match fs::read(path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
            Err(error) => {
                return Err(error).with_context(|| format!("cannot read {}", path.display()))
            }
        };
        let mut hash = DefaultHasher::new();
        hash.write(&content);
        files.insert(path.to_owned(), hash.finish());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_edits_and_deletions_without_watching_outputs() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("Main.java");
        fs::write(&source, "before").unwrap();
        let mut before = BTreeMap::new();
        collect(directory.path(), &mut before).unwrap();
        fs::create_dir_all(directory.path().join("buildSrc/build")).unwrap();
        fs::write(directory.path().join("buildSrc/build/Main.class"), "output").unwrap();
        let mut outputs = BTreeMap::new();
        collect(directory.path(), &mut outputs).unwrap();
        assert_eq!(before, outputs);
        fs::write(&source, "after!").unwrap();
        let mut after = BTreeMap::new();
        collect(directory.path(), &mut after).unwrap();
        assert_ne!(before, after);
        fs::remove_file(source).unwrap();
        let mut deleted = BTreeMap::new();
        collect(directory.path(), &mut deleted).unwrap();
        assert!(deleted.is_empty());
    }

    #[test]
    fn watches_source_packages_and_resources_named_like_outputs() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        for file in [
            "src/main/java/example/build/Main.java",
            "src/main/resources/out/README.txt",
        ] {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "input").unwrap();
        }
        fs::create_dir_all(root.join("module/build")).unwrap();
        fs::write(root.join("module/build/Generated.java"), "output").unwrap();
        let mut inputs = BTreeMap::new();
        collect(root, &mut inputs).unwrap();
        assert_eq!(inputs.len(), 2);
        assert!(inputs.keys().all(|path| path.starts_with(root.join("src"))));
    }
}
