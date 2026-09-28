use clap::{Args, Parser, Subcommand, ValueEnum};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "jav")]
#[command(version, about = "A modern CLI for Java projects")]
pub struct Cli {
    /// Run from this directory.
    #[arg(short = 'C', long, global = true)]
    pub directory: Option<PathBuf>,
    /// Print planned operations without changing files or running commands.
    #[arg(long, global = true)]
    pub dry_run: bool,
    /// Show detailed output.
    #[arg(short, long, global = true)]
    pub verbose: bool,
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
    Build(BuildArgs),
    /// Test the current Java project.
    Test(TestArgs),
    /// Create jav.toml for an existing Java project.
    Init(InitArgs),
    /// Run the current Java project.
    Run(RunArgs),
    /// Clean build outputs for the current Java project.
    Clean,
}

#[derive(Debug, Args)]
pub struct UpgradeArgs {
    /// Show release metadata without downloading the executable.
    #[arg(long)]
    pub check: bool,

    /// Override the runtime identifier to upgrade.
    #[arg(long)]
    pub rid: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Configuration {
    Debug,
    Release,
}

impl Configuration {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Release => "release",
        }
    }
}

impl fmt::Display for Configuration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    /// Build configuration.
    #[arg(short, long, default_value_t = Configuration::Debug)]
    pub configuration: Configuration,

    /// Skip running tests during the build when supported.
    #[arg(long)]
    pub no_tests: bool,
}

#[derive(Debug, Args)]
pub struct InitArgs {}

#[derive(Debug, Args)]
pub struct TestArgs {
    /// Build configuration.
    #[arg(short, long, default_value_t = Configuration::Debug)]
    pub configuration: Configuration,
    /// Run tests matching this filter.
    #[arg(long)]
    pub filter: Option<String>,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Restart when sources or build configuration change.
    #[arg(long)]
    pub watch: bool,
    /// Build configuration used before running.
    #[arg(short, long, default_value_t = Configuration::Debug)]
    pub configuration: Configuration,

    /// Run without checking whether the project needs to be rebuilt.
    #[arg(long)]
    pub no_build: bool,

    /// Main class for simple Java projects.
    #[arg(long)]
    pub main_class: Option<String>,

    /// Arguments passed to the application.
    #[arg(last = true)]
    pub args: Vec<String>,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    /// Template to create, such as console or library. Use 'list' to show installed templates.
    pub template: Option<String>,
    /// Describe a template without creating a project.
    #[arg(long)]
    pub describe: bool,
    /// Project name.
    #[arg(short, long)]
    pub name: Option<String>,
    /// Java package name.
    #[arg(long)]
    pub package: Option<String>,
    /// Output directory. Defaults to the project name.
    #[arg(short, long)]
    pub output: Option<String>,
    /// Build tool to generate: maven or gradle. Defaults depend on the template.
    #[arg(long)]
    pub build_tool: Option<String>,
    /// Do not generate a Nix flake for the project.
    #[arg(long)]
    pub no_flake: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_global_flags_after_subcommands() {
        let cli = Cli::try_parse_from([
            "jav",
            "new",
            "console",
            "--verbose",
            "--dry-run",
            "-C",
            "project",
        ])
        .unwrap();
        assert!(cli.verbose);
        assert!(cli.dry_run);
        assert_eq!(cli.directory, Some(PathBuf::from("project")));
    }

    #[test]
    fn separates_watch_options_from_application_arguments() {
        let cli = Cli::try_parse_from(["jav", "run", "--watch", "--", "--verbose"]).unwrap();
        assert!(!cli.verbose);
        let Commands::Run(args) = cli.command else {
            panic!("expected run command")
        };
        assert!(args.watch);
        assert_eq!(args.args, ["--verbose"]);
    }
}
