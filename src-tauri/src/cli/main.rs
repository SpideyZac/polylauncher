use std::path::PathBuf;

use clap::Parser;
use PolyLauncher::{apply_patch, create_patch};

#[derive(Parser)]
#[command(name = "PolyLauncher CLI", author = "SpideyZac", version = env!("CARGO_PKG_VERSION"), about = "A CLI tool for PolyLauncher.")]
struct Cli {}

fn main() {
    match Cli::try_parse() {
        Ok(_cli) => {
            println!("CLI mode - implement your logic here");
            // TODO: handle cli arguments

            let patch_location = PathBuf::from("patches/example.patch");
            let path1 = PathBuf::from("test1/");
            let path2 = PathBuf::from("test2/");
            let path3 = PathBuf::from("test3/");

            create_patch(&patch_location, &path1, &path2).unwrap();
            apply_patch(&patch_location, &path3).unwrap();
        }
        Err(e) => {
            e.print().ok();
        }
    }
}
