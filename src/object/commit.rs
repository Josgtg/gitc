use std::fmt::Display;
use std::rc::Rc;
use std::str::{FromStr, Split};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::UtcOffset;

use crate::hashing::Hash;

use super::Object;

// TODO: Add support for multiple parents and try to extract the logic to reading from the cursor
// to a function to reduce the size.

pub const TREE_STR: &str = "tree";
pub const PARENT_STR: &str = "parent";
pub const AUTHOR_STR: &str = "author";
pub const COMMITTER_STR: &str = "committer";
const TIMEZONE_FORMAT: &[BorrowedFormatItem] = format_description!("[offset_hour][offset_minute]");

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

#[derive(Debug)]
pub struct CommitUser {
    kind: CommitUserKind,
    /// Generally name and email
    identifier: String,
    timestamp: SystemTime,
    timezone: UtcOffset,
}

/// Returns the commit as the bytes of a string with the following format:
///
/// tree {`tree_hash`}
/// parent {`parent_hash`}
/// author {`author.kind`} {`author.identifier`} {`author.timestamp`} {`author.timezone`}
/// committer {`commiter.kind`} {`commiter.identifier`} {`commiter.timestamp`} {`commiter.timezone`}
///
///
/// {`message`}
pub fn as_bytes(
    tree_hash: &Hash,
    parent: Option<&Hash>,
    author: &CommitUser,
    commiter: &CommitUser,
    message: &str,
) -> Result<Rc<[u8]>> {
    let mut file = format!("{} {}\n", TREE_STR, tree_hash);
    if let Some(parent_hash) = parent {
        file.push_str(&format!("{} {}", PARENT_STR, parent_hash));
    }
    file.push_str(&format_commituser(author)?);
    file.push_str(&format_commituser(commiter)?);
    file.push_str(&format!("\n\n{}", message));

    Ok(file.as_bytes().into())
}

fn format_commituser(user: &CommitUser) -> Result<String> {
    Ok(format!(
        "{} {} {} {:?}\n",
        user.kind,
        user.identifier,
        user.timestamp
            .duration_since(UNIX_EPOCH)
            .context("timestamp was invalid")?
            .as_secs(),
        user.timezone
    ))
}

/// Parses a sequence of bytes expecting the format of a commit file, returning a Commit object.
///
/// # Errors
///
/// This function will fail if the bytes do not conform to the expected format, or if any of the
/// parsing operations fail.
pub fn from_bytes(bytes: &[u8]) -> Result<Object> {
    let commit_str =
        std::str::from_utf8(bytes).context("could not form a string from the given bytes")?;
    let mut lines = commit_str.lines();

    let mut splitted: Split<_>;

    // tree {tree_hash}
    splitted = lines
        .next()
        .context("commit object did not have any lines")?.split(' ');
    if splitted.next() != Some(TREE_STR) {
        bail!("expected commit object to start with {}", TREE_STR)
    }
    let tree_hash_str = splitted.next().context(format!("expected hash after {}", TREE_STR))?;
    let tree_hash = Hash::from_str(tree_hash_str).context(format!("could not create a hash from the {} hash string", TREE_STR))?;

    // parent {parent_hash} (if exists)
    splitted = lines.next().context("commit object only had one line")?.split(' ');
    let next = splitted.next().context(format!("expected either {} or {}, got nothing", PARENT_STR, AUTHOR_STR))?;
    let mut parent = None;
    if next == PARENT_STR {
        let parent_hash_str = splitted.next().context(format!("expected hash after {}", PARENT_STR))?;
        parent = Some(Hash::from_str(parent_hash_str).context(format!("could not create a hash from the {} hash string", PARENT_STR))?);
        // Updating, the next code expects `splitted` to be at the second word of the author line
        splitted = lines.next().context("commit file ended abruptly")?.split(' ');
        splitted.next().context(format!("expected {}", AUTHOR_STR))?;
    }

    // author {identifier} {timestamp} {timezone}
    let mut words = splitted.by_ref().count();
    let mut identifier = splitted.by_ref().take(words - 2).collect::<String>();
    let mut timestamp_str = splitted.next().context("expected timestamp")?;
    let mut timestamp_u64 = timestamp_str.parse::<u64>().context("could not parse timestamp to a number")?;
    let mut timezone = splitted.next().context("expected timezone")?;
    let author = CommitUser {
        kind: CommitUserKind::Author,
        identifier,
        timestamp: UNIX_EPOCH.checked_add(Duration::from_secs(timestamp_u64)).context("author timestamp was invalid")?,
        timezone: UtcOffset::parse(timezone, TIMEZONE_FORMAT).context("timezone was invalid")?
    };

    // committer {identifier} {timestamp} {timezone}
    splitted = lines.next().context("commit file ended abruptly")?.split(' ');
    words = splitted.by_ref().count();
    identifier = splitted.by_ref().take(words - 2).collect::<String>();
    timestamp_str = splitted.next().context("expected timestamp")?;
    timestamp_u64 = timestamp_str.parse::<u64>().context("could not parse timestamp to a number")?;
    timezone = splitted.next().context("expected timezone")?;
    let committer = CommitUser {
        kind: CommitUserKind::Author,
        identifier,
        timestamp: UNIX_EPOCH.checked_add(Duration::from_secs(timestamp_u64)).context("author timestamp was invalid")?,
        timezone: UtcOffset::parse(timezone, TIMEZONE_FORMAT).context("timezone was invalid")?
    };

    let message = lines.next().unwrap_or("");

    Ok(Object::Commit {
        tree: tree_hash,
        parent,
        author,
        committer,
        message: message.into(),
    })
}
