mod add;
mod checkout;
mod commit;
mod init;

use add::add;
use checkout::checkout;
use commit::commit;
use init::init;

use crate::args::Command;
use crate::fs;
use crate::Result;

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
    if !fs::repository_exists() {
        if let Command::Init { folder_name } = command {
            // Only command that can be executed without a repository already existing
            return init(folder_name);
        }
        return Ok("folder is not a git repository".into());
    }
    match command {
        Command::Init { .. } => Ok("".into()), // should never get here
        Command::Add { files } => add(files),
        Command::Commit { message } => commit(message),
        Command::Checkout { reference } => checkout(reference),
    }
}
