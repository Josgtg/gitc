#![allow(clippy::uninlined_format_args)]

mod args;
mod byteable;
mod commands;
mod constants;
mod fs;
mod gitignore;
mod hashing;
mod index;
mod object;
mod utils;

use clap::Parser;
pub use constants::*;

fn main() {
    let args = args::Args::parse();

    match commands::execute_command(&args.command) {
        Ok(message) => {
            if !message.is_empty() {
                println!("{}", message)
            }
        }
        Err(error) => eprintln!("{:?}", error),
    }
}
