use std::fs::File;
use std::io::Cursor;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::rc::Rc;
use std::time::UNIX_EPOCH;
use std::{ffi::OsString, io::Write};

use byteorder::{BigEndian, WriteBytesExt};

use crate::byteable::Byteable;
use crate::hashing::Hash;
use crate::Result;

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
    /// hash the object this file index represents
    object_hash: [u8; 20],
    /// state path length
    flags: u16,
    path: OsString,
}

impl IndexEntry {

    const ASSUME_VALID_FLAG_POSITION: u16 = 0b1011_1111_1111_1111;
    const SKIP_WORKTREE_FLAG_POSITION: u16 = 0b1101_1111_1111_1111;     
    const INTEND_TO_ADD_FLAG_POSITION: u16 = 0b1110_1111_1111_1111;
    const PATH_LEN_FLAG_POSITION: u16 = 0x0FFF;

    /// Tries to build an index entry from the file at `path` and the hash of the blob object for said file.
    ///
    /// # Errors
    ///
    /// This function will fail if:
    /// - The file in the provided path could not be opened.
    /// - It wasn't able to get the metadata of the file.
    pub fn try_from_file(file_path: &Path, object_hash: Hash) -> Result<Self> {
        let file = File::open(file_path)?;
        let metadata = file.metadata()?;
        Ok(IndexEntry {
            creation_time_sec: metadata.created()?.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            creation_time_nsec: metadata.created()?.duration_since(UNIX_EPOCH)?.subsec_nanos() as u32,
            modification_time_sec: metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            modification_time_nsec: metadata.modified()?.duration_since(UNIX_EPOCH)?.subsec_nanos() as u32,
            device: metadata.dev() as u32,
            inode: metadata.ino() as u32,
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            file_size: metadata.size() as u32,
            object_hash: object_hash.into(),
            flags: IndexEntry::default_flags(file_path.as_os_str().len()),
            path: file_path.into()
        })
    }

    /// Returns a 16 bit integer where the first 12 bytes store the length of a path, maxed at
    /// 0xFFF. The next three bytes store the flags (all set to false):
    /// - `assume valid`
    /// - `skip worktree`
    /// - `intent to add`
    /// The last bit is not used.
    fn default_flags(path_len: usize) -> u16 {
        path_len.min(0xFFF) as u16
    }

    /// Returns the 15th bit of the flags.
    pub fn is_assumed_valid(&self) -> bool {
        self.flags & IndexEntry::ASSUME_VALID_FLAG_POSITION != 0
    }
    pub fn set_assumed_valid(&mut self, value: bool) {
        self.flags = match value {
            true => self.flags | IndexEntry::ASSUME_VALID_FLAG_POSITION,
            false => self.flags & IndexEntry::ASSUME_VALID_FLAG_POSITION,
        }
    }

    /// Returns the 14th bit of the flags.
    pub fn is_skip_worktree(&self) -> bool {
        self.flags & IndexEntry::SKIP_WORKTREE_FLAG_POSITION != 0
    }
    pub fn set_skip_worktree(&mut self, value: bool) {
        self.flags = match value {
            true => self.flags | IndexEntry::SKIP_WORKTREE_FLAG_POSITION,
            false => self.flags & IndexEntry::SKIP_WORKTREE_FLAG_POSITION,
        }
    }

    /// Returns the 13th bit of the flags.
    pub fn is_intent_to_add(&self) -> bool {
        self.flags & IndexEntry::INTEND_TO_ADD_FLAG_POSITION != 0
    }
    pub fn set_intent_to_add(&mut self, value: bool) {
        self.flags = match value {
            true => self.flags | IndexEntry::INTEND_TO_ADD_FLAG_POSITION,
            false => self.flags & IndexEntry::INTEND_TO_ADD_FLAG_POSITION,
        }
    }

    /// Returns the first 12 bytes of the flags.
    pub fn name_length(&self) -> u16 {
        self.flags & IndexEntry::PATH_LEN_FLAG_POSITION
    }
}

impl Byteable for IndexEntry {

    /// Reduces this index entry to a binary representation of itself.
    ///
    /// # Errors
    ///
    /// This function can fail if any of the read/write operations made to a cursor fail.
    fn as_bytes(&self) -> Result<Rc<[u8]>> {
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

        Ok(cursor.into_inner().into())
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
