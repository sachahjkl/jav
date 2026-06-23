mod cli;
mod commands;
mod output;
mod process;
mod project;
mod templates;
mod ui;
mod upgrade;
mod version;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::process::RealRunner;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let runner = RealRunner;

    match cli.command {
        Commands::Doctor => commands::doctor::run(&runner),
        Commands::Upgrade(args) => commands::upgrade::run(args),
        Commands::New(args) => commands::new::run(args),
        Commands::Build(args) => commands::build::run(args, &runner),
        Commands::Test => commands::test::run(&runner),
        Commands::Run(args) => commands::run::run(args, &runner),
        Commands::Clean => commands::clean::run(&runner),
    }
}
