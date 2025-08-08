//! lxwengctl entry
//! The cli program to communicate with lxwengd

mod cli;

use clap::Parser;

fn main() {
    let _cli = cli::Cli::parse();
}
