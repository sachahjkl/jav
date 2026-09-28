use anyhow::{bail, Context, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::cli::InitArgs;
use crate::config::{infer_main_class, JavConfig, RunConfig};
use crate::project::{detect::detect_current, ProjectKind};

pub fn run(_args: InitArgs, dry_run: bool) -> Result<()> {
    if Path::new("jav.toml").try_exists()? {
        bail!("jav.toml already exists; refusing to overwrite it");
    }
    let kind = detect_current()?;
    let mut run = RunConfig {
        main_class: infer_main_class()?,
        ..RunConfig::default()
    };
    match kind {
        ProjectKind::Maven => {
            let spring = fs::read_to_string("pom.xml")?.contains("spring-boot");
            run.maven_task = Some(
                if spring {
                    "spring-boot:run"
                } else {
                    "exec:java"
                }
                .to_string(),
            );
        }
        ProjectKind::Gradle => {
            let mut spring = false;
            for file in ["build.gradle", "build.gradle.kts"] {
                if Path::new(file).is_file() {
                    spring |= fs::read_to_string(file)?.contains("org.springframework.boot");
                }
            }
            run.gradle_task = Some(if spring { "bootRun" } else { "run" }.to_string());
        }
        ProjectKind::Simple => {}
    }
    let content = toml::to_string_pretty(&JavConfig { run: Some(run) })?;
    if dry_run {
        println!("create jav.toml (dry run):\n{content}");
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open("jav.toml")
        .context("cannot create jav.toml")?;
    file.write_all(content.as_bytes())
        .context("cannot write jav.toml")?;
    println!("Created jav.toml");
    Ok(())
}
