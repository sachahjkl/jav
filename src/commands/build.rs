use anyhow::{bail, Result};

use crate::cli::{BuildArgs, Configuration};
use crate::process::CommandRunner;
use crate::project::sources::java_sources;
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(args: BuildArgs, runner: &impl CommandRunner) -> Result<()> {
    match detect_current()? {
        ProjectKind::Maven => {
            let mut mvn_args = vec!["package", profile(args.configuration)];
            if args.no_tests {
                mvn_args.push("-DskipTests");
            }
            runner.run("mvn", &mvn_args)?;
        }
        ProjectKind::Gradle => {
            let configuration = format!("-Pjav.configuration={}", args.configuration.as_str());
            let mut gradle_args = vec!["build", configuration.as_str()];
            if args.no_tests {
                gradle_args.push("-x");
                gradle_args.push("test");
            }
            runner.run("gradle", &gradle_args)?;
        }
        ProjectKind::Simple => {
            let sources = java_sources("src/main/java")?;

            if sources.is_empty() {
                bail!("no Java source files found under src/main/java");
            }

            std::fs::create_dir_all("out")?;

            let debug_flag = match args.configuration {
                Configuration::Debug => "-g",
                Configuration::Release => "-g:none",
            };
            let mut javac_args = vec![debug_flag.to_string(), "-d".to_string(), "out".to_string()];
            javac_args.extend(
                sources
                    .into_iter()
                    .map(|source| source.display().to_string()),
            );
            runner.run_owned("javac", &javac_args)?;
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
