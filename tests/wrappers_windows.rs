#![cfg(windows)]

use assert_cmd::Command;
use assert_fs::prelude::*;

fn wrapper(project: &assert_fs::TempDir, gradle: bool, body: &str) {
    project
        .child(if gradle {
            "build.gradle.kts"
        } else {
            "pom.xml"
        })
        .touch()
        .unwrap();
    project
        .child(if gradle { "gradlew.bat" } else { "mvnw.cmd" })
        .write_str(&format!("@echo off\r\n{body}\r\n"))
        .unwrap();
}

#[test]
fn native_windows_wrappers_preserve_exit_codes() {
    for gradle in [false, true] {
        let project = assert_fs::TempDir::new().unwrap();
        wrapper(&project, gradle, "exit /b 23");
        Command::cargo_bin("jav")
            .unwrap()
            .current_dir(project.path())
            .arg("build")
            .assert()
            .code(23);
    }
}

#[test]
fn native_windows_parent_wrappers_run_from_module_paths_with_spaces() {
    for gradle in [false, true] {
        let project = assert_fs::TempDir::new().unwrap();
        wrapper(
            &project,
            gradle,
            "echo module-wrapper > selected\r\nexit /b 23",
        );
        let module = project.child("module with spaces");
        module
            .child(if gradle {
                "build.gradle.kts"
            } else {
                "pom.xml"
            })
            .touch()
            .unwrap();
        Command::cargo_bin("jav")
            .unwrap()
            .arg("-C")
            .arg(module.path())
            .arg("build")
            .assert()
            .code(23);
        assert_eq!(
            std::fs::read_to_string(module.child("selected").path())
                .unwrap()
                .trim(),
            "module-wrapper"
        );
    }
}

#[test]
fn native_windows_wrappers_preserve_argument_boundaries() {
    for gradle in [false, true] {
        let project = assert_fs::TempDir::new().unwrap();
        wrapper(
            &project,
            gradle,
            "(\r\necho [%~1]\r\necho [%~2]\r\necho [%~3]\r\necho [%~4]\r\n)> arguments\r\nexit /b 0",
        );
        Command::cargo_bin("jav")
            .unwrap()
            .current_dir(project.path())
            .args(["test", "--filter", "example.Test with spaces"])
            .assert()
            .success();
        let arguments = std::fs::read_to_string(project.child("arguments").path()).unwrap();
        let lines: Vec<_> = arguments.lines().collect();
        assert_eq!(lines[0], "[test]");
        if gradle {
            assert_eq!(lines[1], "[-Pjav.configuration=debug]");
            assert_eq!(lines[2], "[--tests]");
            assert_eq!(lines[3], "[example.Test with spaces]");
        } else {
            assert_eq!(lines[1], "[-Pdebug]");
            assert_eq!(lines[2], "[-Dtest=example.Test with spaces]");
            assert_eq!(lines[3], "[]");
        }
    }
}
