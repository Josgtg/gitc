use std::io::Cursor;
use std::{ffi::OsString, io::Write};

use byteorder::{BigEndian, WriteBytesExt};

use crate::byteable::Byteable;
use crate::Result;

/// Represents a file stage, mainly related to a merge
#[repr(u8)]
#[derive(Debug)]
pub enum FileStage {
    /// File is tracked and staged normally.
    Normal = 0,
    /// Common ancestor version during a merge.
    Base = 1,
    /// Version from the current branch (HEAD).
    Ours = 2,
    /// Version from the branch being merged in.
    Theirs = 3,
}

/// Represents an entry for a file in the git index. It contains all the information needed to
/// recreate a file.
#[derive(Debug)]
pub struct IndexEntry {
    creation_time_sec: u32,
    creation_time_nsec: u32,
    modification_time_sec: u32,
    modification_time_nsec: u32,
    device: u32,
    inode: u32,
    mode: u32,
    uid: u32,
    gid: u32,
    file_size: u32,
    /// hash of the object this file index represents
    object_hash: [u8; 20],
    /// state and path length
    flags: u16,
    path: OsString,
}

impl Byteable for IndexEntry {

    fn as_bytes(&self) -> Result<Vec<u8>> {
        // 62 fixed bytes, variable path and null byte
        let data_len = 62 + self.path.len() + 1;
        let bytes: Vec<u8> = Vec::with_capacity(data_len);

        let mut cursor = Cursor::new(bytes);

        cursor.write_u32::<BigEndian>(self.creation_time_sec)?;
        cursor.write_u32::<BigEndian>(self.creation_time_nsec)?;
        cursor.write_u32::<BigEndian>(self.modification_time_sec)?;
        cursor.write_u32::<BigEndian>(self.modification_time_nsec)?;
        cursor.write_u32::<BigEndian>(self.device)?;
        cursor.write_u32::<BigEndian>(self.inode)?;
        cursor.write_u32::<BigEndian>(self.mode)?;
        cursor.write_u32::<BigEndian>(self.uid)?;
        cursor.write_u32::<BigEndian>(self.gid)?;
        cursor.write_u32::<BigEndian>(self.file_size)?;
        cursor.write_all(&self.object_hash)?;
        cursor.write_u16::<BigEndian>(self.flags)?;

        cursor.write_all(self.path.as_encoded_bytes())?;
        cursor.write_u8(b'\0')?;

        Ok(cursor.into_inner())
    }

    /// Parses a set of bytes into an `IndexEntry` struct.
    ///
    /// # Errors
    ///
    /// This function will fail if:
    /// - There was an error reading from the bytes.
    /// - The format of the bytes was not the expected one.
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        todo!()
    }
}
