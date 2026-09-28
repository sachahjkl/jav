use anyhow::{bail, Result};

use crate::output;
use crate::process::CommandRunner;
use crate::project::{detect::detect_current, project_command, ProjectKind};
use crate::version::APP_VERSION;

pub fn run(runner: &impl CommandRunner) -> Result<()> {
    output::status("jav", APP_VERSION);
    let project = detect_current().ok();
    let mut problems = Vec::new();
    let mut java = None;
    let mut javac = None;
    for tool in ["java", "javac", "mvn", "gradle"] {
        if !runner.exists(tool) {
            output::warning(format!("{tool} was not found on PATH"));
            if project.is_some() && matches!(tool, "java" | "javac") {
                problems.push(format!(
                    "{tool} is required; install a JDK and add its bin directory to PATH"
                ));
            }
            continue;
        }
        let flag = if matches!(tool, "java" | "javac") {
            "-version"
        } else {
            "--version"
        };
        match runner.capture(tool, &[flag]) {
            Ok(version) => {
                output::status(tool, version.trim());
                if tool == "java" {
                    java = version_major(&version);
                }
                if tool == "javac" {
                    javac = version_major(&version);
                }
            }
            Err(error) => {
                output::warning(format!("{tool}: {error:#}"));
                if project.is_some() && matches!(tool, "java" | "javac") {
                    problems.push(format!(
                        "cannot query {tool}; check its executable and PATH"
                    ));
                }
            }
        }
    }

    match std::env::var("JAVA_HOME") {
        Ok(value) => {
            output::status("JAVA_HOME", &value);
            let suffix = if cfg!(windows) { ".exe" } else { "" };
            for tool in ["java", "javac"] {
                let path = std::path::Path::new(&value)
                    .join("bin")
                    .join(format!("{tool}{suffix}"));
                if !path.is_file() {
                    output::warning(format!("{} is missing", path.display()));
                    if project.is_some() {
                        problems.push(format!(
                            "JAVA_HOME has no {tool}; set JAVA_HOME to a complete JDK directory"
                        ));
                    }
                } else if let Some(path) = path.to_str() {
                    match runner.capture(path, &["-version"]) {
                        Ok(version) => {
                            output::status(&format!("JAVA_HOME {tool}"), version.trim());
                            let expected = if tool == "java" { java } else { javac };
                            if expected
                                .zip(version_major(&version))
                                .is_some_and(|(a, b)| a != b)
                            {
                                problems.push(format!("JAVA_HOME {tool} differs from PATH; set PATH to use JAVA_HOME/bin"));
                            }
                        }
                        Err(error) => problems.push(format!(
                            "JAVA_HOME {tool}: {error:#}; set JAVA_HOME to a working JDK"
                        )),
                    }
                }
            }
        }
        Err(_) => output::warning("JAVA_HOME is not set"),
    }
    if java.zip(javac).is_some_and(|(java, javac)| java != javac) {
        problems.push(
            "java and javac major versions differ; set PATH to use one JDK bin directory".into(),
        );
    }

    if let Some(kind) = project {
        output::status("project", kind.name());
        let program = project_command(kind)?;
        output::status("project command", &program);
        if !runner.exists(&program) {
            problems.push(format!("{program} is required; restore the project wrapper or install the build tool on PATH"));
        } else {
            if std::path::Path::new(&program)
                .file_name()
                .is_some_and(|name| {
                    matches!(
                        name.to_str(),
                        Some("mvnw" | "mvnw.cmd" | "gradlew" | "gradlew.bat")
                    )
                })
            {
                match runner.capture(&program, &["--version"]) {
                    Ok(version) => output::status("wrapper", version.trim()),
                    Err(error) => problems.push(format!(
                        "wrapper failed: {error:#}; check the wrapper files and its Java runtime"
                    )),
                }
            }
            match required_java(runner, kind, &program) {
                Ok(Some(required)) => {
                    output::status("Java target", required.to_string());
                    if javac.is_some_and(|actual| actual < required) {
                        output::warning(format!("PATH javac is older than target Java {required}; configure a compiler toolchain or install JDK {required}. The build tool can select a different compiler."));
                    }
                }
                Ok(None) => output::warning("Java target was not reported by the project"),
                Err(error) => output::warning(format!("cannot determine Java target: {error:#}")),
            }
        }
    } else {
        output::warning("no Java project detected in the current directory");
    }
    if project.is_some() && !problems.is_empty() {
        bail!("{}", problems.join("; "));
    }
    Ok(())
}

fn required_java(
    runner: &impl CommandRunner,
    kind: ProjectKind,
    program: &str,
) -> Result<Option<u32>> {
    // Build tools evaluate their own configuration, including inherited properties.
    match kind {
        ProjectKind::Maven => {
            for property in [
                "maven.compiler.release",
                "maven.compiler.target",
                "maven.compiler.source",
                "java.version",
            ] {
                let expression = format!("-Dexpression={property}");
                let value = runner.capture(
                    program,
                    &[
                        "-q",
                        "help:evaluate",
                        &expression,
                        "-DforceStdout",
                        "-Dstyle.color=never",
                    ],
                )?;
                if let Some(version) = java_major(value.trim()) {
                    return Ok(Some(version));
                }
            }
            Ok(None)
        }
        ProjectKind::Gradle => {
            let properties = runner.capture(program, &["-q", "properties", "--console=plain"])?;
            Ok(properties.lines().find_map(|line| {
                line.strip_prefix("targetCompatibility: ")
                    .and_then(java_major)
            }))
        }
        ProjectKind::Simple => Ok(None),
    }
}

fn java_major(value: &str) -> Option<u32> {
    let value = value.strip_prefix("1.").unwrap_or(value);
    let major = value.split(['.', '-', '+', '_']).next()?;
    major.parse().ok()
}

fn version_major(value: &str) -> Option<u32> {
    value.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        match fields.next()? {
            "java" | "openjdk" => {
                let next = fields.next()?;
                java_major(
                    if next == "version" {
                        fields.next()?
                    } else {
                        next
                    }
                    .trim_matches('"'),
                )
            }
            "javac" => java_major(fields.next()?),
            _ => None,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_java_versions_without_using_dates_or_warning_numbers() {
        assert_eq!(
            version_major("WARNING: warning 42\nopenjdk version \"21.0.2\" 2024-01-16"),
            Some(21)
        );
        assert_eq!(version_major("java version \"1.8.0_402\""), Some(8));
        assert_eq!(version_major("openjdk 25-ea 2025-09-16"), Some(25));
        assert_eq!(version_major("javac 17.0.10"), Some(17));
        assert_eq!(version_major("warning 21"), None);
    }
}
