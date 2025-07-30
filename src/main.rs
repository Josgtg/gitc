#![allow(clippy::uninlined_format_args)]

mod args;
mod byteable;
mod commands;
mod constants;
mod error;
mod fs;
mod gitignore;
mod hashing;
mod index;
mod object;
mod utils;

pub use constants::*;

fn main() {
    let _ = dotenvy::dotenv();
    env_logger::init();

    use clap::Parser;
    let args = args::Args::parse_from(wild::args());

    match commands::execute_command(&args.command) {
        Ok(message) => print!("{}", message),
        Err(error) => eprintln!("{:?}", error),
    }
}
