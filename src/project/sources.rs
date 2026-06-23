use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub fn java_sources(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    collect(root.as_ref(), &mut sources)?;
    sources.sort();
    Ok(sources)
}

fn collect(path: &Path, sources: &mut Vec<PathBuf>) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect(&path, sources)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            sources.push(path);
        }
    }

    Ok(())
}
