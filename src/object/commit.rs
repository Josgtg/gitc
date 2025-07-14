use std::fmt::Display;
use std::io::{BufRead, Cursor, Read};
use std::rc::Rc;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use time::{macros::format_description, UtcOffset};

use crate::hashing::Hash;

use super::Object;

pub const TREE_STR: &'static str = "tree";
pub const PARENT_STR: &'static str = "parent";

#[derive(Debug)]
pub enum CommitUserKind {
    Author,
    Commiter,
}

impl CommitUserKind {
    const AUTHOR_STR: &'static str = "author";
    const COMMITTER_STR: &'static str = "committer";
}

impl FromStr for CommitUserKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            CommitUserKind::AUTHOR_STR => Ok(CommitUserKind::Author),
            CommitUserKind::COMMITTER_STR => Ok(CommitUserKind::Commiter),
            _ => bail!("invalid commit user kind: {}", s),
        }
    }
}

impl Display for CommitUserKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CommitUserKind::Author => CommitUserKind::AUTHOR_STR,
            CommitUserKind::Commiter => CommitUserKind::COMMITTER_STR,
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
    let mut cursor = Cursor::new(bytes);

    // Verifying the file starts with "tree"
    let mut tree_buf = Vec::new();
    cursor
        .read_until(b' ', &mut tree_buf)
        .context("could not read tree word")?;
    if tree_buf.pop() != Some(b' ') {
        bail!("expected space after tree word")
    }
    let tree_str = String::from_utf8(tree_buf).context("could not parse bytes from tree word")?;
    if tree_str != TREE_STR {
        bail!(
            "expected tree line to start with '{}', got: {}",
            TREE_STR,
            tree_str
        );
    }

    // Reading tree hash
    let mut tree_hash_buf = Vec::new();
    cursor
        .read_until(b'\n', &mut tree_hash_buf)
        .context("could not read tree hash")?;
    if tree_hash_buf.pop() != Some(b'\n') {
        bail!("expected newline after tree hash")
    }
    let tree_hash_str =
        String::from_utf8(tree_hash_buf).context("could not parse bytes from tree hash")?;
    let tree_hash = Hash::from_str(&tree_hash_str)
        .context(format!("could not get hash from string {}", tree_hash_str))?;

    // Ensuring next line starts with "parent" or if it's not present then reading the author
    let mut next_buf = Vec::new(); // Next word can be either PARENT_STR or AUTHOR_STR
    cursor
        .read_until(b' ', &mut next_buf)
        .context("could not read word after tree hash")?;
    if next_buf.pop() != Some(b' ') {
        bail!("expected space after the word after tree hash")
    }
    let next_str = String::from_utf8(next_buf).context("could not parse bytes from word after tree hash")?;

    let mut parent_hash = None;
    if next_str == PARENT_STR {
        // Reading parent hash if next word was PARENT_STR
        let mut parent_hash_buf = Vec::new();
        cursor
            .read_until(b'\n', &mut parent_hash_buf)
            .context("could not read parent hash")?;
        if parent_hash_buf.pop() != Some(b'\n') {
            bail!("expected newline after parent hash")
        }
        let parent_hash_str =
            String::from_utf8(parent_hash_buf).context("could not parse bytes from parent hash")?;
        parent_hash = Some(Hash::from_str(&parent_hash_str).context(format!(
            "could not get hash from string {}",
            parent_hash_str
        ))?);
    } else if next_str != CommitUserKind::AUTHOR_STR {
        // Or bailing if next word was not PARENT_STR nor AUTHOR_STR
        bail!(
            "expected parent line to start with '{}' or {}, got: {}",
            PARENT_STR,
            CommitUserKind::AUTHOR_STR,
            next_str
        );
    }

    let author = parse_commituser(&mut cursor).context("could not parse author")?;
    let committer = parse_commituser(&mut cursor).context("could not parse committer")?;

    let mut message_buf = Vec::new();
    cursor
        .read_to_end(&mut message_buf)
        .context("could not read commit message")?;
    let message = String::from_utf8(message_buf)
        .context("could not parse bytes from commit message")?;

    Ok(Object::Commit {
        tree: tree_hash,
        parent: parent_hash,
        author,
        committer,
        message: message.into(),
    })
}

fn parse_commituser(cursor: &mut Cursor<&[u8]>) -> Result<CommitUser> {
    // Parsing either "commiter" or "author"
    let mut kind_buf = Vec::new();
    cursor
        .read_until(b' ', &mut kind_buf)
        .context("could not read user kind")?;
    if kind_buf.pop() != Some(b' ') {
        bail!("expected space after user kind")
    }
    let s = String::from_utf8(kind_buf).context("could not parse bytes from commit user kind")?;
    let kind = CommitUserKind::from_str(&s).context(format!(
        "string was not a valid commit user kind, got: {}",
        s
    ))?;

    // Reading, for example, name and email
    let mut identifier_buf = Vec::new();
    cursor
        .read_until(b' ', &mut identifier_buf)
        .context("could not read user identifier")?;
    if identifier_buf.pop() != Some(b' ') {
        bail!("expected space after user identifier")
    }
    let identifier = String::from_utf8(identifier_buf)
        .context("could not parse bytes from commit user identifier")?;

    // Reading the UNIX timestamp (seconds since UNIX_EPOCH)
    let mut timestamp_buf = Vec::new();
    cursor
        .read_until(b' ', &mut timestamp_buf)
        .context("could not read user timestamp")?;
    if timestamp_buf.pop() != Some(b' ') {
        bail!("expected space after user timestamp")
    };
    let s = String::from_utf8(timestamp_buf)
        .context("could not parse bytes from commit user timestamp")?;
    let seconds: u64 = s
        .parse()
        .context("could not parse commit user timestamp as u64")?;
    let timestamp = UNIX_EPOCH
        .checked_add(Duration::from_secs(seconds))
        .ok_or_else(|| anyhow!("there was a timestamp overflow"))?;

    // Reading the timezone
    let mut timezone_buf = Vec::new();
    cursor
        .read_until(b'\n', &mut timezone_buf)
        .context("could not read user timezone")?;
    if timezone_buf.pop() != Some(b'\n') {
        bail!("expected newline after user timezone")
    };
    let s = String::from_utf8(timezone_buf)
        .context("could not parse bytes from commit user timezone")?;
    let format = format_description!("[offset_hour][offset_minute]");
    let timezone = UtcOffset::parse(&s, format)
        .context(format!("string was not a valid timezone, got: {}", s))?;

    Ok(CommitUser {
        kind,
        identifier,
        timestamp,
        timezone,
    })
}
