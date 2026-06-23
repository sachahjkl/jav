use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use tera::{Context as TeraContext, Tera};

use crate::templates::{files, manifest, TemplateContext};

pub fn render(template: &str, destination: &Path, context: TemplateContext) -> Result<()> {
    let manifest = manifest(template).with_context(|| format!("unknown template '{template}'"))?;

    if manifest.id != template {
        bail!(
            "template manifest id '{}' does not match requested template '{template}'",
            manifest.id
        );
    }

    if manifest.name.trim().is_empty() || manifest.description.trim().is_empty() {
        bail!("template '{template}' has incomplete metadata");
    }

    if destination.exists() {
        bail!("destination already exists: {}", destination.display());
    }

    let files = files(template).with_context(|| format!("template '{template}' has no files"))?;
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
    tera_context
}
