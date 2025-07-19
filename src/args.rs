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
        #[arg(long = "name")]
        folder_name: Option<OsString>,
    },

    /// Creates a new blob and updates index
    Add {
        /// Files to be staged for the next commit
        files: Vec<OsString>,
    },
    /// Unstages files or resets to a previous commit, if no file is specified, all files are unstaged
    Reset {
        #[command(subcommand)]
        command: Option<ResetCommand>,
    },
    /// Shows the files present in the index file
    LsFiles {
        /// Shows more detailed information for every file
        #[arg(short, long)]
        debug: bool,
    },
    /// Shows working tree status
    Status,

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
    /// Shows the object file with the specified hash
    CatFile {
        /// Hash of the file to show
        hash: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ResetCommand {
    /// Unstage specific files
    Files {
        /// Files to unstage
        files: Vec<OsString>,
    },
    /// Reset to a previous commit
    Commit {
        /// Reset all files and working tree
        #[arg(long)]
        hard: bool,
        /// Commit hash to reset to
        commit_hash: String,
    },
}
