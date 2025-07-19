mod add;
mod checkout;
mod commit;
mod init;
mod ls_files;
mod reset;
mod status;
mod cat_file;

use std::fs;

use add::add;
use cat_file::cat_file;
use checkout::checkout;
use commit::commit;
use init::init;
use ls_files::ls_files;
use reset::reset;
use status::status;

use anyhow::Result;

use crate::Constants;
use crate::args::Command;

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
        Command::Reset { command } => reset(command.as_ref()),
        Command::Status => status(),
        Command::Commit { message } => commit(message.as_ref()),
        Command::Checkout { reference } => checkout(reference),
        Command::LsFiles { debug } => ls_files(*debug),
        Command::CatFile { hash } => cat_file(hash),
    }
}
