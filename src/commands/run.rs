use anyhow::{bail, Result};

use crate::process::CommandRunner;
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(runner: &impl CommandRunner) -> Result<()> {
    match detect_current()? {
        ProjectKind::Maven => {
            runner.run("mvn", &["exec:java"])?;
        }
        ProjectKind::Gradle => {
            runner.run("gradle", &["run"])?;
        }
        ProjectKind::Simple => {
            bail!("simple Java projects do not have run support yet; use Maven or Gradle")
        }
    }

    Ok(())
}
