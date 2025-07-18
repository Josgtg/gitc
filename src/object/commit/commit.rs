use std::rc::Rc;
use std::str::{FromStr, Split};
use std::time::{Duration, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use time::UtcOffset;

use crate::hashing::Hash;
use crate::object::Object;

use super::*;

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
    parents: &[Hash],
    author: &CommitUser,
    commiter: &CommitUser,
    message: &str,
) -> Result<Rc<[u8]>> {
    let mut file = format!("{} {}\n", TREE_STR, tree_hash);
    for hash in parents.iter() {
        file.push_str(&format!("{} {}\n", PARENT_STR, hash));
    }
    file.push_str(&format_commituser(author)?);
    file.push_str(&format_commituser(commiter)?);
    file.push_str(&format!("\n{}", message));

    Ok(file.as_bytes().into())
}

fn format_commituser(user: &CommitUser) -> Result<String> {
    Ok(format!(
        "{} {} {} {}\n",
        user.kind,
        user.identifier,
        user.timestamp
            .duration_since(UNIX_EPOCH)
            .context("timestamp was invalid")?
            .as_secs(),
        user.timezone.format(TIMEZONE_FORMAT).unwrap(),
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
        .context("commit object did not have any lines")?
        .split(' ');
    if splitted.next() != Some(TREE_STR) {
        bail!("expected commit object to start with {}", TREE_STR)
    }
    let tree_hash_str = splitted
        .next()
        .context(format!("expected hash after {}", TREE_STR))?;
    let tree_hash = Hash::from_str(tree_hash_str).context(format!(
        "could not create a hash from the {} hash string",
        TREE_STR
    ))?;

    // parent {parent_hash} (if exists)
    splitted = lines
        .next()
        .context("commit object only had one line")?
        .split(' ');
    let mut next = splitted.next().context(format!(
        "expected either {} or {}, got nothing",
        PARENT_STR, AUTHOR_STR
    ))?;

    // Parsing (possibly) multiple parents
    let mut parents = Vec::new();
    let mut parent_hash_str: &str;
    while next == PARENT_STR {
        parent_hash_str = splitted
            .next()
            .context(format!("expected hash after {}", PARENT_STR))?;
        parents.push(Hash::from_str(parent_hash_str).context(format!(
            "could not create a hash from the {} hash string",
            PARENT_STR
        ))?);
        // Updating, the next code expects `splitted` to be at the second word of the author line
        splitted = lines
            .next()
            .context("commit file ended abruptly")?
            .split(' ');
        next = splitted
            .next()
            .context(format!("expected {}, got nothing", AUTHOR_STR))?;
    }

    if next != AUTHOR_STR {
        bail!("expected {}", AUTHOR_STR)
    }
    // author {identifier} {timestamp} {timezone}
    let author_vec: Vec<&str> = splitted.collect();
    let mut words = author_vec.len();
    let mut identifier = author_vec[..words - 2].join(" ");
    if identifier.is_empty() {
        bail!("expected identifier when reading author")
    }
    let mut timestamp_str = *author_vec
        .get(words - 2)
        .context("expected timestamp when reading author")?;
    let mut timestamp_u64 = timestamp_str
        .parse::<u64>()
        .context("could not parse timestamp to a number when reading author")?;
    let mut timezone = *author_vec
        .get(words - 1)
        .context("expected timezone when reading author")?;
    let author = CommitUser {
        kind: CommitUserKind::Author,
        identifier,
        timestamp: UNIX_EPOCH
            .checked_add(Duration::from_secs(timestamp_u64))
            .context("author timestamp was invalid")?,
        timezone: UtcOffset::parse(timezone, TIMEZONE_FORMAT)
            .context("author timezone was invalid")?,
    };

    // committer {identifier} {timestamp} {timezone}
    splitted = lines
        .next()
        .context("commit file ended abruptly")?
        .split(' ');
    if splitted.next() != Some(COMMITTER_STR) {
        bail!("expected {}", COMMITTER_STR)
    }
    let committer_vec: Vec<&str> = splitted.collect();
    words = committer_vec.len();
    identifier = committer_vec[..words - 2].join(" ");
    if identifier.is_empty() {
        bail!("expected identifier when reading commiter")
    }
    timestamp_str = *committer_vec
        .get(words - 2)
        .context("expected timestamp when reading committer")?;
    timestamp_u64 = timestamp_str
        .parse::<u64>()
        .context("could not parse timestamp to a number when reading committer")?;
    timezone = *committer_vec
        .get(words - 1)
        .context("expected timezone when reading committer")?;
    let committer = CommitUser {
        kind: CommitUserKind::Author,
        identifier,
        timestamp: UNIX_EPOCH
            .checked_add(Duration::from_secs(timestamp_u64))
            .context("committer timestamp was invalid")?,
        timezone: UtcOffset::parse(timezone, TIMEZONE_FORMAT)
            .context("committer timezone was invalid")?,
    };

    lines.next(); // skipping empty newline

    let message = lines.collect::<Vec<&str>>().join("\n");

    Ok(Object::Commit {
        tree: tree_hash,
        parents: parents.into(),
        author,
        committer,
        message: message.into(),
    })
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Hash;
    use std::time::{Duration, UNIX_EPOCH};
    use time::UtcOffset;

    const TEST_TREE_HASH: &str = "980a72fb0cd5a4985c44cba8a407e79db7e83e32";
    const TEST_PARENT_HASH: &str = "0c9d7797a0643d9f4c6b5b0ab25daa28818e7d7f";
    const TEST_AUTHOR_NAME: &str = "John Doe";
    const TEST_AUTHOR_EMAIL: &str = "john@example.com";
    const TEST_COMMITTER_NAME: &str = "Jane Smith";
    const TEST_COMMITTER_EMAIL: &str = "jane@example.com";
    const TEST_TIMESTAMP_AUTHOR: u64 = 1640995200;
    const TEST_TIMESTAMP_COMMITTER: u64 = 1640995260;
    const TEST_TIMEZONE_OFFSET: i8 = -5;
    const TEST_TIMEZONE_OFFSET_STR: &str = "-0500";
    const TEST_TIMEZONE_UTC: i8 = 1;
    const TEST_TIMEZONE_UTC_STR: &str = "+0100";
    const TEST_TIMEZONE_POSITIVE: i8 = 5;
    const TEST_TIMEZONE_NEGATIVE: i8 = -8;
    const TEST_MESSAGE: &str = "Initial commit";
    const TEST_MESSAGE_MULTILINE: &str = "Test commit message";
    const TEST_MESSAGE_TIMEZONE: &str = "Test timezones";

    fn create_test_hash(value: &str) -> Hash {
        Hash::from_str(value).unwrap()
    }

    fn create_test_user(
        kind: CommitUserKind,
        name: &str,
        email: &str,
        timestamp_secs: u64,
        offset_hours: i8,
    ) -> CommitUser {
        CommitUser {
            kind,
            identifier: format!("{} <{}>", name, email),
            timestamp: UNIX_EPOCH + Duration::from_secs(timestamp_secs),
            timezone: UtcOffset::from_hms(offset_hours, 0, 0).unwrap(),
        }
    }

    // Tests for CommitUserKind

    #[test]
    fn test_commit_user_kind_from_str() {
        assert!(matches!(
            CommitUserKind::from_str("author"),
            Ok(CommitUserKind::Author)
        ));
        assert!(matches!(
            CommitUserKind::from_str("committer"),
            Ok(CommitUserKind::Commiter)
        ));
        assert!(CommitUserKind::from_str("invalid").is_err());
    }

    #[test]
    fn test_commit_user_kind_display() {
        assert_eq!(CommitUserKind::Author.to_string(), "author");
        assert_eq!(CommitUserKind::Commiter.to_string(), "committer");
    }

    // Tests for as_bytes

    #[test]
    fn test_as_bytes_with_parent() {
        let tree_hash = create_test_hash(TEST_TREE_HASH);
        let parent_hash = create_test_hash(TEST_PARENT_HASH);
        let author = create_test_user(
            CommitUserKind::Author,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_OFFSET,
        );
        let committer = create_test_user(
            CommitUserKind::Commiter,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_OFFSET,
        );

        let result = as_bytes(
            &tree_hash,
            &[parent_hash],
            &author,
            &committer,
            TEST_MESSAGE,
        );

        let bytes = result.unwrap();
        let commit_str = std::str::from_utf8(&bytes).unwrap();
        let mut commit_lines = commit_str.lines();

        assert_eq!(
            commit_lines.next().unwrap(),
            &format!("tree {}", TEST_TREE_HASH)
        );
        assert_eq!(
            commit_lines.next().unwrap(),
            &format!("parent {}", TEST_PARENT_HASH)
        );
        assert_eq!(
            commit_lines.next().unwrap(),
            &format!(
                "author {} <{}> {} {}",
                TEST_AUTHOR_NAME,
                TEST_AUTHOR_EMAIL,
                TEST_TIMESTAMP_AUTHOR,
                TEST_TIMEZONE_OFFSET_STR
            )
        );
        assert_eq!(
            commit_lines.next().unwrap(),
            &format!(
                "committer {} <{}> {} {}",
                TEST_COMMITTER_NAME,
                TEST_COMMITTER_EMAIL,
                TEST_TIMESTAMP_COMMITTER,
                TEST_TIMEZONE_OFFSET_STR
            )
        );
        commit_lines.next(); // Skipping empty new line
        assert_eq!(commit_lines.next().unwrap(), TEST_MESSAGE);
    }

    #[test]
    fn test_as_bytes_without_parent() {
        let tree_hash = create_test_hash(TEST_TREE_HASH);
        let author = create_test_user(
            CommitUserKind::Author,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_UTC,
        );
        let committer = create_test_user(
            CommitUserKind::Commiter,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_UTC,
        );

        let result = as_bytes(&tree_hash, &[], &author, &committer, TEST_MESSAGE);

        let bytes = result.unwrap();
        let commit_str = std::str::from_utf8(&bytes).unwrap();
        let mut commit_lines = commit_str.lines();

        assert_eq!(
            commit_lines.next().unwrap(),
            &format!("tree {}", TEST_TREE_HASH)
        );
        assert_eq!(
            commit_lines.next().unwrap(),
            &format!(
                "author {} <{}> {} {}",
                TEST_AUTHOR_NAME, TEST_AUTHOR_EMAIL, TEST_TIMESTAMP_AUTHOR, TEST_TIMEZONE_UTC_STR
            )
        );
        assert_eq!(
            commit_lines.next().unwrap(),
            &format!(
                "committer {} <{}> {} {}",
                TEST_COMMITTER_NAME,
                TEST_COMMITTER_EMAIL,
                TEST_TIMESTAMP_COMMITTER,
                TEST_TIMEZONE_UTC_STR
            )
        );
        commit_lines.next(); // Skipping empty new line
        assert_eq!(commit_lines.next().unwrap(), TEST_MESSAGE);
    }

    // Tests for from_bytes

    #[test]
    fn test_from_bytes_with_parent() {
        let commit_data = format!(
            "tree {}\nparent {}\nauthor {} <{}> {} {}\ncommitter {} <{}> {} {}\n\n{}",
            TEST_TREE_HASH,
            TEST_PARENT_HASH,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_OFFSET_STR,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_OFFSET_STR,
            TEST_MESSAGE
        );

        let result = from_bytes(commit_data.as_bytes());

        if let Object::Commit {
            tree,
            parents,
            author,
            committer,
            message,
        } = result.unwrap()
        {
            assert_eq!(tree.to_string(), TEST_TREE_HASH);
            assert_eq!(parents.get(0).unwrap().to_string(), TEST_PARENT_HASH);
            assert_eq!(
                author.identifier,
                format!("{} <{}>", TEST_AUTHOR_NAME, TEST_AUTHOR_EMAIL)
            );
            assert_eq!(
                committer.identifier,
                format!("{} <{}>", TEST_COMMITTER_NAME, TEST_COMMITTER_EMAIL)
            );
            assert_eq!(message, TEST_MESSAGE.into());
        } else {
            panic!("Expected Commit object");
        }
    }

    #[test]
    fn test_from_bytes_without_parent() {
        let commit_data = format!(
            "tree {}\nauthor {} <{}> {} {}\ncommitter {} <{}> {} {}\n\n{}",
            TEST_TREE_HASH,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_UTC_STR,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_UTC_STR,
            TEST_MESSAGE
        );

        let result = from_bytes(commit_data.as_bytes());

        if let Object::Commit {
            tree,
            parents,
            author,
            committer,
            message,
        } = result.unwrap()
        {
            assert_eq!(tree.to_string(), TEST_TREE_HASH);
            assert!(parents.is_empty());
            assert_eq!(
                author.identifier,
                format!("{} <{}>", TEST_AUTHOR_NAME, TEST_AUTHOR_EMAIL)
            );
            assert_eq!(
                committer.identifier,
                format!("{} <{}>", TEST_COMMITTER_NAME, TEST_COMMITTER_EMAIL)
            );
            assert_eq!(message, TEST_MESSAGE.into());
        } else {
            panic!("Expected Commit object");
        }
    }

    #[test]
    fn test_round_trip_with_parent() {
        let tree_hash = create_test_hash(TEST_TREE_HASH);
        let parent_hash = create_test_hash(TEST_PARENT_HASH);
        let author = create_test_user(
            CommitUserKind::Author,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_OFFSET,
        );
        let committer = create_test_user(
            CommitUserKind::Commiter,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_OFFSET,
        );

        // Serialize to bytes
        let bytes = as_bytes(
            &tree_hash,
            &[parent_hash],
            &author,
            &committer,
            TEST_MESSAGE_MULTILINE,
        )
        .unwrap();

        // Parse back from bytes
        let parsed = from_bytes(&bytes).unwrap();

        if let Object::Commit {
            tree,
            parents,
            author: parsed_author,
            committer: parsed_committer,
            message: parsed_message,
        } = parsed
        {
            assert_eq!(tree.to_string(), TEST_TREE_HASH);
            assert_eq!(parents.get(0).unwrap().to_string(), TEST_PARENT_HASH);
            assert_eq!(
                parsed_author.identifier,
                format!("{} <{}>", TEST_AUTHOR_NAME, TEST_AUTHOR_EMAIL)
            );
            assert_eq!(
                parsed_committer.identifier,
                format!("{} <{}>", TEST_COMMITTER_NAME, TEST_COMMITTER_EMAIL)
            );
            assert_eq!(parsed_message, TEST_MESSAGE_MULTILINE.into());
        } else {
            panic!("Expected Commit object");
        }
    }

    #[test]
    fn test_round_trip_without_parent() {
        let tree_hash = create_test_hash(TEST_TREE_HASH);
        let author = create_test_user(
            CommitUserKind::Author,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_UTC,
        );
        let committer = create_test_user(
            CommitUserKind::Commiter,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_UTC,
        );

        // Serialize to bytes
        let bytes = as_bytes(&tree_hash, &[], &author, &committer, TEST_MESSAGE).unwrap();

        // Parse back from bytes
        let parsed = from_bytes(&bytes).unwrap();

        if let Object::Commit {
            tree,
            parents,
            author: parsed_author,
            committer: parsed_committer,
            message: parsed_message,
        } = parsed
        {
            assert_eq!(tree.to_string(), TEST_TREE_HASH);
            assert!(parents.is_empty());
            assert_eq!(
                parsed_author.identifier,
                format!("{} <{}>", TEST_AUTHOR_NAME, TEST_AUTHOR_EMAIL)
            );
            assert_eq!(
                parsed_committer.identifier,
                format!("{} <{}>", TEST_COMMITTER_NAME, TEST_COMMITTER_EMAIL)
            );
            assert_eq!(parsed_message, TEST_MESSAGE.into());
        } else {
            panic!("Expected Commit object");
        }
    }

    #[test]
    fn test_from_bytes_malformed_missing_tree() {
        let commit_data = format!(
            "author {} <{}> {} {}\ncommitter {} <{}> {} {}\n\nTest",
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_UTC_STR,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_UTC_STR
        );

        let result = from_bytes(commit_data.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_from_bytes_malformed_invalid_tree() {
        let commit_data = format!(
            "notree {}\nauthor {} <{}> {} {}\ncommitter {} <{}> {} {}\n\nTest",
            TEST_TREE_HASH,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_UTC_STR,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_OFFSET_STR
        );

        let result = from_bytes(commit_data.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_from_bytes_malformed_invalid_timestamp() {
        let commit_data = format!(
            "tree {}\nauthor {} <{}> notanumber {}\ncommitter {} <{}> {} {}\n\nTest",
            TEST_TREE_HASH,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMEZONE_UTC_STR,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_UTC_STR
        );

        let result = from_bytes(commit_data.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_from_bytes_malformed_invalid_timezone() {
        let commit_data = format!(
            "tree {}\nauthor {} <{}> {} invalid\ncommitter {} <{}> {} {}\n\nTest",
            TEST_TREE_HASH,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_UTC_STR
        );

        let result = from_bytes(commit_data.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn test_from_bytes_empty_message() {
        let commit_data = format!(
            "tree {}\nauthor {} <{}> {} {}\ncommitter {} <{}> {} {}\n\n",
            TEST_TREE_HASH,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_UTC_STR,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_UTC_STR
        );

        let result = from_bytes(commit_data.as_bytes());

        if let Object::Commit { message, .. } = result.unwrap() {
            assert_eq!(message, "".into());
        } else {
            panic!("Expected Commit object");
        }
    }

    #[test]
    fn test_from_bytes_multiline_message() {
        let og_message = "This is a multiline\ncommit message\nwith multiple lines!";
        let commit_data = format!(
            "tree {}\nauthor {} <{}> {} {}\ncommitter {} <{}> {} {}\n\n{}",
            TEST_TREE_HASH,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_UTC_STR,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_UTC_STR,
            og_message,
        );

        let result = from_bytes(commit_data.as_bytes());

        if let Object::Commit { message, .. } = result.unwrap() {
            assert_eq!(message, og_message.into());
        } else {
            panic!("Expected Commit object");
        }
    }

    #[test]
    fn test_different_timezones() {
        let tree_hash = create_test_hash(TEST_TREE_HASH);
        let author = create_test_user(
            CommitUserKind::Author,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_POSITIVE,
        );
        let committer = create_test_user(
            CommitUserKind::Commiter,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_NEGATIVE,
        );

        let bytes = as_bytes(&tree_hash, &[], &author, &committer, TEST_MESSAGE_TIMEZONE).unwrap();
        let parsed = from_bytes(&bytes).unwrap();

        if let Object::Commit {
            author: parsed_author,
            committer: parsed_committer,
            ..
        } = parsed
        {
            assert_eq!(parsed_author.timezone.whole_hours(), TEST_TIMEZONE_POSITIVE);
            assert_eq!(
                parsed_committer.timezone.whole_hours(),
                TEST_TIMEZONE_NEGATIVE
            );
        } else {
            panic!("Expected Commit object");
        }
    }

    #[test]
    pub fn test_multiple_parent() {
        let tree_hash = create_test_hash(TEST_TREE_HASH);
        let parents_num = 10;
        let mut parent_hashes = Vec::new();
        for _ in 0..parents_num {
            parent_hashes.push(create_test_hash(TEST_PARENT_HASH));
        }
        let author = create_test_user(
            CommitUserKind::Author,
            TEST_AUTHOR_NAME,
            TEST_AUTHOR_EMAIL,
            TEST_TIMESTAMP_AUTHOR,
            TEST_TIMEZONE_OFFSET,
        );
        let committer = create_test_user(
            CommitUserKind::Commiter,
            TEST_COMMITTER_NAME,
            TEST_COMMITTER_EMAIL,
            TEST_TIMESTAMP_COMMITTER,
            TEST_TIMEZONE_OFFSET,
        );

        let bytes = as_bytes(
            &tree_hash,
            &parent_hashes,
            &author,
            &committer,
            TEST_MESSAGE,
        )
        .unwrap();

        let parsed = from_bytes(&bytes).unwrap();

        if let Object::Commit { parents, .. } = parsed {
            assert!(parents.len() == parents_num);
            for p in parents.iter() {
                assert_eq!(p.to_string(), TEST_PARENT_HASH);
            }
        } else {
            panic!("expected commit object")
        }
    }
}
