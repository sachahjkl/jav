use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::time::Duration;

fn project(root: &Path, build_file: Option<(&str, &str)>) {
    fs::create_dir_all(root.join("src/main/java/example")).unwrap();
    fs::write(
        root.join("src/main/java/example/Main.java"),
        "package example; public class Main { public static void main(String[] args) {} }",
    )
    .unwrap();
    if let Some((name, content)) = build_file {
        fs::write(root.join(name), content).unwrap();
    }
}

fn jav(root: &Path) -> Command {
    let mut command = Command::cargo_bin("jav").unwrap();
    command.current_dir(root).timeout(Duration::from_secs(10));
    command
}

#[test]
fn dry_run_does_not_create_configuration() {
    let root = tempfile::tempdir().unwrap();
    project(root.path(), None);
    jav(root.path())
        .args(["init", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("create jav.toml (dry run)"));
    assert!(!root.path().join("jav.toml").exists());
}

#[test]
fn initializes_maven_gradle_and_simple_projects() {
    for (build_file, task) in [
        (None, None),
        (
            Some(("pom.xml", "<project/>")),
            Some(("maven_task", "exec:java")),
        ),
        (
            Some(("build.gradle", "plugins { id 'application' }")),
            Some(("gradle_task", "run")),
        ),
        (
            Some((
                "pom.xml",
                "<project><artifactId>spring-boot</artifactId></project>",
            )),
            Some(("maven_task", "spring-boot:run")),
        ),
        (
            Some((
                "build.gradle.kts",
                "plugins { id(\"org.springframework.boot\") }",
            )),
            Some(("gradle_task", "bootRun")),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        project(root.path(), build_file);
        jav(root.path()).arg("init").assert().success();
        let config: toml::Value =
            toml::from_str(&fs::read_to_string(root.path().join("jav.toml")).unwrap()).unwrap();
        assert_eq!(config["run"]["main_class"].as_str(), Some("example.Main"));
        if let Some((key, value)) = task {
            assert_eq!(config["run"][key].as_str(), Some(value));
        }
    }
}

#[test]
fn existing_configuration_is_never_overwritten() {
    let root = tempfile::tempdir().unwrap();
    project(root.path(), None);
    let original = "# User configuration\n[run]\nmain_class = 'Custom'\n";
    fs::write(root.path().join("jav.toml"), original).unwrap();
    for args in [vec!["init"], vec!["init", "--dry-run"]] {
        jav(root.path())
            .args(args)
            .assert()
            .failure()
            .stderr(predicate::str::contains("already exists"));
        assert_eq!(
            fs::read_to_string(root.path().join("jav.toml")).unwrap(),
            original
        );
    }
}

#[test]
fn ambiguous_main_classes_do_not_create_configuration() {
    let root = tempfile::tempdir().unwrap();
    project(root.path(), None);
    fs::write(
        root.path().join("src/main/java/example/Other.java"),
        "package example; class Other { public static void main(String[] args) {} }",
    )
    .unwrap();
    jav(root.path())
        .arg("init")
        .assert()
        .failure()
        .stderr(predicate::str::contains("multiple main classes found"))
        .stderr(predicate::str::contains("example.Main"))
        .stderr(predicate::str::contains("example.Other"));
    assert!(!root.path().join("jav.toml").exists());
}
