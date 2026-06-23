use anyhow::Result;
use dialoguer::Input;

pub fn input(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut input = Input::<String>::new().with_prompt(prompt.to_string());

    if let Some(default) = default {
        input = input.default(default.to_string());
    }

    Ok(input.interact_text()?)
}
