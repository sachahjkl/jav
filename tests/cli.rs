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
