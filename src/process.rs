use anyhow::{bail, Context, Result};
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
        let status = Command::new(program)
            .args(args)
            .status()
            .with_context(|| format!("failed to start {program}"))?;

        if !status.success() {
            bail!("{program} exited with {status}");
        }

        Ok(())
    }

    fn exists(&self, program: &str) -> bool {
        Command::new(program)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
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
}
