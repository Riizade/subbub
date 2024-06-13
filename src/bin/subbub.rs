// this file contains the CLI binary for subbub

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        _ => todo!(),
    }
}
