use anyhow::Result;

use crate::output;
use crate::process::CommandRunner;
use crate::project::detect::detect_current;

pub fn run(runner: &impl CommandRunner) -> Result<()> {
    output::status("jav", env!("CARGO_PKG_VERSION"));

    for tool in ["java", "javac", "mvn", "gradle"] {
        if runner.exists(tool) {
            output::status(tool, "found");
        } else {
            output::warning(format!("{tool} was not found on PATH"));
        }
    }

    match std::env::var("JAVA_HOME") {
        Ok(value) => output::status("JAVA_HOME", value),
        Err(_) => output::warning("JAVA_HOME is not set"),
    }

    match detect_current() {
        Ok(kind) => output::status("project", kind.name()),
        Err(_) => output::warning("no Java project detected in the current directory"),
    }

    Ok(())
}
