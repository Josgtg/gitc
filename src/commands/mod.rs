mod add;
mod checkout;
mod commit;
mod init;
mod ls_files;

use std::fs;

use add::add;
use checkout::checkout;
use commit::commit;
use init::init;
use ls_files::ls_files;

use crate::args::Command;
use crate::{Constants, Result};

/// Calls the corresponding function to perform every command variant.
///
/// # Return
///
/// The success message
///
/// # Errors
///
/// This function will fail if any of the executed commands return an error.
pub fn execute_command(command: &Command) -> Result<String> {
    if !fs::exists(Constants::repository_path())? {
        if let Command::Init { folder_name } = command {
            // Only command that can be executed without a repository already existing
            return init(folder_name.as_deref());
        }
        return Ok("Folder is not a git repository".into());
    }
    match command {
        Command::Init { folder_name } => init(folder_name.as_deref()), // always returns an "already a git repository"
        Command::Add { files } => add(files),
        Command::Commit { message } => commit(message.as_deref()),
        Command::Checkout { reference } => checkout(reference),
        Command::LsFiles { stage, debug } => ls_files(*stage, *debug),
    }
}
