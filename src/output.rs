use anstream::println;
use console::style;

pub fn status(label: &str, value: impl AsRef<str>) {
    println!("{} {}", style(label).green().bold(), value.as_ref());
}

pub fn warning(message: impl AsRef<str>) {
    println!("{} {}", style("warning:").yellow().bold(), message.as_ref());
}
