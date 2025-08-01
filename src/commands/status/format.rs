use colored::Colorize;

use crate::fs;

use super::status::{FileWithStatus, StageStatus, Status};

/// Given a list of file statuses, returns a formatted string depicting this status for every file.
pub fn format_status(status: Vec<FileWithStatus>, no_commits: bool) -> String {
    let filtered_status: Vec<FileWithStatus> = status
        .into_iter()
        .filter(|fws| fws.status != Status::Unchanged)
        .collect();

    let mut header = format!(
        "On branch {}\n",
        fs::get_current_branch_name().unwrap_or("!".into())
    );

    if no_commits {
        header.push_str("\nNo commits yet\n");
    }

    if filtered_status.is_empty() {
        return format!("{}\nThere is nothing to commit, all clean!\n", header);
    }

    let mut commit = String::new();
    let mut notcommit = String::new();
    let mut untracked = String::new();
    let mut status_str: String;
    let mut path_str: String;
    for s in filtered_status {
        path_str = s.path.to_string_lossy().to_string();

        if s.stage_status == StageStatus::Untracked {
            untracked.push_str(&format!("\t{}\n", path_str));
            continue;
        }

        status_str = match &s.status {
            Status::New => format!("\tnew file:\t{}\n", path_str),
            Status::Moved { previous } => format!(
                "\tmoved:\t{} -> {}\n",
                previous.to_string_lossy().to_string(),
                path_str
            ),
            Status::Deleted => format!("\tdeleted:\t{}\n", path_str),
            Status::Modified => format!("\tmodified:\t{}\n", path_str),
            Status::Unchanged => continue,
        };

        match s.stage_status {
            StageStatus::Commit => commit.push_str(&status_str),
            StageStatus::NotCommit => notcommit.push_str(&status_str),
            StageStatus::Untracked => untracked.push_str(&status_str),
        };
    }

    if !commit.is_empty() {
        header = format!("{}\nChanges staged for commit:\n{}", header, commit.green());
    }
    if !notcommit.is_empty() {
        header = format!("{}\nNot staged for commit:\n{}", header, notcommit.red());
    }
    if !untracked.is_empty() {
        header = format!("{}\nUntracked files:\n{}", header, untracked.red());
    }

    header
}
