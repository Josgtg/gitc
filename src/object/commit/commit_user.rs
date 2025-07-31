use std::fmt::Display;
use std::str::FromStr;
use std::time::SystemTime;

use anyhow::{Result, bail};
use time::UtcOffset;

use super::*;

#[derive(Debug)]
pub struct CommitUser {
    pub kind: CommitUserKind,
    /// Generally name and email
    pub identifier: String,
    pub timestamp: SystemTime,
    pub timezone: UtcOffset,
}

impl CommitUser {
    /// Just for debugging for now
    pub fn default(kind: CommitUserKind) -> Self {
        CommitUser {
            kind,
            identifier: "Josué Torres <josue.ger.torres.gar@gmail.com>".to_owned(),
            timestamp: SystemTime::now(),
            timezone: UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
        }
    }
}

#[derive(Debug)]
pub enum CommitUserKind {
    Author,
    Committer,
}

impl FromStr for CommitUserKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            AUTHOR_STR => Ok(CommitUserKind::Author),
            COMMITTER_STR => Ok(CommitUserKind::Committer),
            _ => bail!("invalid commit user kind: {}", s),
        }
    }
}

impl Display for CommitUserKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CommitUserKind::Author => AUTHOR_STR,
            CommitUserKind::Committer => COMMITTER_STR,
        })
    }
}
