use anyhow::Result;

use crate::process::CommandRunner;
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(runner: &impl CommandRunner) -> Result<()> {
    match detect_current()? {
        ProjectKind::Maven => {
            runner.run("mvn", &["clean"])?;
        }
        ProjectKind::Gradle => {
            runner.run("gradle", &["clean"])?;
        }
        ProjectKind::Simple => {
            if std::path::Path::new("out").exists() {
                std::fs::remove_dir_all("out")?;
            }
        }
    }

    Ok(())
}
