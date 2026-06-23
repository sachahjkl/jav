use anyhow::{bail, Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub trait CommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<()>;
    fn run_owned(&self, program: &str, args: &[String]) -> Result<()> {
        let args = args.iter().map(String::as_str).collect::<Vec<_>>();
        self.run(program, &args)
    }

    fn exists(&self, program: &str) -> bool;
}

pub struct RealRunner;

impl CommandRunner for RealRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<()> {
        let resolved = resolve_program(program).unwrap_or_else(|| PathBuf::from(program));
        let status = Command::new(&resolved)
            .args(args)
            .status()
            .with_context(|| format!("failed to start {}", resolved.display()))?;

        if !status.success() {
            bail!("{program} exited with {status}");
        }

        Ok(())
    }

    fn exists(&self, program: &str) -> bool {
        resolve_program(program).is_some()
    }
}

fn resolve_program(program: &str) -> Option<PathBuf> {
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
                if resolved.is_file() {
                    return Some(resolved);
                }
            }
        }

        let direct = dir.join(program);
        if direct.is_file() {
            return Some(direct);
        }
    }

    None
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
        temp.child("mvn").touch().unwrap();

        let resolved = resolve_program_in("mvn", vec![temp.path().to_path_buf()], Vec::new());

        assert_eq!(resolved, Some(temp.child("mvn").path().to_path_buf()));
    }

    #[test]
    fn resolves_windows_style_suffixes_from_pathext() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("mvn.cmd").touch().unwrap();

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
        temp.child("mvn").touch().unwrap();
        temp.child("mvn.cmd").touch().unwrap();

        let resolved = resolve_program_in(
            "mvn",
            vec![temp.path().to_path_buf()],
            vec![".com".into(), ".exe".into(), ".bat".into(), ".cmd".into()],
        );

        assert_eq!(resolved, Some(temp.child("mvn.cmd").path().to_path_buf()));
    }
}
