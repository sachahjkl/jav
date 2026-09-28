use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use crate::project::ProjectKind;

pub fn detect_current() -> Result<ProjectKind> {
    detect(find_root(std::env::current_dir()?)?)
}

pub fn find_root(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path
        .as_ref()
        .canonicalize()
        .with_context(|| format!("cannot resolve {}", path.as_ref().display()))?;
    let start = if path.is_file() {
        path.parent().unwrap_or(&path)
    } else {
        &path
    };
    for parent in start.ancestors() {
        if detect(parent).is_ok() {
            return Ok(parent.to_path_buf());
        }
    }
    bail!("not in a Java project; expected pom.xml, build.gradle, or src/main/java")
}

pub fn detect(path: impl AsRef<Path>) -> Result<ProjectKind> {
    let path = path.as_ref();

    if path.join("pom.xml").is_file() {
        return Ok(ProjectKind::Maven);
    }

    if path.join("build.gradle.kts").is_file()
        || path.join("build.gradle").is_file()
        || path.join("settings.gradle.kts").is_file()
        || path.join("settings.gradle").is_file()
    {
        return Ok(ProjectKind::Gradle);
    }

    if path.join("src/main/java").is_dir() {
        return Ok(ProjectKind::Simple);
    }

    bail!("not in a Java project; expected pom.xml, build.gradle, or src/main/java")
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;

    #[test]
    fn finds_nearest_project_from_nested_directory() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("pom.xml").touch().unwrap();
        temp.child("module/build.gradle").touch().unwrap();
        temp.child("module/src/main/java").create_dir_all().unwrap();
        assert_eq!(
            find_root(temp.child("module/src/main/java").path()).unwrap(),
            temp.child("module").path().canonicalize().unwrap()
        );
        assert!(detect(temp.child("module/src").path()).is_err());
    }

    #[test]
    fn detects_maven_project() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("pom.xml").touch().unwrap();

        assert_eq!(detect(temp.path()).unwrap(), ProjectKind::Maven);
    }

    #[test]
    fn detects_gradle_project() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("build.gradle.kts").touch().unwrap();

        assert_eq!(detect(temp.path()).unwrap(), ProjectKind::Gradle);
    }

    #[test]
    fn detects_simple_project() {
        let temp = assert_fs::TempDir::new().unwrap();
        temp.child("src/main/java").create_dir_all().unwrap();

        assert_eq!(detect(temp.path()).unwrap(), ProjectKind::Simple);
    }
}
