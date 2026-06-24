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

    if destination.exists() {
        bail!("destination already exists: {}", destination.display());
    }

    let files = files(&manifest, &context)
        .with_context(|| format!("template '{template}' has no files"))?;
    let tera_context = tera_context(&context);

    for (relative_path, content) in files {
        let rendered_path = Tera::one_off(relative_path, &tera_context, false)?;
        let target = destination.join(rendered_path);

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        let rendered = Tera::one_off(content, &tera_context, false)?;
        fs::write(target, rendered)?;
    }

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
