use std::process::exit;

use clap::{Parser, Subcommand};
use colored::Colorize;

mod commands;
mod config;
mod downloader;
mod error;

use commands::init::handle_init;

#[derive(Parser)]
#[command(
    name = "PolyLauncher CLI",
    author = "SpideyZac",
    version = env!("CARGO_PKG_VERSION"),
    about = "A CLI tool for PolyLauncher."
)]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new PolyTrack installation
    Init {
        #[arg(
            name = "polytrack-version",
            default_value = "latest",
            help = "The PolyTrack version to patch."
        )]
        polytrack_version: String,
    },
}

fn main() {
    // Parse CLI arguments
    let result = match Cli::try_parse() {
        Ok(cli) => {
            if let Some(command) = cli.subcommand {
                match command {
                    Commands::Init { polytrack_version } => handle_init(polytrack_version),
                }
            } else {
                Ok(())
            }
        }
        Err(e) => {
            e.print().ok();
            exit(1);
        }
    };

    // Handle errors gracefully
    if let Err(e) = result {
        eprintln!("{}", format!("Error: {}", e).red());
        exit(1);
    }
}
