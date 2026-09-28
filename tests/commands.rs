use assert_cmd::Command;
use assert_fs::prelude::*;
use predicates::prelude::*;

fn jav(directory: &std::path::Path) -> Command {
    let mut command = Command::cargo_bin("jav").unwrap();
    command.current_dir(directory);
    command
}

#[cfg(unix)]
fn wrapper(project: &assert_fs::TempDir, kind: &str, body: &str) {
    let (build, script) = if kind == "maven" {
        ("pom.xml", "mvnw")
    } else {
        ("build.gradle.kts", "gradlew")
    };
    project.child(build).touch().unwrap();
    project
        .child(script)
        .write_str(&format!("#!/bin/sh\n{body}\n"))
        .unwrap();
    // Deliberately leave the wrapper non-executable, as in a downloaded archive.
}

#[cfg(unix)]
fn recorded(project: &assert_fs::TempDir) -> Vec<String> {
    std::fs::read_to_string(project.child("arguments").path())
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
#[cfg(unix)]
fn wrappers_receive_build_test_clean_and_run_arguments() {
    for kind in ["maven", "gradle"] {
        let project = assert_fs::TempDir::new().unwrap();
        wrapper(&project, kind, "printf '%s\\n' \"$@\" > arguments");
        jav(project.path())
            .args(["build", "--configuration", "release", "--no-tests"])
            .assert()
            .success();
        let args = recorded(&project);
        assert_eq!(args[0], if kind == "maven" { "package" } else { "build" });
        assert!(args.iter().any(|arg| arg.contains("release")));
        assert!(args.iter().any(|arg| arg
            == if kind == "maven" {
                "-DskipTests"
            } else {
                "test"
            }));

        jav(project.path())
            .args(["test", "--filter", "dev.example.*Test"])
            .assert()
            .success();
        let args = recorded(&project);
        assert_eq!(args[0], "test");
        assert!(args.iter().any(|arg| arg.contains("dev.example.*Test")));
        if kind == "gradle" {
            assert!(args.contains(&"--tests".into()));
        }

        jav(project.path()).arg("clean").assert().success();
        assert_eq!(recorded(&project), ["clean"]);

        jav(project.path())
            .args(["run", "--", "hello world", "can't"])
            .assert()
            .success();
        let args = recorded(&project);
        if kind == "maven" {
            assert_eq!(&args[..2], ["compile", "exec:java"]);
        } else {
            assert_eq!(args[0], "run");
            assert!(!args.contains(&"build".into()));
        }
        assert!(args.iter().any(|arg| arg.contains("hello world")));

        jav(project.path())
            .args(["run", "--no-build"])
            .assert()
            .success();
        let args = recorded(&project);
        assert!(!args.contains(&"compile".into()));
        if kind == "gradle" {
            assert!(args.contains(&"classes".into()));
            assert!(args.contains(&"compileJava".into()));
            assert!(args.contains(&"processResources".into()));
        }
    }
}

#[test]
#[cfg(unix)]
fn commands_find_the_root_and_honor_directory_option() {
    let project = assert_fs::TempDir::new().unwrap();
    wrapper(&project, "maven", "pwd > working-directory");
    let nested = project.child("src/main/java");
    nested.create_dir_all().unwrap();
    // A source subtree is not a second project root.
    jav(nested.path()).arg("build").assert().success();
    assert_eq!(
        std::fs::read_to_string(project.child("working-directory").path())
            .unwrap()
            .trim(),
        project.path().to_str().unwrap()
    );
    let outside = assert_fs::TempDir::new().unwrap();
    jav(outside.path())
        .arg("-C")
        .arg(nested.path())
        .arg("test")
        .assert()
        .success();
}

#[test]
#[cfg(unix)]
fn child_exit_code_is_preserved() {
    let project = assert_fs::TempDir::new().unwrap();
    wrapper(&project, "maven", "exit 23");
    jav(project.path()).arg("build").assert().code(23);
}

#[test]
#[cfg(unix)]
fn dry_run_does_not_execute_wrapper() {
    let project = assert_fs::TempDir::new().unwrap();
    wrapper(&project, "gradle", "touch executed");
    jav(project.path())
        .args(["--dry-run", "build"])
        .assert()
        .success()
        .stderr(predicate::str::contains("gradlew"));
    project.child("executed").assert(predicate::path::missing());
}

#[test]
#[cfg(unix)]
fn unknown_configuration_key_fails_before_execution() {
    let project = assert_fs::TempDir::new().unwrap();
    wrapper(&project, "maven", "touch executed");
    project
        .child("jav.toml")
        .write_str("[run]\nmain_clas = 'Main'\n")
        .unwrap();
    jav(project.path())
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains("main_clas"));
    project.child("executed").assert(predicate::path::missing());
}

