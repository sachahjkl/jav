pub mod render;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateManifest {
    pub id: String,
    pub name: String,
    pub description: String,
    pub short_name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub language: String,
    pub tags: Vec<String>,
    pub default_build_tool: String,
    pub renderer: String,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub default_features: Vec<String>,
    #[serde(default)]
    pub supported_features: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct SpringFeature {
    pub id: &'static str,
    pub description: &'static str,
    pub maven_dependency: Option<&'static str>,
    pub gradle_dependency: Option<&'static str>,
    pub runtime_maven_dependency: Option<&'static str>,
    pub runtime_gradle_dependency: Option<&'static str>,
    pub files: &'static [(&'static str, &'static str)],
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
    pub main_class: String,
    pub include_flake: bool,
}

pub const TEMPLATE_IDS: &[&str] = &[
    "console",
    "cli",
    "worker",
    "library",
    "junit",
    "springboot",
    "springweb",
    "springdata",
    "springsecurity",
    "springbatch",
];

pub const SPRING_FEATURES: &[SpringFeature] = &[
    SpringFeature {
        id: "web",
        description: "Spring MVC REST endpoints with embedded Tomcat.",
        maven_dependency: Some("spring-boot-starter-web"),
        gradle_dependency: Some("org.springframework.boot:spring-boot-starter-web"),
        runtime_maven_dependency: None,
        runtime_gradle_dependency: None,
        files: &[(
            "src/main/java/{{package_path}}/HelloController.java",
            include_str!("../../templates/springboot/content/src/main/java/__package_path__/HelloController.java"),
        )],
    },
    SpringFeature {
        id: "actuator",
        description: "Production health, info, metrics, and management endpoints.",
        maven_dependency: Some("spring-boot-starter-actuator"),
        gradle_dependency: Some("org.springframework.boot:spring-boot-starter-actuator"),
        runtime_maven_dependency: None,
        runtime_gradle_dependency: None,
        files: &[],
    },
    SpringFeature {
        id: "data-jpa",
        description: "Spring Data JPA repositories and entity support.",
        maven_dependency: Some("spring-boot-starter-data-jpa"),
        gradle_dependency: Some("org.springframework.boot:spring-boot-starter-data-jpa"),
        runtime_maven_dependency: None,
        runtime_gradle_dependency: None,
        files: &[
            (
                "src/main/java/{{package_path}}/Customer.java",
                include_str!("../../templates/springboot/content/src/main/java/__package_path__/Customer.java"),
            ),
            (
                "src/main/java/{{package_path}}/CustomerRepository.java",
                include_str!("../../templates/springboot/content/src/main/java/__package_path__/CustomerRepository.java"),
            ),
        ],
    },
    SpringFeature {
        id: "security",
        description: "Spring Security with HTTP Basic defaults.",
        maven_dependency: Some("spring-boot-starter-security"),
        gradle_dependency: Some("org.springframework.boot:spring-boot-starter-security"),
        runtime_maven_dependency: None,
        runtime_gradle_dependency: None,
        files: &[(
            "src/main/java/{{package_path}}/SecurityConfig.java",
            include_str!("../../templates/springboot/content/src/main/java/__package_path__/SecurityConfig.java"),
        )],
    },
    SpringFeature {
        id: "lombok",
        description: "Lombok annotations and annotation processing setup.",
        maven_dependency: None,
        gradle_dependency: None,
        runtime_maven_dependency: None,
        runtime_gradle_dependency: None,
        files: &[],
    },
    SpringFeature {
        id: "postgresql",
        description: "PostgreSQL JDBC driver as a runtime dependency.",
        maven_dependency: None,
        gradle_dependency: None,
        runtime_maven_dependency: Some("postgresql"),
        runtime_gradle_dependency: Some("org.postgresql:postgresql"),
        files: &[],
    },
    SpringFeature {
        id: "batch",
        description: "Spring Batch job and step infrastructure.",
        maven_dependency: Some("spring-boot-starter-batch"),
        gradle_dependency: Some("org.springframework.boot:spring-boot-starter-batch"),
        runtime_maven_dependency: None,
        runtime_gradle_dependency: None,
        files: &[(
            "src/main/java/{{package_path}}/BatchJobConfig.java",
            include_str!("../../templates/springboot/content/src/main/java/__package_path__/BatchJobConfig.java"),
        )],
    },
];

pub fn manifests() -> Vec<TemplateManifest> {
    TEMPLATE_IDS
        .iter()
        .filter_map(|template| manifest_by_id(template))
        .collect()
}

pub fn manifest(template: &str) -> Option<TemplateManifest> {
    manifests().into_iter().find(|manifest| {
        manifest.id == template || manifest.aliases.iter().any(|alias| alias == template)
    })
}

fn manifest_by_id(template: &str) -> Option<TemplateManifest> {
    let raw = match template {
        "console" => include_str!("../../templates/console/template.toml"),
        "cli" => include_str!("../../templates/cli/template.toml"),
        "worker" => include_str!("../../templates/worker/template.toml"),
        "library" => include_str!("../../templates/library/template.toml"),
        "junit" => include_str!("../../templates/junit/template.toml"),
        "springboot" => include_str!("../../templates/springboot/template.toml"),
        "springweb" => include_str!("../../templates/springweb/template.toml"),
        "springdata" => include_str!("../../templates/springdata/template.toml"),
        "springsecurity" => include_str!("../../templates/springsecurity/template.toml"),
        "springbatch" => include_str!("../../templates/springbatch/template.toml"),
        _ => return None,
    };

    toml::from_str(raw).ok()
}

pub fn feature(id: &str) -> Option<&'static SpringFeature> {
    SPRING_FEATURES.iter().find(|feature| feature.id == id)
}

