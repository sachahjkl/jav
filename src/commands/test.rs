use anyhow::{bail, Result};

use crate::cli::TestArgs;
use crate::process::{project_command, CommandRunner};
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(args: TestArgs, runner: &impl CommandRunner) -> Result<()> {
    crate::config::read()?;
    let kind = detect_current()?;
    let mut command = vec!["test".to_string()];
    match kind {
        ProjectKind::Maven => {
            command.push(format!("-P{}", args.configuration.as_str()));
            if let Some(filter) = args.filter {
                command.push(format!("-Dtest={filter}"));
            }
        }
        ProjectKind::Gradle => {
            command.push(format!(
                "-Pjav.configuration={}",
                args.configuration.as_str()
            ));
            if let Some(filter) = args.filter {
                command.extend(["--tests".into(), filter]);
            }
        }
        ProjectKind::Simple => {
            bail!("simple Java projects do not have test support; use Maven or Gradle");
        }
    }
    runner.run_owned(&project_command(kind)?, &command)
}
