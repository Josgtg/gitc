#![allow(dead_code)]

mod args;
mod byteable;
mod commands;
mod constants;
mod error;
mod fs;
mod hashing;
mod index;
mod object;

use clap::Parser;
pub use constants::*;
pub use error::{Error, Result};

fn main() {
    let args = args::Args::parse();

    match commands::execute_command(&args.command) {
        Ok(message) => println!("{message}"),
        Err(error) => eprintln!("{error}"),
    }
}