pub fn files(
    manifest: &TemplateManifest,
    context: &TemplateContext,
) -> Option<Vec<(&'static str, &'static str)>> {
    let mut files = match manifest.renderer.as_str() {
        "plain-app" => Some(match context.build_tool.as_str() {
            "gradle" => vec![
                (".gitignore", include_str!("../../templates/common/gitignore")),
                ("README.md", include_str!("../../templates/common/README.md")),
                ("jav.toml", include_str!("../../templates/common/plain-jav.toml")),
                (
                    "settings.gradle.kts",
                    include_str!("../../templates/console/content/settings.gradle.kts"),
                ),
                (
                    "build.gradle.kts",
                    include_str!("../../templates/console/content/build.gradle.kts"),
                ),
                (
                    "src/main/java/{{package_path}}/{{main_class}}.java",
                    plain_main_file(&manifest.id),
                ),
                (
                    "src/test/java/{{package_path}}/{{main_class}}Test.java",
                    plain_test_file(&manifest.id),
                ),
            ],
            _ => vec![
                (".gitignore", include_str!("../../templates/common/gitignore")),
                ("README.md", include_str!("../../templates/common/README.md")),
                ("jav.toml", include_str!("../../templates/common/plain-jav.toml")),
                ("pom.xml", include_str!("../../templates/console/content/pom.xml")),
                (
                    "src/main/java/{{package_path}}/{{main_class}}.java",
                    plain_main_file(&manifest.id),
                ),
                (
                    "src/test/java/{{package_path}}/{{main_class}}Test.java",
                    plain_test_file(&manifest.id),
                ),
            ],
        }),
        "library" => Some(match context.build_tool.as_str() {
            "gradle" => vec![
                (".gitignore", include_str!("../../templates/common/gitignore")),
                ("README.md", include_str!("../../templates/common/README.md")),
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
                (".gitignore", include_str!("../../templates/common/gitignore")),
                ("README.md", include_str!("../../templates/common/README.md")),
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
                    (".gitignore", include_str!("../../templates/common/gitignore")),
                    ("README.md", include_str!("../../templates/common/README.md")),
                    ("jav.toml", include_str!("../../templates/common/spring-jav.toml")),
                    (
                        "settings.gradle.kts",
                        include_str!("../../templates/springboot/content/settings.gradle.kts"),
                    ),
                    (
                        "build.gradle.kts",
                        include_str!("../../templates/springboot/content/build.gradle.kts"),
                    ),
                ],
                _ => vec![
                    (".gitignore", include_str!("../../templates/common/gitignore")),
                    ("README.md", include_str!("../../templates/common/README.md")),
                    ("jav.toml", include_str!("../../templates/common/spring-jav.toml")),
                    (
                        "pom.xml",
                        include_str!("../../templates/springboot/content/pom.xml"),
                    ),
                ],
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

            for selected in &context.spring_features {
                if let Some(feature) = feature(selected) {
                    files.extend(feature.files);
                }
            }

            Some(files)
        }
        _ => None,
    }?;

    if context.include_flake {
        files.push((
            "flake.nix",
            include_str!("../../templates/common/flake.nix"),
        ));
    }

    Some(files)
}

fn plain_main_file(template: &str) -> &'static str {
    match template {
        "cli" => include_str!(
            "../../templates/cli/content/src/main/java/__package_path__/CommandApp.java"
        ),
        "worker" => include_str!(
            "../../templates/worker/content/src/main/java/__package_path__/Worker.java"
        ),
        "junit" => include_str!(
            "../../templates/junit/content/src/main/java/__package_path__/Calculator.java"
        ),
        _ => {
            include_str!("../../templates/console/content/src/main/java/__package_path__/Main.java")
        }
    }
}

fn plain_test_file(template: &str) -> &'static str {
    match template {
        "cli" => include_str!(
            "../../templates/cli/content/src/test/java/__package_path__/CommandAppTest.java"
        ),
        "worker" => include_str!(
            "../../templates/worker/content/src/test/java/__package_path__/WorkerTest.java"
        ),
        "junit" => include_str!(
            "../../templates/junit/content/src/test/java/__package_path__/CalculatorTest.java"
        ),
        _ => include_str!(
            "../../templates/console/content/src/test/java/__package_path__/MainTest.java"
        ),
    }
}
