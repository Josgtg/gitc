use std::ffi::OsString;

use clap::{Parser, Subcommand};

/// Contains the commands passed to the program
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

/// A list of subcommands the program can perform
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Creates a new repository
    Init {
        /// If set, creates a new folder with the given name and initializes the empty repository
        /// in that folder.
        #[arg(short, long)]
        folder_name: Option<OsString>,
    },
    /// Creates a new blob and updates index
    Add {
        /// Files to be added
        files: Vec<OsString>,
    },
    /// Creates a new commit object representing the current index
    Commit {
        /// Adds a commit message
        message: String,
    },
    /// Sets HEAD ref to specified commit
    Checkout {
        /// Reference or commit hash
        reference: String,
    },
    /// Shows the files present in the index file
    LsFiles {
        /// Shows more detailed information for every file
        #[arg(short, long)]
        debug: bool,
    }
}
