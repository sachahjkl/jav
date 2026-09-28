use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::Configuration;
use crate::process::CommandRunner;
use crate::project::sources::java_sources;

const STATE_FILE: &str = ".jav-build.json";

#[derive(Deserialize, Serialize)]
struct BuildState {
    inputs: String,
    outputs: String,
}

pub fn is_stale(configuration: Configuration) -> Result<bool> {
    let state = match fs::read(Path::new("out").join(STATE_FILE)) {
        Ok(bytes) => match serde_json::from_slice::<BuildState>(&bytes) {
            Ok(state) => state,
            Err(_) => return Ok(true),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(true),
        Err(error) => return Err(error.into()),
    };
    Ok(state.inputs != input_hash(configuration)? || state.outputs != tree_hash(Path::new("out"))?)
}

pub fn build(configuration: Configuration, runner: &impl CommandRunner) -> Result<()> {
    let sources = java_sources("src/main/java")?;
    if sources.is_empty() {
        bail!("no Java source files found under src/main/java");
    }
    let debug_flag = match configuration {
        Configuration::Debug => "-g",
        Configuration::Release => "-g:none",
    };
    if runner.dry_run() {
        let mut args = vec![debug_flag.to_string(), "-d".into(), "out".into()];
        args.extend(sources.iter().map(|path| path.display().to_string()));
        runner.run_owned("javac", &args)?;
        println!("copy src/main/resources into out; record build inputs");
        return Ok(());
    }

    let inputs = input_hash(configuration)?;
    let stage = tempfile::Builder::new()
        .prefix(".jav-build-")
        .tempdir_in(".")?;
    let mut args = vec![
        debug_flag.to_string(),
        "-d".into(),
        stage.path().display().to_string(),
    ];
    args.extend(sources.iter().map(|path| path.display().to_string()));
    runner.run_owned("javac", &args)?;

    let resources = Path::new("src/main/resources");
    for file in files(resources)? {
        let relative = file.strip_prefix(resources)?;
        if relative == Path::new(STATE_FILE) {
            bail!("resource name {STATE_FILE} is reserved by jav");
        }
        let target = stage.path().join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&file, &target)
            .with_context(|| format!("failed to copy resource {}", file.display()))?;
    }
    let state = BuildState {
        inputs,
        outputs: tree_hash(stage.path())?,
    };
    fs::write(stage.path().join(STATE_FILE), serde_json::to_vec(&state)?)?;
    let output = Path::new("out");
    match fs::symlink_metadata(output) {
        Ok(metadata) => {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                bail!("build output 'out' must be a directory, not a file or symbolic link");
            }
            fs::remove_dir_all(output)?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    fs::rename(stage.path(), output).context("failed to publish Java build outputs")?;
    Ok(())
}

fn input_hash(configuration: Configuration) -> Result<String> {
    let mut hash = Sha256::new();
    hash.update(configuration.as_str());
    hash.update(tree_hash(Path::new("src/main/java"))?);
    hash.update(tree_hash(Path::new("src/main/resources"))?);
    if Path::new("jav.toml").is_file() {
        hash.update(fs::read("jav.toml")?);
    }
    // A different JDK selection requires a new compilation.
    hash.update(
        std::env::var_os("JAVA_HOME")
            .unwrap_or_default()
            .as_encoded_bytes(),
    );
    hash.update(
        std::env::var_os("PATH")
            .unwrap_or_default()
            .as_encoded_bytes(),
    );
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn tree_hash(root: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    for file in files(root)? {
        let relative = file.strip_prefix(root)?;
        if relative == Path::new(STATE_FILE) {
            continue;
        }
        let name = relative.as_os_str().as_encoded_bytes();
        let content = fs::read(&file)?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name);
        hash.update((content.len() as u64).to_le_bytes());
        hash.update(content);
    }
    Ok(hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    collect(root, &mut result)?;
    result.sort();
    Ok(result)
}

fn collect(root: &Path, result: &mut Vec<PathBuf>) -> Result<()> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() {
        bail!(
            "symbolic links are not supported in simple Java build inputs: {}",
            root.display()
        );
    }
    if metadata.is_file() {
        result.push(root.to_path_buf());
    } else if metadata.is_dir() {
        for entry in fs::read_dir(root)? {
            collect(&entry?.path(), result)?;
        }
    } else {
        bail!("unsupported build input: {}", root.display());
    }
    Ok(())
}
