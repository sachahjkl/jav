use assert_cmd::Command;
use std::fs;
use std::process::Command as ProcessCommand;

fn generate(root: &std::path::Path, template: &str, tool: &str) -> std::path::PathBuf {
    let destination = root.join(format!("{template}-{tool}"));
    Command::cargo_bin("jav")
        .unwrap()
        .args([
            "new",
            template,
            "--name",
            "Example",
            "--package",
            "example.app",
            "--build-tool",
            tool,
            "--output",
        ])
        .arg(&destination)
        .assert()
        .success();
    destination
}

#[test]
fn rejects_unsafe_inputs_without_creating_destination() {
    let root = tempfile::tempdir().unwrap();
    for (option, value) in [
        ("--name", "../escape"),
        ("--name", "bad\"name"),
        ("--name", "."),
        ("--package", "app.class"),
        ("--package", "app._"),
        ("--java-version", "21;exit"),
        ("--spring-boot-version", "3.5.0\""),
    ] {
        let destination = root.path().join("result");
        let mut command = Command::cargo_bin("jav").unwrap();
        command
            .args(["new", "console", "--output"])
            .arg(&destination);
        if option != "--name" {
            command.args(["--name", "Example"]);
        }
        if option != "--package" {
            command.args(["--package", "example.app"]);
        }
        command.args([option, value]).assert().failure();
        assert!(!destination.exists());
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn generates_every_family_with_both_build_tools() {
    let root = tempfile::tempdir().unwrap();
    for tool in ["maven", "gradle"] {
        for template in [
            "console",
            "cli",
            "worker",
            "library",
            "junit",
            "springboot",
            "springweb",
            "springdata",
            "springsecurity",
            "springbatch",
        ] {
            let destination = generate(root.path(), template, tool);
            assert!(destination.join("flake.nix").exists());
            let readme = fs::read_to_string(destination.join("README.md")).unwrap();
            assert!(readme.contains("lock-deps"));
            if matches!(template, "library" | "junit") {
                assert!(!readme.contains("jav run"));
            }
        }
    }
}

#[test]
fn no_flake_readme_and_dry_run_match_output() {
    let root = tempfile::tempdir().unwrap();
    for dry_run in [true, false] {
        let destination = root.path().join("library");
        let mut command = Command::cargo_bin("jav").unwrap();
        command
            .args([
                "new",
                "library",
                "--name",
                "Library",
                "--package",
                "example.library",
                "--no-flake",
                "--output",
            ])
            .arg(&destination);
        if dry_run {
            command.arg("--dry-run");
        }
        command.assert().success();
        if dry_run {
            assert!(!destination.exists());
        } else {
            let readme = fs::read_to_string(destination.join("README.md")).unwrap();
            assert!(!readme.contains("nix "));
            assert!(!readme.contains("jav run"));
            assert!(!destination.join("flake.nix").exists());
        }
    }
}

fn java_end_to_end(tool: &str) {
    let root = tempfile::tempdir().unwrap();
    for template in [
        "console",
        "cli",
        "worker",
        "library",
        "junit",
        "springboot",
        "springweb",
        "springsecurity",
        "springdata",
        "springbatch",
    ] {
        let destination = generate(root.path(), template, tool);
        let (executable, arguments) = if tool == "maven" {
            ("mvn", vec!["-B", "package"])
        } else {
            ("gradle", vec!["--no-daemon", "build"])
        };
        let output = ProcessCommand::new(executable)
            .args(arguments)
            .current_dir(&destination)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{template}/{tool}:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let reports = destination.join(if tool == "maven" {
            "target/debug/surefire-reports"
        } else {
            "build/test-results/test"
        });
        assert!(fs::read_dir(reports).unwrap().any(|entry| entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "xml")));
        if template == "console" {
            let class = |configuration: &str| {
                destination.join(if tool == "maven" {
                    format!("target/{configuration}/classes/example/app/Main.class")
                } else {
                    "build/classes/java/main/example/app/Main.class".to_string()
                })
            };
            let debug = ProcessCommand::new("javap")
                .arg("-l")
                .arg(class("debug"))
                .output()
                .unwrap();
            assert!(debug.status.success());
            assert!(String::from_utf8_lossy(&debug.stdout).contains("LineNumberTable"));
            Command::cargo_bin("jav")
                .unwrap()
                .current_dir(&destination)
                .args(["run", "--configuration", "release"])
                .assert()
                .success();
            let release = ProcessCommand::new("javap")
                .arg("-l")
                .arg(class("release"))
                .output()
                .unwrap();
            assert!(release.status.success());
            assert!(!String::from_utf8_lossy(&release.stdout).contains("LineNumberTable"));
        }
        if template == "springbatch" {
            let jars = destination.join(if tool == "maven" {
                "target/debug"
            } else {
                "build/libs"
            });
            let jar = fs::read_dir(jars)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.extension().is_some_and(|extension| extension == "jar")
                        && !path.to_string_lossy().contains("-plain")
                })
                .unwrap();
            let output = ProcessCommand::new("timeout")
                .args(["60", "java", "-jar"])
                .arg(jar)
                .current_dir(&destination)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "batch startup failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("COMPLETED"));
        }
    }
}

#[test]
#[ignore = "requires Maven Central network access, Maven, JDK 21, and timeout"]
fn maven_end_to_end() {
    java_end_to_end("maven");
}

#[test]
#[ignore = "requires Maven Central network access, Gradle, JDK 21, and timeout"]
fn gradle_end_to_end() {
    java_end_to_end("gradle");
}

fn java_arguments(tool: &str) {
    let root = tempfile::tempdir().unwrap();
    let destination = generate(root.path(), "console", tool);
    fs::write(
        destination.join("src/main/java/example/app/Main.java"),
        r#"package example.app;
public class Main {
  public static void main(String[] args) {
    for (String arg : args) {
      System.out.println("ARG[" + arg + "]");
    }
  }
}
"#,
    )
    .unwrap();
    fs::remove_file(destination.join("src/test/java/example/app/MainTest.java")).unwrap();
    let mut arguments = vec!["two words", "it's", "say \"hello\"", "one\\two", "a,b"];
    if tool == "gradle" {
        arguments.push("");
    } else {
        Command::cargo_bin("jav")
            .unwrap()
            .current_dir(&destination)
            .args(["run", "--", ""])
            .assert()
            .failure()
            .stderr(predicates::str::contains(
                "cannot preserve empty application arguments",
            ));
    }
    let output = Command::cargo_bin("jav")
        .unwrap()
        .current_dir(&destination)
        .args(["run", "--"])
        .args(&arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "argument run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: Vec<_> = stdout
        .lines()
        .filter(|line| line.starts_with("ARG["))
        .collect();
    let expected: Vec<_> = arguments
        .iter()
        .map(|argument| format!("ARG[{argument}]"))
        .collect();
    assert_eq!(actual, expected, "argument parsing for {tool}");
}

#[test]
#[ignore = "requires Maven Central network access, Maven, and JDK 21"]
fn maven_arguments() {
    java_arguments("maven");
}

#[test]
#[ignore = "requires Maven Central network access, Gradle, and JDK 21"]
fn gradle_arguments() {
    java_arguments("gradle");
}
