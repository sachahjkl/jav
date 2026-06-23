use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::cli::NewArgs;
use crate::output;
use crate::templates::{render, TemplateContext};
use crate::ui;

pub fn run(args: NewArgs) -> Result<()> {
    let template = match args.template {
        Some(template) => template,
        None => ui::select_template()?,
    };

    let name = match args.name {
        Some(name) => name,
        None => ui::input("Project name", Some("Demo"))?,
    };

    validate_project_name(&name)?;

    let package_name = match args.package {
        Some(package) => package,
        None => ui::input("Package name", Some(&default_package(&name)))?,
    };

    validate_package_name(&package_name)?;

    let destination = PathBuf::from(args.output.unwrap_or_else(|| name.clone()));
    let context = TemplateContext {
        project_name: name,
        package_path: package_name.replace('.', "/"),
        package_name,
        java_version: args.java_version,
    };

    render::render(&template, &destination, context)
        .with_context(|| format!("failed to create project from template '{template}'"))?;

    output::status("created", destination.display().to_string());
    Ok(())
}

fn default_package(name: &str) -> String {
    let normalized = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '.'
            }
        })
        .collect::<String>()
        .split('.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(".");

    format!("com.example.{normalized}")
}

fn validate_project_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("project name cannot be empty");
    }

    if name.contains('/') || name.contains('\\') {
        bail!("project name cannot contain path separators");
    }

    Ok(())
}

fn validate_package_name(package_name: &str) -> Result<()> {
    let mut parts = package_name.split('.').peekable();

    if parts.peek().is_none() {
        bail!("package name cannot be empty");
    }

    for part in parts {
        let mut chars = part.chars();
        let Some(first) = chars.next() else {
            bail!("package name cannot contain empty segments");
        };

        if !first.is_ascii_alphabetic() && first != '_' {
            bail!("package segment '{part}' must start with a letter or underscore");
        }

        if chars.any(|character| !character.is_ascii_alphanumeric() && character != '_') {
            bail!("package segment '{part}' contains invalid characters");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_package_names() {
        assert!(validate_package_name("dev.example.demo").is_ok());
        assert!(validate_package_name("dev..demo").is_err());
        assert!(validate_package_name("dev.1demo").is_err());
    }
}
