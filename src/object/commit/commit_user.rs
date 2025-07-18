use std::{fmt::Display, str::FromStr, time::SystemTime};

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

impl Default for CommitUser {
    /// Just for debugging from now
    fn default() -> Self {
        CommitUser {
            kind: CommitUserKind::Author,
            identifier: "Josué Torres <josue.ger.torres.gar@gmail.com>".to_owned(),
            timestamp: SystemTime::now(),
            timezone: UtcOffset::UTC,
        }
    }
}

#[derive(Debug)]
pub enum CommitUserKind {
    Author,
    Commiter,
}

impl FromStr for CommitUserKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            AUTHOR_STR => Ok(CommitUserKind::Author),
            COMMITTER_STR => Ok(CommitUserKind::Commiter),
            _ => bail!("invalid commit user kind: {}", s),
        }
    }
}

impl Display for CommitUserKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CommitUserKind::Author => AUTHOR_STR,
            CommitUserKind::Commiter => COMMITTER_STR,
        })
    }
}
