use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::cli::NewArgs;
use crate::output;
use crate::templates::{feature, manifest, manifests, render, TemplateContext, TemplateManifest};
use crate::ui;

const SUPPORTED_BUILD_TOOLS: &[&str] = &["maven", "gradle"];
pub fn run(args: NewArgs) -> Result<()> {
    let template = match args.template {
        Some(template) => template,
        None => {
            print_overview();
            return Ok(());
        }
    };

    if template == "list" {
        print_templates(args.verbose);
        return Ok(());
    }

    let Some(template_manifest) = manifest(&template) else {
        bail!("unknown template '{template}'; run 'jav new list' to see installed templates");
    };

    if args.describe {
        print_template_description(&template_manifest);
        return Ok(());
    }

    let name = match args.name {
        Some(name) => name,
        None => ui::input("Project name", Some("Demo"))?,
    };

    validate_project_name(&name)?;
    let build_tool = args
        .build_tool
        .unwrap_or_else(|| template_manifest.default_build_tool.clone());
    validate_build_tool(&build_tool)?;

    let package_name = match args.package {
        Some(package) => package,
        None => ui::input("Package name", Some(&default_package(&name)))?,
    };

    validate_package_name(&package_name)?;
    validate_template_options(&template_manifest, &args.features)?;

    let spring_features = resolve_features(&template_manifest, &args.features);
    let main_class = template_manifest
        .main_class
        .clone()
        .unwrap_or_else(|| "Main".to_string());

    let destination = PathBuf::from(args.output.unwrap_or_else(|| name.clone()));
    let context = TemplateContext {
        project_name: name,
        package_path: package_name.replace('.', "/"),
        package_name,
        build_tool,
        java_version: args.java_version,
        spring_boot_version: args.spring_boot_version,
        spring_features,
        main_class,
    };

    render::render(&template_manifest.id, &destination, context).with_context(|| {
        format!(
            "failed to create project from template '{}'",
            template_manifest.id
        )
    })?;

    output::status("created", destination.display().to_string());
    output::status("next", format!("cd {} && jav run", destination.display()));
    Ok(())
}

fn print_overview() {
    println!("Create a Java project from a template.\n");
    println!("Common templates:");
    print_template_table(&manifests());
    println!("\nExamples:");
    println!("  jav new console --name Demo");
    println!("  jav new springweb --name Api --build-tool gradle");
    println!("  jav new springboot --name Service --feature web --feature actuator");
    println!("  jav new springweb --describe");
    println!("\nShow all installed templates with:");
    println!("  jav new list --verbose");
}

fn print_templates(verbose: bool) {
    let templates = manifests();
    print_template_table(&templates);

    if verbose {
        println!();
        for template in templates {
            print_template_description(&template);
            println!();
        }
    }
}

fn print_template_table(templates: &[TemplateManifest]) {
    let name_width = templates
        .iter()
        .map(|template| template.name.len())
        .max()
        .unwrap_or(4)
        .max(13);
    let short_width = templates
        .iter()
        .map(|template| template.short_name.len())
        .max()
        .unwrap_or(5)
        .max(10);

    println!(
        "{:<name_width$}  {:<short_width$}  {:<8}  {:<7}  Tags",
        "Template Name", "Short Name", "Language", "Default"
    );
    println!(
        "{:-<name_width$}  {:-<short_width$}  {:-<8}  {:-<7}  {:-<24}",
        "", "", "", "", ""
    );

    for template in templates {
        println!(
            "{:<name_width$}  {:<short_width$}  {:<8}  {:<7}  {}",
            template.name,
            template.short_name,
            template.language,
            template.default_build_tool,
            template.tags.join("/")
        );
    }
}

fn print_template_description(template: &TemplateManifest) {
    println!("{} ({})", template.name, template.short_name);
    println!("{}", template.description);
    println!("Default build tool: {}", template.default_build_tool);
    println!("Tags: {}", template.tags.join("/"));

    if !template.aliases.is_empty() {
        println!("Aliases: {}", template.aliases.join(", "));
    }

    if !template.default_features.is_empty() {
        println!("Default features: {}", template.default_features.join(", "));
    }

    if !template.supported_features.is_empty() {
        println!("Supported features:");
        for id in &template.supported_features {
            if let Some(feature) = feature(id) {
                println!("  {:<12} {}", feature.id, feature.description);
            } else {
                println!("  {id}");
            }
        }
    }
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

fn validate_template_options(manifest: &TemplateManifest, features: &[String]) -> Result<()> {
    if manifest.supported_features.is_empty() && !features.is_empty() {
        bail!(
            "--feature is not supported for the '{}' template",
            manifest.id
        );
    }

    for feature in features {
        if !manifest
            .supported_features
            .iter()
            .any(|supported| supported == feature)
        {
            bail!(
                "unsupported feature '{feature}' for template '{}'; expected one of: {}",
                manifest.id,
                manifest.supported_features.join(", ")
            );
        }
    }

    Ok(())
}

fn resolve_features(manifest: &TemplateManifest, features: &[String]) -> Vec<String> {
    if !features.is_empty() {
        let mut resolved = manifest.default_features.clone();
        for feature in features {
            if !resolved.iter().any(|existing| existing == feature) {
                resolved.push(feature.clone());
            }
        }
        return resolved;
    }

    manifest.default_features.clone()
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
        let springboot = manifest("springboot").unwrap();
        let console = manifest("console").unwrap();

        assert_eq!(resolve_features(&springboot, &[]), vec!["web"]);
        assert!(resolve_features(&console, &[]).is_empty());
    }
}
