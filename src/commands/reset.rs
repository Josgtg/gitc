use std::ffi::OsString;
use std::str::FromStr;

use anyhow::{Context, Result};

use crate::args::ResetCommand;
use crate::fs;
use crate::hashing::Hash;
use crate::index::Index;

pub fn reset(command: Option<&ResetCommand>) -> Result<String> {
    match command {
        Some(cmd) => match cmd {
            ResetCommand::Files { files } => reset_files(files),
            ResetCommand::Commit { hard, commit_hash } => {
                let hash = Hash::from_str(&commit_hash)
                    .context("hash provided was not a valid hexadecimal hash string")?;

                reset_to_commit(*hard, hash)
            }
        },
        None => {
            // the reset command, without arguments, resets to the previous commit
            let last_commit =
                fs::get_last_commit_hash().context("could not get las commit hash")?;
            match last_commit {
                Some(hash) => reset_to_commit(false, hash),
                None => {
                    // there is no previous commit, so we can just reset the index
                    fs::index::write_index_file(Index::default())
                        .context("could not write index file")?;

                    Ok("cleaned index file\n".into())
                }
            }
        }
    }
}

#[allow(unused)]
fn reset_files(files: &[OsString]) -> Result<String> {
    Ok("files have been reset".into())
}

#[allow(unused)]
fn reset_to_commit(hard: bool, commit_hash: Hash) -> Result<String> {
    Ok(format!("reset to commit {}", commit_hash))
}
