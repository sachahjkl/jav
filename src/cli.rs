use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "jav")]
#[command(version, about = "A modern CLI for Java projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Inspect the Java tooling available on this machine.
    Doctor,
    /// Upgrade jav from the latest GitHub release.
    Upgrade(UpgradeArgs),
    /// Create a new Java project from a template.
    New(NewArgs),
    /// Build the current Java project.
    Build,
    /// Test the current Java project.
    Test,
    /// Run the current Java project.
    Run,
    /// Clean build outputs for the current Java project.
    Clean,
}

#[derive(Debug, Args)]
pub struct UpgradeArgs {
    /// Print release metadata without downloading anything.
    #[arg(long)]
    pub check: bool,

    /// Override the runtime identifier to upgrade.
    #[arg(long)]
    pub rid: Option<String>,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Template to create, such as console or library.
    pub template: Option<String>,
    /// Project name.
    #[arg(short, long)]
    pub name: Option<String>,
    /// Java package name.
    #[arg(long)]
    pub package: Option<String>,
    /// Output directory. Defaults to the project name.
    #[arg(short, long)]
    pub output: Option<String>,
    /// Build tool to generate: maven or gradle.
    #[arg(long, default_value = "maven")]
    pub build_tool: String,
    /// Java language version.
    #[arg(long, default_value = "21")]
    pub java_version: String,
    /// Spring Boot dependency features for the springboot template.
    #[arg(long = "feature")]
    pub features: Vec<String>,
    /// Spring Boot version for the springboot template.
    #[arg(long, default_value = "3.5.0")]
    pub spring_boot_version: String,
}
