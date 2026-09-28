use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub use crate::project::project_command;

pub trait CommandRunner {
    fn dry_run(&self) -> bool {
        false
    }
    fn verbose(&self) -> bool {
        false
    }
    fn run(&self, program: &str, args: &[&str]) -> Result<()>;
    fn run_owned(&self, program: &str, args: &[String]) -> Result<()> {
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        self.run(program, &args)
    }

    fn exists(&self, program: &str) -> bool;

    fn capture(&self, program: &str, args: &[&str]) -> Result<String> {
        self.run(program, args)?;
        Ok(String::new())
    }
}

#[derive(Debug)]
pub struct ProcessExit {
    pub code: i32,
}

impl std::fmt::Display for ProcessExit {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "process exited with code {}", self.code)
    }
}

impl std::error::Error for ProcessExit {}

#[derive(Default)]
pub struct RealRunner {
    dry_run: bool,
    verbose: bool,
}

impl RealRunner {
    pub fn new(dry_run: bool, verbose: bool) -> Self {
        Self { dry_run, verbose }
    }

    fn command(&self, program: &str, args: &[&str]) -> Result<Command> {
        let resolved = resolve_program(program).unwrap_or_else(|| PathBuf::from(program));
        let mut command = Command::new(&resolved);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let wrapper = resolved
                .file_name()
                .is_some_and(|name| name == "mvnw" || name == "gradlew");
            if wrapper && std::fs::metadata(&resolved)?.permissions().mode() & 0o111 == 0 {
                command = Command::new("sh");
                command.arg(&resolved);
            }
        }
        command.args(args);
        if self.dry_run || self.verbose {
            eprintln!(
                "cwd={:?} program={:?} argv={:?}",
                env::current_dir()?,
                command.get_program(),
                command.get_args().collect::<Vec<_>>()
            );
        }
        Ok(command)
    }
}

fn check_status(status: std::process::ExitStatus) -> Result<()> {
    if status.success() {
        return Ok(());
    }
    #[cfg(unix)]
    let code = {
        use std::os::unix::process::ExitStatusExt;
        status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(1))
    };
    #[cfg(not(unix))]
    let code = status.code().unwrap_or(1);
    Err(ProcessExit { code }.into())
}

impl CommandRunner for RealRunner {
    fn dry_run(&self) -> bool {
        self.dry_run
    }
    fn verbose(&self) -> bool {
        self.verbose
    }
    fn run(&self, program: &str, args: &[&str]) -> Result<()> {
        let mut command = self.command(program, args)?;
        if self.dry_run {
            return Ok(());
        }
        let status = command
            .status()
            .with_context(|| format!("failed to start {program}"))?;
        check_status(status).with_context(|| format!("{program} failed"))
    }

    fn exists(&self, program: &str) -> bool {
        resolve_program(program).is_some()
    }

