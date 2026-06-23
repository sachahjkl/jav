use anyhow::{bail, Result};
use std::path::Path;

use crate::project::ProjectKind;

pub fn detect_current() -> Result<ProjectKind> {
    detect(std::env::current_dir()?)
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
