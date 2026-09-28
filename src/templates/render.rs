use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use tera::{Context as TeraContext, Tera};

use crate::templates::{feature, files, manifest, TemplateContext};

pub fn render(template: &str, destination: &Path, context: TemplateContext) -> Result<()> {
    let manifest = manifest(template).with_context(|| format!("unknown template '{template}'"))?;

    if manifest.id != template {
        bail!(
            "template manifest id '{}' does not match requested template '{template}'",
            manifest.id
        );
    }

    if manifest.name.trim().is_empty()
        || manifest.description.trim().is_empty()
        || manifest.short_name.trim().is_empty()
        || manifest.language.trim().is_empty()
        || manifest.default_build_tool.trim().is_empty()
        || manifest.renderer.trim().is_empty()
        || manifest.tags.is_empty()
    {
        bail!("template '{template}' has incomplete metadata");
    }

    if destination.symlink_metadata().is_ok() {
        bail!("destination already exists: {}", destination.display());
    }

    let files = files(&manifest, &context)
        .with_context(|| format!("template '{template}' has no files"))?;
    let mut tera_context = tera_context(&context);
    tera_context.insert("is_spring", &(manifest.renderer == "springboot"));
    tera_context.insert(
        "is_runnable",
        &(!matches!(manifest.id.as_str(), "library" | "junit")),
    );
    tera_context.insert("include_flake", &context.include_flake);
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".jav-new-")
        .tempdir_in(parent)?;

    for (relative_path, content) in files {
        let rendered_path = Tera::one_off(relative_path, &tera_context, false)?;
        let target = staging.path().join(rendered_path);

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        let rendered = Tera::one_off(content, &tera_context, false)?;
        let rendered = rendered
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(target, format!("{}\n", rendered.trim_end()))?;
    }

    fs::rename(staging.path(), destination)?;
    Ok(())
}

fn tera_context(context: &TemplateContext) -> TeraContext {
    let mut tera_context = TeraContext::new();
    tera_context.insert("project_name", &context.project_name);
    tera_context.insert("package_name", &context.package_name);
    tera_context.insert("package_path", &context.package_path);
    tera_context.insert("java_version", &context.java_version);
    tera_context.insert("build_tool", &context.build_tool);
    tera_context.insert("is_maven", &(context.build_tool == "maven"));
    tera_context.insert("is_gradle", &(context.build_tool == "gradle"));
    tera_context.insert(
        "is_spring",
        &context.spring_features.iter().any(|feature| {
            matches!(
                feature.as_str(),
                "web" | "actuator" | "data-jpa" | "security" | "postgresql" | "batch"
            )
        }),
    );
    tera_context.insert("spring_boot_version", &context.spring_boot_version);
    tera_context.insert("main_class", &context.main_class);
    tera_context.insert(
        "spring_maven_dependencies",
        &spring_values(context, |feature| feature.maven_dependency),
    );
    tera_context.insert(
        "spring_gradle_dependencies",
        &spring_values(context, |feature| feature.gradle_dependency),
    );
    tera_context.insert(
        "spring_runtime_maven_dependencies",
        &spring_values(context, |feature| feature.runtime_maven_dependency),
    );
    tera_context.insert(
        "spring_runtime_gradle_dependencies",
        &spring_values(context, |feature| feature.runtime_gradle_dependency),
    );
    tera_context.insert(
        "spring_has_web",
        &context
            .spring_features
            .iter()
            .any(|feature| feature == "web"),
    );
    tera_context.insert(
        "spring_has_actuator",
        &context
            .spring_features
            .iter()
            .any(|feature| feature == "actuator"),
    );
    tera_context.insert(
        "spring_has_data_jpa",
        &context
            .spring_features
            .iter()
            .any(|feature| feature == "data-jpa"),
    );
    tera_context.insert(
        "spring_has_security",
        &context
            .spring_features
            .iter()
            .any(|feature| feature == "security"),
    );
    tera_context.insert(
        "spring_has_lombok",
        &context
            .spring_features
            .iter()
            .any(|feature| feature == "lombok"),
    );
    tera_context.insert(
        "spring_has_postgresql",
        &context
            .spring_features
            .iter()
            .any(|feature| feature == "postgresql"),
    );
    tera_context.insert(
        "spring_has_batch",
        &context
            .spring_features
            .iter()
            .any(|feature| feature == "batch"),
    );
    tera_context
}

fn spring_values(
    context: &TemplateContext,
    value: impl Fn(&crate::templates::SpringFeature) -> Option<&'static str>,
) -> Vec<&'static str> {
    context
        .spring_features
        .iter()
        .filter_map(|selected| feature(selected))
        .filter_map(value)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_generation_removes_staged_files() {
        let root = tempfile::tempdir().unwrap();
        let destination = root.path().join("project");
        let context = TemplateContext {
            project_name: "Example".into(),
            package_name: "example.app".into(),
            package_path: "invalid\0path".into(),
            java_version: "21".into(),
            build_tool: "maven".into(),
            spring_boot_version: "3.5.0".into(),
            spring_features: vec![],
            main_class: "Main".into(),
            include_flake: false,
        };
        assert!(render("console", &destination, context).is_err());
        assert!(!destination.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
