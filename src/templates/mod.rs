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
    pub build_tool: String,
    pub spring_boot_version: String,
    pub spring_features: Vec<String>,
}

pub fn manifest(template: &str) -> Option<TemplateManifest> {
    let raw = match template {
        "console" => include_str!("../../templates/console/template.toml"),
        "library" => include_str!("../../templates/library/template.toml"),
        "springboot" => include_str!("../../templates/springboot/template.toml"),
        _ => return None,
    };

    toml::from_str(raw).ok()
}

pub fn files(template: &str, context: &TemplateContext) -> Option<Vec<(&'static str, &'static str)>> {
    match template {
        "console" => Some(match context.build_tool.as_str() {
            "gradle" => vec![
                (
                    "settings.gradle.kts",
                    include_str!("../../templates/console/content/settings.gradle.kts"),
                ),
                (
                    "build.gradle.kts",
                    include_str!("../../templates/console/content/build.gradle.kts"),
                ),
                (
                    "src/main/java/{{package_path}}/Main.java",
                    include_str!("../../templates/console/content/src/main/java/__package_path__/Main.java"),
                ),
                (
                    "src/test/java/{{package_path}}/MainTest.java",
                    include_str!("../../templates/console/content/src/test/java/__package_path__/MainTest.java"),
                ),
            ],
            _ => vec![
                ("pom.xml", include_str!("../../templates/console/content/pom.xml")),
                (
                    "src/main/java/{{package_path}}/Main.java",
                    include_str!("../../templates/console/content/src/main/java/__package_path__/Main.java"),
                ),
                (
                    "src/test/java/{{package_path}}/MainTest.java",
                    include_str!("../../templates/console/content/src/test/java/__package_path__/MainTest.java"),
                ),
            ],
        }),
        "library" => Some(match context.build_tool.as_str() {
            "gradle" => vec![
                (
                    "settings.gradle.kts",
                    include_str!("../../templates/library/content/settings.gradle.kts"),
                ),
                (
                    "build.gradle.kts",
                    include_str!("../../templates/library/content/build.gradle.kts"),
                ),
                (
                    "src/main/java/{{package_path}}/Library.java",
                    include_str!("../../templates/library/content/src/main/java/__package_path__/Library.java"),
                ),
                (
                    "src/test/java/{{package_path}}/LibraryTest.java",
                    include_str!("../../templates/library/content/src/test/java/__package_path__/LibraryTest.java"),
                ),
            ],
            _ => vec![
                ("pom.xml", include_str!("../../templates/library/content/pom.xml")),
                (
                    "src/main/java/{{package_path}}/Library.java",
                    include_str!("../../templates/library/content/src/main/java/__package_path__/Library.java"),
                ),
                (
                    "src/test/java/{{package_path}}/LibraryTest.java",
                    include_str!("../../templates/library/content/src/test/java/__package_path__/LibraryTest.java"),
                ),
            ],
        }),
        "springboot" => {
            let mut files = match context.build_tool.as_str() {
                "gradle" => vec![
                    (
                        "settings.gradle.kts",
                        include_str!("../../templates/springboot/content/settings.gradle.kts"),
                    ),
                    (
                        "build.gradle.kts",
                        include_str!("../../templates/springboot/content/build.gradle.kts"),
                    ),
                ],
                _ => vec![(
                    "pom.xml",
                    include_str!("../../templates/springboot/content/pom.xml"),
                )],
            };

            files.extend([
                (
                    "src/main/java/{{package_path}}/Application.java",
                    include_str!("../../templates/springboot/content/src/main/java/__package_path__/Application.java"),
                ),
                (
                    "src/main/resources/application.properties",
                    include_str!("../../templates/springboot/content/src/main/resources/application.properties"),
                ),
                (
                    "src/test/java/{{package_path}}/ApplicationTests.java",
                    include_str!("../../templates/springboot/content/src/test/java/__package_path__/ApplicationTests.java"),
                ),
            ]);

            if context.spring_features.iter().any(|feature| feature == "web") {
                files.push((
                    "src/main/java/{{package_path}}/HelloController.java",
                    include_str!("../../templates/springboot/content/src/main/java/__package_path__/HelloController.java"),
                ));
            }

            Some(files)
        }
        _ => None,
    }
}
