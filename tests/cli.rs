use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

#[test]
fn doctor_runs() {
    let mut command = Command::cargo_bin("jav").unwrap();

    command
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("jav"));
}

#[test]
fn upgrade_check_requires_no_download_when_nix_managed() {
    let mut command = Command::cargo_bin("jav").unwrap();

    command
        .env("JAV_UPGRADE_OWNER", "example")
        .env("JAV_UPGRADE_REPOSITORY", "jav")
        .arg("upgrade")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Upgrade jav"));
}

#[test]
fn new_console_creates_maven_project() {
    let temp = assert_fs::TempDir::new().unwrap();
    let project = temp.child("Demo");

    let mut command = Command::cargo_bin("jav").unwrap();
    command
        .current_dir(temp.path())
        .args([
            "new",
            "console",
            "--name",
            "Demo",
            "--package",
            "dev.example.demo",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("created"));

    project.child("pom.xml").assert(predicate::path::exists());
    project
        .child("src/main/java/dev/example/demo/Main.java")
        .assert(predicate::path::exists());
    project
        .child("src/test/java/dev/example/demo/MainTest.java")
        .assert(predicate::path::exists());
}

#[test]
fn new_console_gradle_creates_gradle_project() {
    let temp = assert_fs::TempDir::new().unwrap();
    let project = temp.child("GradleDemo");

    let mut command = Command::cargo_bin("jav").unwrap();
    command
        .current_dir(temp.path())
        .args([
            "new",
            "console",
            "--name",
            "GradleDemo",
            "--package",
            "dev.example.demo",
            "--build-tool",
            "gradle",
        ])
        .assert()
        .success();

    project
        .child("build.gradle.kts")
        .assert(predicate::path::exists());
    project
        .child("settings.gradle.kts")
        .assert(predicate::path::exists());
    project.child("pom.xml").assert(predicate::path::missing());
}

#[test]
fn new_springboot_creates_project_with_requested_features() {
    let temp = assert_fs::TempDir::new().unwrap();
    let project = temp.child("ApiDemo");

    let mut command = Command::cargo_bin("jav").unwrap();
    command
        .current_dir(temp.path())
        .args([
            "new",
            "springboot",
            "--name",
            "ApiDemo",
            "--package",
            "dev.example.api",
            "--feature",
            "web",
            "--feature",
            "actuator",
            "--feature",
            "security",
        ])
        .assert()
        .success();

    project.child("pom.xml").assert(predicate::path::exists());
    project
        .child("src/main/java/dev/example/api/Application.java")
        .assert(predicate::path::exists());
    project
        .child("src/main/java/dev/example/api/HelloController.java")
        .assert(predicate::path::exists());

    let pom = std::fs::read_to_string(project.child("pom.xml").path()).unwrap();
    assert!(pom.contains("spring-boot-starter-web"));
    assert!(pom.contains("spring-boot-starter-actuator"));
    assert!(pom.contains("spring-boot-starter-security"));
}

#[test]
fn new_rejects_spring_features_for_non_springboot_templates() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut command = Command::cargo_bin("jav").unwrap();

    command
        .current_dir(temp.path())
        .args([
            "new",
            "console",
            "--name",
            "Bad",
            "--package",
            "dev.example.bad",
            "--feature",
            "web",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("only supported for the springboot template"));
}

#[test]
fn new_library_rejects_invalid_package() {
    let temp = assert_fs::TempDir::new().unwrap();
    let mut command = Command::cargo_bin("jav").unwrap();

    command
        .current_dir(temp.path())
        .args(["new", "library", "--name", "Bad", "--package", "dev.1bad"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("package segment"));
}
