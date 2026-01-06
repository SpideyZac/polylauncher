use clap::Parser;

#[derive(Parser)]
#[command(name = "PolyLauncher CLI", author = "SpideyZac", version = env!("CARGO_PKG_VERSION"), about = "A CLI tool for PolyLauncher.")]
struct Cli {}

fn main() {
    match Cli::try_parse() {
        Ok(_cli) => {
            println!("CLI mode - implement your logic here");
            // TODO: handle cli arguments
        }
        Err(e) => {
            e.print().ok();
        }
    }
}
