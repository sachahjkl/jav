use anyhow::Result;

use crate::cli::{BuildArgs, Configuration};
use crate::process::{project_command, CommandRunner};
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(args: BuildArgs, runner: &impl CommandRunner) -> Result<()> {
    crate::config::read()?;
    let kind = detect_current()?;
    match kind {
        ProjectKind::Maven => {
            let mut mvn_args = vec!["package", profile(args.configuration)];
            if args.no_tests {
                mvn_args.push("-DskipTests");
            }
            runner.run(&project_command(kind)?, &mvn_args)?;
        }
        ProjectKind::Gradle => {
            let configuration = format!("-Pjav.configuration={}", args.configuration.as_str());
            let mut gradle_args = vec!["build", configuration.as_str()];
            if args.no_tests {
                gradle_args.push("-x");
                gradle_args.push("test");
            }
            runner.run(&project_command(kind)?, &gradle_args)?;
        }
        ProjectKind::Simple => {
            super::simple::build(args.configuration, runner)?;
        }
    }

    Ok(())
}

fn profile(configuration: Configuration) -> &'static str {
    match configuration {
        Configuration::Debug => "-Pdebug",
        Configuration::Release => "-Prelease",
    }
}
