mod cli;
mod commands;
mod config;
mod output;
mod process;
mod project;
mod templates;
mod ui;
mod upgrade;
mod version;

use anyhow::{Context, Result};
use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::process::{ProcessExit, RealRunner};

fn main() -> std::process::ExitCode {
    match execute() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            if let Some(status) = error.downcast_ref::<ProcessExit>() {
                std::process::exit(status.code);
            }
            std::process::ExitCode::FAILURE
        }
    }
}

fn execute() -> Result<()> {
    let cli = Cli::parse();
    if let Some(directory) = cli.directory {
        std::env::set_current_dir(&directory)
            .with_context(|| format!("failed to enter {}", directory.display()))?;
    }
    match &cli.command {
        Commands::Build(_)
        | Commands::Test(_)
        | Commands::Run(_)
        | Commands::Clean
        | Commands::Init(_) => {
            let root = project::detect::find_root(std::env::current_dir()?)?;
            std::env::set_current_dir(root)?;
        }
        Commands::Doctor => {
            if let Ok(root) = project::detect::find_root(std::env::current_dir()?) {
                std::env::set_current_dir(root)?;
            }
        }
        _ => {}
    }
    let runner = RealRunner::new(cli.dry_run, cli.verbose);

    match cli.command {
        Commands::Doctor => commands::doctor::run(&runner),
        Commands::Upgrade(args) if cli.dry_run => {
            if args.check {
                println!("query release metadata (dry run)");
            } else {
                println!(
                    "download, verify, and replace the executable with a newer release (dry run)"
                );
            }
            Ok(())
        }
        Commands::Upgrade(args) => commands::upgrade::run(args),
        Commands::New(args) => commands::new::run(args, cli.verbose, cli.dry_run),
        Commands::Init(args) => commands::init::run(args, cli.dry_run),
        Commands::Build(args) => commands::build::run(args, &runner),
        Commands::Test(args) => commands::test::run(args, &runner),
        Commands::Run(args) if args.watch && !cli.dry_run => {
            commands::watch::run(args, cli.verbose)
        }
        Commands::Run(args) => commands::run::run(args, &runner),
        Commands::Clean => commands::clean::run(&runner),
    }
}
