use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::cli::{BuildArgs, RunArgs};
use crate::commands::build;
use crate::process::CommandRunner;
use crate::project::sources::java_sources;
use crate::project::{detect::detect_current, ProjectKind};

#[derive(Debug, Default, Deserialize)]
struct JavConfig {
    run: Option<RunConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct RunConfig {
    main_class: Option<String>,
    maven_task: Option<String>,
    gradle_task: Option<String>,
    #[serde(default)]
    args: Vec<String>,
}

pub fn run(args: RunArgs, runner: &impl CommandRunner) -> Result<()> {
    let kind = detect_current()?;
    let config = read_config()?;

    if !args.no_build && is_stale(kind)? {
        build::run(
            BuildArgs {
                configuration: args.configuration,
                no_tests: true,
            },
            runner,
        )?;
    }

    match kind {
        ProjectKind::Maven => {
            let task = config
                .run
                .as_ref()
                .and_then(|run| run.maven_task.as_deref())
                .unwrap_or("exec:java");
            let mut mvn_args = vec![
                task.to_string(),
                format!("-P{}", args.configuration.as_str()),
            ];
            let app_args = app_args(&config, &args.args);
            if !app_args.is_empty() {
                let property = if task == "spring-boot:run" {
                    "spring-boot.run.arguments"
                } else {
                    "exec.args"
                };
                mvn_args.push(format!("-D{property}={}", join_args(&app_args)));
            }
            runner.run_owned("mvn", &mvn_args)?;
        }
        ProjectKind::Gradle => {
            let task = config
                .run
                .as_ref()
                .and_then(|run| run.gradle_task.as_deref())
                .map(ToString::to_string)
                .unwrap_or_else(|| {
                    if is_spring_boot_gradle_project().unwrap_or(false) {
                        "bootRun".to_string()
                    } else {
                        "run".to_string()
                    }
                });
            let mut gradle_args = vec![
                task,
                format!("-Pjav.configuration={}", args.configuration.as_str()),
            ];
            let app_args = app_args(&config, &args.args);
            if !app_args.is_empty() {
                gradle_args.push("--args".to_string());
                gradle_args.push(join_args(&app_args));
            }
            runner.run_owned("gradle", &gradle_args)?;
        }
        ProjectKind::Simple => {
            let main_class = match args.main_class {
                Some(main_class) => main_class,
                None => config
                    .run
                    .as_ref()
                    .and_then(|run| run.main_class.clone())
                    .or(infer_main_class()?)
                    .with_context(|| {
                        "could not infer main class; pass --main-class com.example.Main"
                    })?,
            };
            let mut java_args = vec!["-cp".to_string(), "out".to_string(), main_class];
            java_args.extend(app_args(&config, &args.args));
            runner.run_owned("java", &java_args)?;
        }
    }

    Ok(())
}

fn read_config() -> Result<JavConfig> {
    let path = Path::new("jav.toml");
    if !path.is_file() {
        return Ok(JavConfig::default());
    }

    Ok(toml::from_str(&fs::read_to_string(path)?)?)
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

fn is_stale(kind: ProjectKind) -> Result<bool> {
    let inputs = project_inputs(kind)?;
    if inputs.is_empty() {
        return Ok(true);
    }

    let output = match kind {
        ProjectKind::Maven => PathBuf::from("target/classes"),
        ProjectKind::Gradle => PathBuf::from("build/classes/java/main"),
        ProjectKind::Simple => PathBuf::from("out"),
    };

    if !output.exists() {
        return Ok(true);
    }

    let newest_source = newest_modified(&inputs)?.unwrap_or(SystemTime::UNIX_EPOCH);
    let outputs = class_files(&output)?;
    let newest_output = newest_modified(&outputs)?.unwrap_or(SystemTime::UNIX_EPOCH);

    Ok(outputs.is_empty() || newest_source > newest_output)
}

fn project_inputs(kind: ProjectKind) -> Result<Vec<PathBuf>> {
    let mut inputs = java_sources("src/main/java")?;
    inputs.extend(java_sources("src/test/java")?);
    collect_files(Path::new("src/main/resources"), &mut inputs, |_| true)?;

    match kind {
        ProjectKind::Maven => {
            push_if_file(&mut inputs, "pom.xml");
            push_if_file(&mut inputs, "jav.toml");
        }
        ProjectKind::Gradle => {
            for file in [
                "build.gradle.kts",
                "build.gradle",
                "settings.gradle.kts",
                "settings.gradle",
                "gradle.properties",
                "jav.toml",
            ] {
                push_if_file(&mut inputs, file);
            }
        }
        ProjectKind::Simple => push_if_file(&mut inputs, "jav.toml"),
    }

    Ok(inputs)
}

fn push_if_file(inputs: &mut Vec<PathBuf>, path: &str) {
    let path = PathBuf::from(path);
    if path.is_file() {
        inputs.push(path);
    }
}

fn newest_modified(paths: &[PathBuf]) -> Result<Option<SystemTime>> {
    let mut newest = None;
    for path in paths {
        let modified = fs::metadata(path)?.modified()?;
        newest = Some(newest.map_or(modified, |newest: SystemTime| newest.max(modified)));
    }
    Ok(newest)
}

fn class_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files(root, &mut files, |path| {
        path.extension()
            .is_some_and(|extension| extension == "class")
    })?;
    Ok(files)
}

fn collect_files(
    path: &Path,
    files: &mut Vec<PathBuf>,
    include: impl Fn(&Path) -> bool + Copy,
) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, files, include)?;
        } else if include(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn is_spring_boot_gradle_project() -> Result<bool> {
    for file in ["build.gradle.kts", "build.gradle"] {
        let path = Path::new(file);
        if path.is_file() && fs::read_to_string(path)?.contains("org.springframework.boot") {
            return Ok(true);
        }
    }

    Ok(false)
}

fn join_args(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.is_empty()
                || arg.chars().any(|character| {
                    character.is_whitespace() || matches!(character, '\'' | '"' | '\\')
                })
            {
                format!("'{}'", arg.replace('\'', "'\\''"))
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn infer_main_class() -> Result<Option<String>> {
    for source in java_sources("src/main/java")? {
        let content = fs::read_to_string(&source)?;
        if !content.contains("public static void main") {
            continue;
        }

        let package = content
            .lines()
            .find_map(|line| line.trim().strip_prefix("package "))
            .and_then(|line| line.strip_suffix(';'));
        let class = source.file_stem().and_then(|stem| stem.to_str());

        if let Some(class) = class {
            return Ok(Some(match package {
                Some(package) => format!("{package}.{class}"),
                None => class.to_string(),
            }));
        }
    }

    Ok(None)
}
