use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::project::sources::java_sources;

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JavConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run: Option<RunConfig>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maven_task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradle_task: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

pub fn read() -> Result<JavConfig> {
    match fs::read_to_string("jav.toml") {
        Ok(content) => toml::from_str(&content).context("invalid jav.toml"),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(JavConfig::default()),
        Err(error) => Err(error).context("cannot read jav.toml"),
    }
}

pub fn infer_main_class() -> Result<Option<String>> {
    let mut classes = Vec::new();
    for source in java_sources("src/main/java")? {
        classes.extend(main_classes(&fs::read_to_string(&source)?, &source)?);
    }
    classes.sort();
    classes.dedup();
    if classes.len() > 1 {
        bail!(
            "multiple main classes found: {}; set run.main_class in jav.toml",
            classes.join(", ")
        );
    }
    Ok(classes.pop())
}

// Ignore comments and literals before inspecting Java declarations.
fn tokens(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut result = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let current = chars[index];
        let next = chars.get(index + 1).copied();
        if current == '/' && next == Some('/') {
            index += 2;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
        } else if current == '/' && next == Some('*') {
            index += 2;
            while index + 1 < chars.len() && !(chars[index] == '*' && chars[index + 1] == '/') {
                index += 1;
            }
            index = (index + 2).min(chars.len());
        } else if current == '"' && chars.get(index..index + 3) == Some(&['"', '"', '"']) {
            index += 3;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                } else if chars.get(index..index + 3) == Some(&['"', '"', '"']) {
                    index += 3;
                    break;
                } else {
                    index += 1;
                }
            }
        } else if matches!(current, '"' | '\'') {
            index += 1;
            while index < chars.len() {
                let character = chars[index];
                index += 1;
                if character == '\\' {
                    index = (index + 1).min(chars.len());
                } else if character == current {
                    break;
                }
            }
        } else if current.is_alphanumeric() || matches!(current, '_' | '$') {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_alphanumeric() || matches!(chars[index], '_' | '$'))
            {
                index += 1;
            }
            result.push(chars[start..index].iter().collect());
        } else {
            if !current.is_whitespace() {
                result.push(current.to_string());
            }
            index += 1;
        }
    }
    result
}

fn main_classes(source: &str, path: &Path) -> Result<Vec<String>> {
    let words = tokens(source);
    let package = words
        .iter()
        .position(|word| word == "package")
        .map(|start| {
            words[start + 1..]
                .iter()
                .take_while(|word| word.as_str() != ";")
                .cloned()
                .collect::<String>()
        });
    let mut classes = Vec::new();
    let mut depth = 0usize;
    let mut class = None;
    let mut declaration = 0;
    for (index, word) in words.iter().enumerate() {
        match word.as_str() {
            "class" | "record" | "enum" | "interface" if depth == 0 => {
                class = words.get(index + 1).cloned()
            }
            "{" => {
                depth += 1;
                declaration = index + 1;
            }
            "}" => {
                depth = depth.saturating_sub(1);
                declaration = index + 1;
            }
            ";" => declaration = index + 1,
            "main"
                if index > 0
                    && words[index - 1] == "void"
                    && words.get(index + 1).is_some_and(|word| word == "(") =>
            {
                let modifiers = &words[declaration..index];
                if !modifiers.iter().any(|word| word == "public")
                    || !modifiers.iter().any(|word| word == "static")
                {
                    continue;
                }
                let parameters: Vec<&str> = words[index + 2..]
                    .iter()
                    .take_while(|word| word.as_str() != ")")
                    .map(String::as_str)
                    .collect();
                let parameters = parameters.strip_prefix(&["final"]).unwrap_or(&parameters);
                let parameters = parameters
                    .strip_prefix(&["java", ".", "lang", "."])
                    .unwrap_or(parameters);
                let valid = matches!(
                    parameters,
                    ["String", "[", "]", _]
                        | ["String", _, "[", "]"]
                        | ["String", ".", ".", ".", _]
                );
                if !valid {
                    continue;
                }
                if depth != 1 {
                    bail!(
                        "cannot infer nested main class in {}; set run.main_class in jav.toml",
                        path.display()
                    );
                }
                if let Some(class) = &class {
                    classes.push(match &package {
                        Some(package) => format!("{package}.{class}"),
                        None => class.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(classes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_configuration_fields() {
        assert!(toml::from_str::<JavConfig>("unknown = true").is_err());
        assert!(toml::from_str::<JavConfig>("[run]\nmain_clas = 'Main'").is_err());
    }

    #[test]
    fn reads_commented_multiline_declarations() {
        let source = "/* package wrong; */ package example /* note */ . app; class Main { String example = \"public static void main(String[] args)\"; public\nstatic void main (String ... args) {} }";
        assert_eq!(
            main_classes(source, Path::new("Main.java")).unwrap(),
            ["example.app.Main"]
        );
    }

    #[test]
    fn finds_each_top_level_main() {
        let source = "class One { public static void main(String[] args) {} } class Two { public static void main(String args[]) {} }";
        assert_eq!(
            main_classes(source, Path::new("One.java")).unwrap(),
            ["One", "Two"]
        );
    }

    #[test]
    fn ignores_comments_literals_and_wrong_signatures() {
        let source = "class Main { /* public static void main(String[] args) {} */ String text = \"\"\"public static void main(String[] args) {}\"\"\"; public static void main(int value) {} }";
        assert!(main_classes(source, Path::new("Main.java"))
            .unwrap()
            .is_empty());
    }
}
