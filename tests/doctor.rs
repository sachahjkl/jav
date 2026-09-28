#![cfg(unix)]

use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::os::unix::fs::PermissionsExt;

fn tool(root: &assert_fs::TempDir, name: &str, output: &str) {
    let file = root.child(format!("bin/{name}"));
    file.write_str(&format!("#!/bin/sh\nprintf '%s\\n' '{output}' >&2\n"))
        .unwrap();
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
}

fn doctor(project: &assert_fs::TempDir, tools: &assert_fs::TempDir) -> Command {
    tools.child("bin").create_dir_all().unwrap();
    let mut command = Command::cargo_bin("jav").unwrap();
    command
        .current_dir(project.path())
        .env("PATH", tools.child("bin").path())
        .env_remove("JAVA_HOME")
        .arg("doctor");
    command
}

fn simple_project() -> assert_fs::TempDir {
    let project = assert_fs::TempDir::new().unwrap();
    project.child("src/main/java").create_dir_all().unwrap();
    project
}

#[test]
fn reports_versions_from_stderr_and_checks_java_home() {
    let project = simple_project();
    let tools = assert_fs::TempDir::new().unwrap();
    tool(&tools, "java", "openjdk version \"21.0.2\"");
    tool(&tools, "javac", "javac 21.0.2");
    doctor(&project, &tools)
        .env("JAVA_HOME", tools.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("openjdk version \"21.0.2\""))
        .stdout(predicate::str::contains("javac 21.0.2"))
        .stdout(predicate::str::contains("JAVA_HOME javac"));
}

#[test]
fn inconsistent_path_jdk_reports_a_corrective_action() {
    let project = simple_project();
    let tools = assert_fs::TempDir::new().unwrap();
    tool(&tools, "java", "openjdk version \"21.0.2\"");
    tool(&tools, "javac", "javac 17.0.10");
    doctor(&project, &tools)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "java and javac major versions differ",
        ))
        .stderr(predicate::str::contains("set PATH"));
}

#[test]
fn java_home_mismatch_reports_a_corrective_action() {
    let project = simple_project();
    let tools = assert_fs::TempDir::new().unwrap();
    let home = assert_fs::TempDir::new().unwrap();
    for (root, version) in [(&tools, "21"), (&home, "17")] {
        tool(root, "java", &format!("openjdk version \"{version}\""));
        tool(root, "javac", &format!("javac {version}"));
    }
    doctor(&project, &tools)
        .env("JAVA_HOME", home.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "differs from PATH; set PATH to use JAVA_HOME/bin",
        ));
}

#[test]
fn missing_or_non_executable_java_is_reported_without_running_it() {
    let project = simple_project();
    let tools = assert_fs::TempDir::new().unwrap();
    tool(&tools, "javac", "javac 21");
    for non_executable in [false, true] {
        if non_executable {
            tools
                .child("bin/java")
                .write_str("this file cannot run")
                .unwrap();
        }
        doctor(&project, &tools)
            .assert()
            .failure()
            .stdout(predicate::str::contains("java was not found on PATH"))
            .stderr(predicate::str::contains("install a JDK"))
            .stderr(predicate::str::contains("failed to start java").not());
    }
}

#[test]
fn outside_project_succeeds_when_tools_are_missing() {
    let outside = assert_fs::TempDir::new().unwrap();
    let tools = assert_fs::TempDir::new().unwrap();
    doctor(&outside, &tools)
        .assert()
        .success()
        .stdout(predicate::str::contains("no Java project detected"));
}

#[test]
fn gradle_target_does_not_require_matching_path_compiler() {
    let project = assert_fs::TempDir::new().unwrap();
    project
        .child("build.gradle.kts")
        .write_str("java { toolchain { languageVersion.set(JavaLanguageVersion.of(25)) } }")
        .unwrap();
    let tools = assert_fs::TempDir::new().unwrap();
    tool(&tools, "java", "openjdk version \"21\"");
    tool(&tools, "javac", "javac 21");
    tool(&tools, "gradle", "targetCompatibility: 25");
    doctor(&project, &tools)
        .assert()
        .success()
        .stdout(predicate::str::contains("Java target 25"))
        .stdout(predicate::str::contains(
            "build tool can select a different compiler",
        ));
}

#[test]
fn dry_run_does_not_capture_versions() {
    let project = simple_project();
    let tools = assert_fs::TempDir::new().unwrap();
    tool(&tools, "java", "must-not-be-captured");
    tool(&tools, "javac", "must-not-be-captured");
    doctor(&project, &tools)
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicate::str::contains("must-not-be-captured").not())
        .stderr(predicate::str::contains("-version"));
}

#[test]
fn module_build_uses_parent_wrapper_instead_of_global_tool() {
    for (build, wrapper, global) in [
        ("pom.xml", "mvnw", "mvn"),
        ("build.gradle", "gradlew", "gradle"),
    ] {
        let project = assert_fs::TempDir::new().unwrap();
        project.child(".git").create_dir_all().unwrap();
        project.child(format!("module/{build}")).touch().unwrap();
        let script = project.child(wrapper);
        script
            .write_str("#!/bin/sh\nprintf 'parent wrapper\\n'\n")
            .unwrap();
        std::fs::set_permissions(script.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        let tools = assert_fs::TempDir::new().unwrap();
        tool(&tools, global, "global tool must not execute");
        Command::cargo_bin("jav")
            .unwrap()
            .current_dir(project.child("module").path())
            .env("PATH", tools.child("bin").path())
            .arg("build")
            .assert()
            .success()
            .stdout("parent wrapper\n")
            .stderr(predicate::str::contains("global tool").not());
    }
}

#[test]
fn doctor_reports_parent_wrapper_version() {
    let project = assert_fs::TempDir::new().unwrap();
    project.child(".git").create_dir_all().unwrap();
    project.child("module/pom.xml").touch().unwrap();
    let script = project.child("mvnw");
    script.write_str("#!/bin/sh\nif [ \"$1\" = --version ]; then printf 'Parent Maven 3.9\\n'; else printf '21'; fi\n").unwrap();
    std::fs::set_permissions(script.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    let tools = assert_fs::TempDir::new().unwrap();
    tool(&tools, "java", "openjdk version \"21\"");
    tool(&tools, "javac", "javac 21");
    doctor(&project, &tools)
        .current_dir(project.child("module").path())
        .assert()
        .success()
        .stdout(predicate::str::contains("wrapper Parent Maven 3.9"));
}
