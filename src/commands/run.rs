use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use crate::cli::RunArgs;
use crate::config::{self, JavConfig};
use crate::process::{project_command, CommandRunner};
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(args: RunArgs, runner: &impl CommandRunner) -> Result<()> {
    let kind = detect_current()?;
    let config = config::read()?;
    let app_args = app_args(&config, &args.args);
    match kind {
        ProjectKind::Maven => {
            let task = config
                .run
                .as_ref()
                .and_then(|run| run.maven_task.as_deref());
            let task = match task {
                Some(task) => task,
                None if is_spring_boot_project(kind)? => "spring-boot:run",
                None => "exec:java",
            };
            let goal = task.split('@').next().unwrap_or(task);
            let exec_java = goal == "exec:java"
                || (goal.starts_with("org.codehaus.mojo:exec-maven-plugin:")
                    && goal.ends_with(":java"));
            if exec_java && app_args.iter().any(String::is_empty) {
                bail!("Maven exec:java cannot preserve empty application arguments");
            }
            let mut command = Vec::new();
            if !args.no_build {
                explain(runner, "Maven checks compilation inputs before running");
                command.push("compile".to_string());
            }
            command.extend([
                task.to_string(),
                format!("-P{}", args.configuration.as_str()),
            ]);
            let spring = task == "spring-boot:run";
            if let Some(main) = config.run.as_ref().and_then(|run| run.main_class.as_ref()) {
                let property = if spring {
                    "spring-boot.run.main-class"
                } else {
                    "exec.mainClass"
                };
                command.push(format!("-D{property}={main}"));
            }
            if !app_args.is_empty() {
                let property = if spring {
                    "spring-boot.run.arguments"
                } else {
                    "exec.args"
                };
                command.push(format!("-D{property}={}", join_args(&app_args)));
            }
            runner.run_owned(&project_command(kind)?, &command)?;
        }
        ProjectKind::Gradle => {
            let task = config
                .run
                .as_ref()
                .and_then(|run| run.gradle_task.as_deref());
            let task = match task {
                Some(task) => task,
                None if is_spring_boot_project(kind)? => "bootRun",
                None => "run",
            };
            let mut command = vec![
                task.to_string(),
                format!("-Pjav.configuration={}", args.configuration.as_str()),
            ];
            if args.no_build {
                for task in ["classes", "compileJava", "processResources"] {
                    command.extend(["-x".into(), task.into()]);
                }
            } else {
                explain(
                    runner,
                    "Gradle checks the run task dependencies before running",
                );
            }
            if !app_args.is_empty() {
                command.extend(["--args".to_string(), join_args(&app_args)]);
            }
            runner.run_owned(&project_command(kind)?, &command)?;
        }
        ProjectKind::Simple => {
            let main_class = match args
                .main_class
                .or_else(|| config.run.as_ref().and_then(|run| run.main_class.clone()))
            {
                Some(main) => main,
                None => config::infer_main_class()?
                    .context("could not infer main class; pass --main-class com.example.Main")?,
            };
            if !args.no_build {
                if super::simple::is_stale(args.configuration)? {
                    explain(
                        runner,
                        "sources, resources, configuration, or outputs changed; rebuilding",
                    );
                    super::simple::build(args.configuration, runner)?;
                } else {
                    explain(
                        runner,
                        "build inputs and outputs are unchanged; reusing out",
                    );
                }
            }
            let mut command = vec!["-cp".to_string(), "out".to_string(), main_class];
            command.extend(app_args);
            runner.run_owned("java", &command)?;
        }
    }
    Ok(())
}

fn explain(runner: &impl CommandRunner, message: &str) {
    if runner.verbose() || runner.dry_run() {
        eprintln!("jav: {message}");
    }
}

fn app_args(config: &JavConfig, cli_args: &[String]) -> Vec<String> {
    if !cli_args.is_empty() {
        return cli_args.to_vec();
    }
    config
        .run
        .as_ref()
        .map(|run| run.args.clone())
        .unwrap_or_default()
}

fn is_spring_boot_project(kind: ProjectKind) -> Result<bool> {
    let files: &[&str] = match kind {
        ProjectKind::Maven => &["pom.xml"],
        ProjectKind::Gradle => &["build.gradle.kts", "build.gradle"],
        ProjectKind::Simple => &[],
    };
    for file in files {
        let path = Path::new(file);
        if path.is_file() && fs::read_to_string(path)?.contains("org.springframework.boot") {
            return Ok(true);
        }
    }
    Ok(false)
}

// Maven and Gradle parse quotes themselves. They do not use shell backslash escaping.
fn join_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| format!("'{}'", arg.replace('\'', "'\"'\"'")))
        .collect::<Vec<_>>()
        .join(" ")
}
