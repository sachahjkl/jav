use anyhow::{bail, Result};

use crate::process::CommandRunner;
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(runner: &impl CommandRunner) -> Result<()> {
    match detect_current()? {
        ProjectKind::Maven => {
            runner.run("mvn", &["test"])?;
        }
        ProjectKind::Gradle => {
            runner.run("gradle", &["test"])?;
        }
        ProjectKind::Simple => {
            bail!("simple Java projects do not have test support yet; use Maven or Gradle")
        }
    }

    Ok(())
}