    fn capture(&self, program: &str, args: &[&str]) -> Result<String> {
        let mut command = self.command(program, args)?;
        if self.dry_run {
            return Ok(String::new());
        }
        let output = command
            .output()
            .with_context(|| format!("failed to start {program}"))?;
        check_status(output.status).with_context(|| {
            format!(
                "{program} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
        })?;
        Ok(format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn resolve_program(program: &str) -> Option<PathBuf> {
    let candidate = Path::new(program);
    if candidate.is_absolute() || candidate.components().count() > 1 {
        return candidate.is_file().then(|| candidate.to_path_buf());
    }
    let path = env::var_os("PATH")?;
    resolve_program_in(
        program,
        env::split_paths(&path).collect::<Vec<_>>(),
        executable_extensions(),
    )
}

fn resolve_program_in(
    program: &str,
    path_dirs: Vec<PathBuf>,
    extensions: Vec<String>,
) -> Option<PathBuf> {
    let candidate = Path::new(program);
    if candidate.is_absolute() || candidate.components().count() > 1 {
        return candidate.is_file().then(|| candidate.to_path_buf());
    }

    for dir in path_dirs {
        if candidate.extension().is_none() {
            for extension in &extensions {
                let resolved = dir.join(format!("{program}{extension}"));
                if is_executable(&resolved) {
                    return Some(resolved);
                }
            }
        }

        let direct = dir.join(program);
        if is_executable(&direct) {
            return Some(direct);
        }
    }

    None
}

fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn executable_extensions() -> Vec<String> {
    if cfg!(windows) {
        env::var("PATHEXT")
            .ok()
            .map(|value| {
                value
                    .split(';')
                    .filter(|entry| !entry.is_empty())
                    .map(|entry| entry.to_ascii_lowercase())
                    .collect::<Vec<_>>()
            })
            .filter(|extensions| !extensions.is_empty())
            .unwrap_or_else(|| vec![".com".into(), ".exe".into(), ".bat".into(), ".cmd".into()])
    } else {
        Vec::new()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::cell::RefCell;

    fn executable(path: &Path) {
        std::fs::write(path, "").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[cfg(unix)]
    #[test]
    fn path_skips_non_executable_files() {
        let first = assert_fs::TempDir::new().unwrap();
        let second = assert_fs::TempDir::new().unwrap();
        first.child("java").touch().unwrap();
        executable(second.child("java").path());
        assert_eq!(
            resolve_program_in(
                "java",
                vec![first.path().into(), second.path().into()],
                vec![]
            ),
            Some(second.child("java").path().into())
        );
        assert!(resolve_program_in("java", vec![first.path().into()], vec![]).is_none());
    }

    #[test]
    fn dry_run_does_not_start_missing_program() {
        let runner = RealRunner::new(true, true);
        runner
            .run("jav-nonexistent-command-test", &["argument with spaces"])
            .unwrap();
        assert_eq!(
            runner.capture("jav-nonexistent-command-test", &[]).unwrap(),
            ""
        );
        assert!(runner.dry_run());
        assert!(runner.verbose());
    }

    #[cfg(unix)]
    #[test]
    fn runs_non_executable_wrapper_and_preserves_failure_code() {
        let temp = assert_fs::TempDir::new().unwrap();
        let wrapper = temp.child("mvnw");
        wrapper
            .write_str("#!/bin/sh\nprintf '%s' \"$1\"\nexit 23\n")
            .unwrap();
        let runner = RealRunner::new(false, false);
        let error = runner
            .run(wrapper.path().to_str().unwrap(), &["one argument"])
            .unwrap_err();
        assert_eq!(error.downcast_ref::<ProcessExit>().unwrap().code, 23);
        wrapper
            .write_str("#!/bin/sh\nprintf '%s' \"$1\"\n")
            .unwrap();
        assert_eq!(
            runner
                .capture(wrapper.path().to_str().unwrap(), &["one argument"])
                .unwrap(),
            "one argument"
        );
    }

    #[derive(Default)]
    pub struct RecordingRunner {
        pub commands: RefCell<Vec<(String, Vec<String>)>>,
        pub available: Vec<String>,
    }

    impl CommandRunner for RecordingRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<()> {
            self.commands.borrow_mut().push((
                program.to_string(),
                args.iter().map(|arg| arg.to_string()).collect(),
            ));

            Ok(())
        }

        fn exists(&self, program: &str) -> bool {
            self.available.iter().any(|available| available == program)
        }
    }

    #[test]
    fn recording_runner_records_commands_without_running_them() {
        let runner = RecordingRunner::default();

        runner.run("mvn", &["test"]).unwrap();

        assert_eq!(
            runner.commands.borrow().as_slice(),
            &[("mvn".to_string(), vec!["test".to_string()])]
        );
    }

    #[test]
    fn resolves_plain_executable_on_path() {
        let temp = assert_fs::TempDir::new().unwrap();
        executable(temp.child("mvn").path());

        let resolved = resolve_program_in("mvn", vec![temp.path().to_path_buf()], Vec::new());

        assert_eq!(resolved, Some(temp.child("mvn").path().to_path_buf()));
    }

    #[test]
    fn resolves_windows_style_suffixes_from_pathext() {
        let temp = assert_fs::TempDir::new().unwrap();
        executable(temp.child("mvn.cmd").path());

        let resolved = resolve_program_in(
            "mvn",
            vec![temp.path().to_path_buf()],
            vec![".com".into(), ".exe".into(), ".bat".into(), ".cmd".into()],
        );

        assert_eq!(resolved, Some(temp.child("mvn.cmd").path().to_path_buf()));
    }

    #[test]
    fn prefers_windows_script_suffix_over_plain_file() {
        let temp = assert_fs::TempDir::new().unwrap();
        executable(temp.child("mvn").path());
        executable(temp.child("mvn.cmd").path());

        let resolved = resolve_program_in(
            "mvn",
            vec![temp.path().to_path_buf()],
            vec![".com".into(), ".exe".into(), ".bat".into(), ".cmd".into()],
        );

        assert_eq!(resolved, Some(temp.child("mvn.cmd").path().to_path_buf()));
    }
}
