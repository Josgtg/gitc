use std::fs::File;
use std::io::{BufRead, Cursor, Read};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::rc::Rc;
use std::time::UNIX_EPOCH;
use std::{ffi::OsString, io::Write};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::byteable::Byteable;
use crate::error::CustomResult;
use crate::fs::path::relative_path;
use crate::hashing::Hash;
use crate::{Constants, Error, Result};

/// Represents an entry for a file in the git index. It contains all the information needed to
/// recreate a file.
#[derive(Debug, Default, Clone)]
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

    pub fn object_hash(&self) -> Hash {
        Hash::from(self.object_hash)
    }

    /// Tries to build an index entry from the file at `path` and the hash of the blob object for said file.
    ///
    /// # Errors
    ///
    /// This function will fail if:
    /// - The file in the provided path could not be opened.
    /// - It wasn't able to get the metadata of the file.
    pub fn try_from_file(file_path: &Path, object_hash: Hash) -> Result<Self> {
        let file = File::open(file_path).map_err_with("failed to open file when encoding index entry")?;
        let metadata = file.metadata().map_err_with("could not get file metadata when encoding index entry")?;
        Ok(IndexEntry {
            creation_time_sec: metadata.created()?.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            creation_time_nsec: metadata
                .created()?
                .duration_since(UNIX_EPOCH)?
                .subsec_nanos() as u32,
            modification_time_sec: metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs()
                as u32,
            modification_time_nsec: metadata
                .modified()?
                .duration_since(UNIX_EPOCH)?
                .subsec_nanos() as u32,
            device: metadata.dev() as u32,
            inode: metadata.ino() as u32,
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            file_size: metadata.size() as u32,
            object_hash: object_hash.into(),
            flags: IndexEntry::default_flags(file_path.as_os_str().len()),
            path: relative_path(file_path, &Constants::repository_folder_path())
                .unwrap_or(file_path.into())
                .into(),
        })
    }

    /// Returns the length (in bytes) of this index entry.
    pub fn len(&self) -> usize {
        // 62 fixed bytes, variable path length and null byte
        62 + self.path_len() + 1
    }

    const ASSUME_VALID_FLAG_POSITION: u16 = 0b1011_1111_1111_1111;
    const SKIP_WORKTREE_FLAG_POSITION: u16 = 0b1101_1111_1111_1111;
    const INTEND_TO_ADD_FLAG_POSITION: u16 = 0b1110_1111_1111_1111;
    const PATH_LEN_FLAG_POSITION: u16 = 0x0FFF;
    const MAX_PATH_LEN: u16 = 0x0FFF;

    /// Returns a 16 bit integer where the first 12 bytes store the length of a path, maxed at
    /// 0xFFF. The next three bytes store the flags (all set to false):
    /// - `assume valid`
    /// - `skip worktree`
    /// - `intent to add`
    /// The last bit is not used.
    fn default_flags(path_len: usize) -> u16 {
        path_len.min(IndexEntry::MAX_PATH_LEN as usize) as u16
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
    pub fn flag_path_len(&self) -> u16 {
        self.flags & IndexEntry::PATH_LEN_FLAG_POSITION
    }

    /// Returns the length of the entry's path.
    pub fn path_len(&self) -> usize {
        self.path.len()
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

        cursor.write_u32::<BigEndian>(self.creation_time_sec).map_err_with("could not write creation_time_sec when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.creation_time_nsec).map_err_with("could not write creation_time_nsec when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.modification_time_sec).map_err_with("could not write modification_time_sec when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.modification_time_nsec).map_err_with("could not write modification_time_nsec when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.device).map_err_with("could not write device when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.inode).map_err_with("could not write inode when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.mode).map_err_with("could not write mode when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.uid).map_err_with("could not write uid when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.gid).map_err_with("could not write gid when encoding index entry")?;
        cursor.write_u32::<BigEndian>(self.file_size).map_err_with("could not write file_size when encoding index entry")?;
        cursor.write_all(&self.object_hash).map_err_with("could not write object_hash when encoding index entry")?;
        cursor.write_u16::<BigEndian>(self.flags).map_err_with("could not write flags when encoding index entry")?;

        cursor.write_all(self.path.as_encoded_bytes()).map_err_with("could not write path when encoding index entry")?;
        cursor.write_u8(b'\0')?;

        let padding = (8 - (cursor.get_ref().len() % 8)) % 8;
        for _ in 0..padding {
            cursor.write_u8(b'\0')?;
        }

        Ok(cursor.into_inner().into())
    }

    /// Parses a set of bytes into an `IndexEntry` struct.
    ///
    /// # Errors
    ///
    /// This function will fail if:
    /// - There was an error reading from the bytes.
    /// - The format of the bytes was not the expected one.
    fn from_bytes<T: AsRef<[u8]>>(cursor: &mut Cursor<T>) -> Result<Self> {
        let mut entry = IndexEntry::default();

        entry.creation_time_sec = cursor.read_u32::<BigEndian>().map_err_with("could not read creation_time_sec when decoding index entry")?;
        entry.creation_time_nsec = cursor.read_u32::<BigEndian>().map_err_with("could not read creation_time_nsec when decoding index entry")?;
        entry.modification_time_sec = cursor.read_u32::<BigEndian>().map_err_with("could not read modification_time_sec when decoding index entry")?;
        entry.modification_time_nsec = cursor.read_u32::<BigEndian>().map_err_with("could not read modification_time_nsec when decoding index entry")?;
        entry.device = cursor.read_u32::<BigEndian>().map_err_with("could not read device when decoding index entry")?;
        entry.inode = cursor.read_u32::<BigEndian>().map_err_with("could not read inode when decoding index entry")?;
        entry.mode = cursor.read_u32::<BigEndian>().map_err_with("could not read mode when decoding index entry")?;
        entry.uid = cursor.read_u32::<BigEndian>().map_err_with("could not read uid when decoding index entry")?;
        entry.gid = cursor.read_u32::<BigEndian>().map_err_with("could not read gid when decoding index entry")?;
        entry.file_size = cursor.read_u32::<BigEndian>().map_err_with("could not read file_size when decoding index entry")?;
        cursor.read_exact(&mut entry.object_hash).map_err_with("could not read object hash when decoding index entry")?;
        entry.flags = cursor.read_u16::<BigEndian>().map_err_with("could not read flags when decoding index entry")?;

        let mut path_buf = Vec::new();
        cursor.read_until(b'\0', &mut path_buf).map_err_with("could not read path when decoding index entry")?;
        if path_buf.pop() != Some(b'\0') {
            return Err(Error::Formatting(
                "expected null byte after index entry path".into(),
            ));
        }

        if entry.flag_path_len() != IndexEntry::PATH_LEN_FLAG_POSITION
            && entry.flag_path_len() as usize != path_buf.len()
        {
            return Err(Error::DataConsistency(
                format!(
                    "index entry path length {:?} did not match actual path length {:?}",
                    entry.flag_path_len(),
                    path_buf.len()
                )
                .into(),
            ));
        }

        entry.path = OsString::from_vec(path_buf);

        let offset = (8 - (entry.len() % 8)) % 8;
        for _ in 0..offset {
            cursor.read_u8()?;
        }

        dbg!(&entry);

        Ok(entry)
    }
}
