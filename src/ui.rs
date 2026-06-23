use anyhow::Result;
use dialoguer::{Input, Select};

pub fn select_template() -> Result<String> {
    let templates = ["console", "library", "springboot"];
    let selection = Select::new()
        .with_prompt("What do you want to create?")
        .items(&templates)
        .default(0)
        .interact()?;

    Ok(templates[selection].to_string())
}

pub fn input(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut input = Input::<String>::new().with_prompt(prompt.to_string());

    if let Some(default) = default {
        input = input.default(default.to_string());
    }

    Ok(input.interact_text()?)
}
