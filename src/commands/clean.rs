use anyhow::Result;

use crate::process::{project_command, CommandRunner};
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(runner: &impl CommandRunner) -> Result<()> {
    crate::config::read()?;
    let kind = detect_current()?;
    match kind {
        ProjectKind::Maven => {
            runner.run(&project_command(kind)?, &["clean"])?;
        }
        ProjectKind::Gradle => {
            runner.run(&project_command(kind)?, &["clean"])?;
        }
        ProjectKind::Simple => {
            if runner.dry_run() {
                println!("remove out");
                return Ok(());
            }
            if std::path::Path::new("out").exists() {
                std::fs::remove_dir_all("out")?;
            }
        }
    }

    Ok(())
}
