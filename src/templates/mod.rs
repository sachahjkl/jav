pub mod render;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateManifest {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct TemplateContext {
    pub project_name: String,
    pub package_name: String,
    pub package_path: String,
    pub java_version: String,
}

pub fn manifest(template: &str) -> Option<TemplateManifest> {
    let raw = match template {
        "console" => include_str!("../../templates/console/template.toml"),
        "library" => include_str!("../../templates/library/template.toml"),
        _ => return None,
    };

    toml::from_str(raw).ok()
}

pub fn files(template: &str) -> Option<&'static [(&'static str, &'static str)]> {
    match template {
        "console" => Some(&[
            ("pom.xml", include_str!("../../templates/console/content/pom.xml")),
            (
                "src/main/java/{{package_path}}/Main.java",
                include_str!("../../templates/console/content/src/main/java/__package_path__/Main.java"),
            ),
            (
                "src/test/java/{{package_path}}/MainTest.java",
                include_str!("../../templates/console/content/src/test/java/__package_path__/MainTest.java"),
            ),
        ]),
        "library" => Some(&[
            ("pom.xml", include_str!("../../templates/library/content/pom.xml")),
            (
                "src/main/java/{{package_path}}/Library.java",
                include_str!("../../templates/library/content/src/main/java/__package_path__/Library.java"),
            ),
            (
                "src/test/java/{{package_path}}/LibraryTest.java",
                include_str!("../../templates/library/content/src/test/java/__package_path__/LibraryTest.java"),
            ),
        ]),
        _ => None,
    }
}
