use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::cli::NewArgs;
use crate::output;
use crate::templates::{render, TemplateContext};
use crate::ui;

const SUPPORTED_BUILD_TOOLS: &[&str] = &["maven", "gradle"];
const SUPPORTED_SPRING_FEATURES: &[&str] = &[
    "web",
    "actuator",
    "data-jpa",
    "security",
    "lombok",
    "postgresql",
];

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
    validate_build_tool(&args.build_tool)?;

    let package_name = match args.package {
        Some(package) => package,
        None => ui::input("Package name", Some(&default_package(&name)))?,
    };

    validate_package_name(&package_name)?;
    validate_template_options(&template, &args.features)?;

    let spring_features = resolve_spring_features(&template, &args.features);

    let destination = PathBuf::from(args.output.unwrap_or_else(|| name.clone()));
    let context = TemplateContext {
        project_name: name,
        package_path: package_name.replace('.', "/"),
        package_name,
        build_tool: args.build_tool,
        java_version: args.java_version,
        spring_boot_version: args.spring_boot_version,
        spring_features,
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

fn validate_build_tool(build_tool: &str) -> Result<()> {
    if SUPPORTED_BUILD_TOOLS.contains(&build_tool) {
        Ok(())
    } else {
        bail!(
            "unsupported build tool '{build_tool}'; expected one of: {}",
            SUPPORTED_BUILD_TOOLS.join(", ")
        )
    }
}

fn validate_template_options(template: &str, features: &[String]) -> Result<()> {
    if template != "springboot" && !features.is_empty() {
        bail!("--feature is only supported for the springboot template");
    }

    for feature in features {
        if !SUPPORTED_SPRING_FEATURES.contains(&feature.as_str()) {
            bail!(
                "unsupported springboot feature '{feature}'; expected one of: {}",
                SUPPORTED_SPRING_FEATURES.join(", ")
            );
        }
    }

    Ok(())
}

fn resolve_spring_features(template: &str, features: &[String]) -> Vec<String> {
    if template != "springboot" {
        return Vec::new();
    }

    if features.is_empty() {
        return vec!["web".to_string()];
    }

    let mut resolved = Vec::new();
    for feature in features {
        if !resolved.iter().any(|existing| existing == feature) {
            resolved.push(feature.clone());
        }
    }
    resolved
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

    #[test]
    fn validates_build_tools() {
        assert!(validate_build_tool("maven").is_ok());
        assert!(validate_build_tool("gradle").is_ok());
        assert!(validate_build_tool("ant").is_err());
    }

    #[test]
    fn springboot_defaults_to_web_feature() {
        assert_eq!(resolve_spring_features("springboot", &[]), vec!["web"]);
        assert!(resolve_spring_features("console", &[]).is_empty());
    }
}