#[test]
#[cfg(unix)]
fn configured_tasks_and_arguments_are_used_and_cli_arguments_take_precedence() {
    let project = assert_fs::TempDir::new().unwrap();
    wrapper(&project, "maven", "printf '%s\\n' \"$@\" > arguments");
    project
        .child("jav.toml")
        .write_str("[run]\nmaven_task = 'custom:run'\nargs = ['configured value']\n")
        .unwrap();
    jav(project.path())
        .args(["run", "--no-build"])
        .assert()
        .success();
    let args = recorded(&project);
    assert_eq!(args[0], "custom:run");
    assert!(args.iter().any(|arg| arg.contains("configured value")));
    jav(project.path())
        .args(["run", "--no-build", "--", "override"])
        .assert()
        .success();
    let args = recorded(&project);
    assert!(!args.iter().any(|arg| arg.contains("configured value")));
    assert!(args.iter().any(|arg| arg.contains("override")));
}

#[test]
#[cfg(unix)]
fn maven_rejects_unrepresentable_arguments_before_starting_the_tool() {
    let project = assert_fs::TempDir::new().unwrap();
    wrapper(&project, "maven", "touch executed");
    jav(project.path())
        .args(["run", "--", ""])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "cannot preserve empty application arguments",
        ));
    project
        .child("jav.toml")
        .write_str("[run]\nargs = ['']\n")
        .unwrap();
    jav(project.path())
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "cannot preserve empty application arguments",
        ));
    project.child("executed").assert(predicate::path::missing());
}

#[test]
fn dry_run_does_not_create_or_clean_simple_outputs() {
    let project = assert_fs::TempDir::new().unwrap();
    project
        .child("src/main/java/Main.java")
        .write_str("public class Main { public static void main(String[] args) {} }")
        .unwrap();
    jav(project.path())
        .args(["build", "--dry-run"])
        .assert()
        .success();
    project.child("out").assert(predicate::path::missing());
    project.child("out/keep").write_str("keep").unwrap();
    jav(project.path())
        .args(["clean", "--dry-run"])
        .assert()
        .success();
    project.child("out/keep").assert("keep");
}

