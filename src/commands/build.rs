use anyhow::{bail, Result};

use crate::process::CommandRunner;
use crate::project::sources::java_sources;
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(runner: &impl CommandRunner) -> Result<()> {
    match detect_current()? {
        ProjectKind::Maven => {
            runner.run("mvn", &["package"])?;
        }
        ProjectKind::Gradle => {
            runner.run("gradle", &["build"])?;
        }
        ProjectKind::Simple => {
            let sources = java_sources("src/main/java")?;

            if sources.is_empty() {
                bail!("no Java source files found under src/main/java");
            }

            std::fs::create_dir_all("out")?;

            let mut args = vec!["-d".to_string(), "out".to_string()];
            args.extend(
                sources
                    .into_iter()
                    .map(|source| source.display().to_string()),
            );
            runner.run_owned("javac", &args)?;
        }
    }

    Ok(())
}
