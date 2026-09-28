use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn java_sources(root: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    collect(root.as_ref(), &mut sources, &mut HashSet::new())?;
    sources.sort();
    Ok(sources)
}

fn collect(path: &Path, sources: &mut Vec<PathBuf>, visited: &mut HashSet<PathBuf>) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    if !visited.insert(path.canonicalize()?) {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect(&path, sources, visited)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "java")
        {
            sources.push(path);
        }
    }

    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

    #[test]
    fn directory_symlink_cycle_terminates() {
        let root = assert_fs::TempDir::new().unwrap();
        root.child("nested/Main.java").touch().unwrap();
        std::os::unix::fs::symlink(root.path(), root.child("nested/loop").path()).unwrap();
        assert_eq!(
            java_sources(root.path()).unwrap(),
            vec![root.child("nested/Main.java").path().to_path_buf()]
        );
    }
}