#[test]
fn configured_main_class_resolves_multiple_entry_points() {
    let project = assert_fs::TempDir::new().unwrap();
    for name in ["First", "Second"] {
        project
            .child(format!("src/main/java/{name}.java"))
            .write_str(&format!(
                "public class {name} {{ public static void main(String[] args) {{}} }}"
            ))
            .unwrap();
    }
    jav(project.path())
        .args(["run", "--dry-run"])
        .assert()
        .failure();
    project
        .child("jav.toml")
        .write_str("[run]\nmain_class = 'Second'\n")
        .unwrap();
    jav(project.path())
        .args(["run", "--dry-run"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Second"));
}

#[test]
#[ignore = "requires a JDK; run through the Nix native-java check"]
fn simple_java_tracks_configuration_resources_deletions_and_outputs() {
    let project = assert_fs::TempDir::new().unwrap();
    project
        .child("src/main/java/Main.java")
        .write_str(
            r#"
public class Main {
    public static void main(String[] args) throws Exception {
        try (var input = Main.class.getResourceAsStream("/message.txt")) {
            System.out.println(new String(input.readAllBytes()));
        }
    }
}
"#,
        )
        .unwrap();
    project
        .child("src/main/java/Old.java")
        .write_str("public class Old {}\n")
        .unwrap();
    project
        .child("src/main/resources/message.txt")
        .write_str("hello")
        .unwrap();
    project
        .child("src/main/resources/deleted.txt")
        .write_str("old")
        .unwrap();
    jav(project.path())
        .arg("run")
        .assert()
        .success()
        .stdout("hello\n");
    project
        .child("out/Old.class")
        .assert(predicate::path::exists());
    let class = project.child("out/Main.class");
    let debug = std::fs::read(class.path()).unwrap();
    assert!(debug
        .windows(b"LineNumberTable".len())
        .any(|bytes| bytes == b"LineNumberTable"));
    let built = std::fs::metadata(class.path()).unwrap().modified().unwrap();
    jav(project.path())
        .args(["run", "--verbose"])
        .assert()
        .success()
        .stderr(predicate::str::contains("reusing out"));
    assert_eq!(
        built,
        std::fs::metadata(class.path()).unwrap().modified().unwrap()
    );

    jav(project.path())
        .args(["run", "--configuration", "release"])
        .assert()
        .success();
    let release = std::fs::read(class.path()).unwrap();
    assert!(!release
        .windows(b"LineNumberTable".len())
        .any(|bytes| bytes == b"LineNumberTable"));

    std::fs::remove_file(project.child("src/main/java/Old.java").path()).unwrap();
    std::fs::remove_file(project.child("src/main/resources/deleted.txt").path()).unwrap();
    project
        .child("src/main/resources/message.txt")
        .write_str("changed")
        .unwrap();
    jav(project.path())
        .arg("run")
        .assert()
        .success()
        .stdout("changed\n");
    project
        .child("out/Old.class")
        .assert(predicate::path::missing());
    project
        .child("out/deleted.txt")
        .assert(predicate::path::missing());

    std::fs::remove_file(class.path()).unwrap();
    jav(project.path())
        .arg("run")
        .assert()
        .success()
        .stdout("changed\n");
    project
        .child("src/main/java/Main.java")
        .write_str("invalid Java")
        .unwrap();
    jav(project.path()).arg("build").assert().failure();
    // A failed compilation keeps the last successful build available.
    jav(project.path())
        .args(["run", "--no-build", "--main-class", "Main"])
        .assert()
        .success()
        .stdout("changed\n");
    jav(project.path()).arg("clean").assert().success();
    project.child("out").assert(predicate::path::missing());
}

#[test]
#[ignore = "requires a JDK; run through the Nix native-java check"]
fn simple_java_preserves_application_exit_code() {
    let project = assert_fs::TempDir::new().unwrap();
    project
        .child("src/main/java/Main.java")
        .write_str(
            "public class Main { public static void main(String[] args) { System.exit(7); } }",
        )
        .unwrap();
    jav(project.path()).arg("run").assert().code(7);
}

#[test]
#[ignore = "requires Gradle and a JDK; run through the Nix native-java check"]
fn gradle_no_build_uses_existing_classes_and_resources() {
    let project = assert_fs::TempDir::new().unwrap();
    project
        .child("build.gradle")
        .write_str("plugins { id 'application' }\napplication { mainClass = 'Main' }\n")
        .unwrap();
    project
        .child("settings.gradle")
        .write_str("rootProject.name = 'RunTest'\n")
        .unwrap();
    project
        .child("src/main/java/Main.java")
        .write_str(
            r#"
public class Main {
    public static void main(String[] args) throws Exception {
        try (var input = Main.class.getResourceAsStream("/message.txt")) {
            System.out.println(new String(input.readAllBytes()));
        }
    }
}
"#,
        )
        .unwrap();
    project
        .child("src/main/resources/message.txt")
        .write_str("built resource")
        .unwrap();
    let gradle_home = tempfile::tempdir().unwrap();
    let mut first = jav(project.path());
    first
        .env("GRADLE_USER_HOME", gradle_home.path())
        .env("GRADLE_OPTS", "-Dorg.gradle.daemon=false")
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("built resource"));
    project
        .child("src/main/java/Main.java")
        .write_str("invalid Java")
        .unwrap();
    project
        .child("src/main/resources/message.txt")
        .write_str("unbuilt resource")
        .unwrap();
    jav(project.path())
        .env("GRADLE_USER_HOME", gradle_home.path())
        .env("GRADLE_OPTS", "-Dorg.gradle.daemon=false")
        .args(["run", "--no-build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("built resource"))
        .stdout(predicate::str::contains("unbuilt resource").not());
}
