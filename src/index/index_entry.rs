use std::ffi::OsString;
use std::fmt::{Debug, Display};
use std::fs::File;
use std::io::{BufRead, Cursor, Read, Write};
use std::os::unix::{ffi::OsStringExt, fs::MetadataExt};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result, bail};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::Constants;
use crate::byteable::Byteable;
use crate::hashing::Hash;
use crate::utils::path::relative_path;

use super::FileStage;

/// Represents an entry for a file in the git index. It contains all the information needed to
/// recreate a file.
#[derive(Clone)]
pub struct IndexEntry {
    pub creation_time_sec: u32,
    pub creation_time_nsec: u32,
    pub modification_time_sec: u32,
    pub modification_time_nsec: u32,
    pub device: u32,
    pub inode: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub file_size: u32,
    /// hash the object this file index represents
    object_hash: Hash,
    /// state path length
    flags: u16,
    path: PathBuf,
}

#[allow(unused)]
impl IndexEntry {
    pub fn object_hash(&self) -> Hash {
        self.object_hash.clone()
    }

    /// Returns the length (in bytes) of this index entry.
    pub fn len(&self) -> usize {
        // 62 fixed bytes, variable path length and null byte
        let len = 62 + self.path_len() + 1;
        len + (len.next_multiple_of(8) - len)
    }

    /// Returns a reference to the path of this index entry.
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    /// Tries to build an index entry from the file at `path` and the hash of the blob object for said file.
    ///
    /// # Errors
    ///
    /// This function will fail if:
    /// - The file in the provided path could not be opened.
    /// - It wasn't able to get the metadata of the file.
    pub fn try_from_file(file_path: &Path, object_hash: Hash) -> Result<Self> {
        let file =
            File::open(file_path).context("failed to open file when encoding index entry")?;
        let metadata = file
            .metadata()
            .context("could not get file metadata when encoding index entry")?;
        Ok(IndexEntry {
            creation_time_sec: metadata.created()?.duration_since(UNIX_EPOCH)?.as_secs() as u32,
            creation_time_nsec: metadata
                .created()?
                .duration_since(UNIX_EPOCH)?
                .subsec_nanos(),
            modification_time_sec: metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs()
                as u32,
            modification_time_nsec: metadata
                .modified()?
                .duration_since(UNIX_EPOCH)?
                .subsec_nanos(),
            device: metadata.dev() as u32,
            inode: metadata.ino() as u32,
            mode: metadata.mode(),
            uid: metadata.uid(),
            gid: metadata.gid(),
            file_size: metadata.size() as u32,
            object_hash,
            flags: IndexEntry::default_flags(file_path.as_os_str().len()),
            path: relative_path(file_path, &Constants::repository_folder_path())
                .unwrap_or(file_path.into())
                .into(),
        })
    }

    const ASSUME_VALID_FLAG_POSITION: u16 = 0b1101_1111_1111_1111;
    const STAGE_POSITION: u16 = 0b0011_1111_1111_1111;
    const PATH_LEN_FLAG_POSITION: u16 = 0x0FFF;
    const MAX_PATH_LEN: u16 = 0x0FFF;

    /// Returns a 16 bit integer where the first 12 bytes store the length of a path, maxed at
    /// 0xFFF. The next three bytes store:
    /// - 13: assume valid
    /// - 14: extended
    /// - 15-16: stage
    ///
    /// The last bit is not used.
    fn default_flags(path_len: usize) -> u16 {
        path_len.min(IndexEntry::MAX_PATH_LEN as usize) as u16
    }

    /// Returns the 13th bit of the flags.
    pub fn is_assumed_valid(&self) -> bool {
        self.flags & !IndexEntry::ASSUME_VALID_FLAG_POSITION != 0
    }
    pub fn set_assumed_valid(&mut self, value: bool) {
        self.flags = match value {
            true => self.flags | IndexEntry::ASSUME_VALID_FLAG_POSITION,
            false => self.flags & IndexEntry::ASSUME_VALID_FLAG_POSITION,
        }
    }

    /// Returns the 15th to 16th bit of the flags.
    pub fn get_stage(&self) -> FileStage {
        FileStage::try_from(self.flags & !IndexEntry::STAGE_POSITION)
            .context("index entry did not have a valid stage")
            .unwrap()
    }
    pub fn set_stage(&mut self, stage: FileStage) {
        let stage_u16 = stage as u16;
        self.flags &= IndexEntry::STAGE_POSITION;
        self.flags |= stage_u16;
    }

    /// Returns the first 12 bytes of the flags.
    pub fn flag_path_len(&self) -> u16 {
        self.flags & IndexEntry::PATH_LEN_FLAG_POSITION
    }

    /// Returns the length of the entry's path.
    pub fn path_len(&self) -> usize {
        self.path.as_os_str().len()
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
        let data_len = 62 + self.path_len() + 1;
        let bytes: Vec<u8> = Vec::with_capacity(data_len);

        let mut cursor = Cursor::new(bytes);

        cursor
            .write_u32::<BigEndian>(self.creation_time_sec)
            .context("could not write creation_time_sec when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.creation_time_nsec)
            .context("could not write creation_time_nsec when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.modification_time_sec)
            .context("could not write modification_time_sec when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.modification_time_nsec)
            .context("could not write modification_time_nsec when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.device)
            .context("could not write device when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.inode)
            .context("could not write inode when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.mode)
            .context("could not write mode when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.uid)
            .context("could not write uid when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.gid)
            .context("could not write gid when encoding index entry")?;
        cursor
            .write_u32::<BigEndian>(self.file_size)
            .context("could not write file_size when encoding index entry")?;

        cursor
            .write_all(self.object_hash.as_ref())
            .context("could not write object_hash when encoding index entry")?;

        cursor
            .write_u16::<BigEndian>(self.flags)
            .context("could not write flags when encoding index entry")?;

        cursor
            .write_all(self.path.as_os_str().as_encoded_bytes())
            .context("could not write path when encoding index entry")?;
        cursor.write_u8(b'\0')?;

        let inner_len = cursor.get_ref().len();
        let padding = inner_len.next_multiple_of(8) - inner_len;
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
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes);

        let entry = IndexEntry {
            creation_time_sec: cursor
                .read_u32::<BigEndian>()
                .context("could not read creation_time_sec when decoding index entry")?,
            creation_time_nsec: cursor
                .read_u32::<BigEndian>()
                .context("could not read creation_time_nsec when decoding index entry")?,
            modification_time_sec: cursor
                .read_u32::<BigEndian>()
                .context("could not read modification_time_sec when decoding index entry")?,
            modification_time_nsec: cursor
                .read_u32::<BigEndian>()
                .context("could not read modification_time_nsec when decoding index entry")?,
            device: cursor
                .read_u32::<BigEndian>()
                .context("could not read device when decoding index entry")?,
            inode: cursor
                .read_u32::<BigEndian>()
                .context("could not read inode when decoding index entry")?,
            mode: cursor
                .read_u32::<BigEndian>()
                .context("could not read mode when decoding index entry")?,
            uid: cursor
                .read_u32::<BigEndian>()
                .context("could not read uid when decoding index entry")?,
            gid: cursor
                .read_u32::<BigEndian>()
                .context("could not read gid when decoding index entry")?,
            file_size: cursor
                .read_u32::<BigEndian>()
                .context("could not read file_size when decoding index entry")?,

            object_hash: {
                let mut hash_buf: [u8; 20] = [0; 20];
                cursor
                    .read_exact(&mut hash_buf)
                    .context("could not read object hash when decoding index entry")?;
                Hash::from(hash_buf)
            },

            flags: cursor
                .read_u16::<BigEndian>()
                .context("could not read flags when decoding index entry")?,

            path: {
                let mut path_buf = Vec::new();
                cursor
                    .read_until(b'\0', &mut path_buf)
                    .context("could not read path when decoding index entry")?;
                if path_buf.pop() != Some(b'\0') {
                    bail!("expected null byte after index entry path")
                }

                PathBuf::from(OsString::from_vec(path_buf))
            },
        };

        let flag_path_len = entry.flag_path_len();
        let actual_path_len = entry.path_len();

        if flag_path_len != IndexEntry::PATH_LEN_FLAG_POSITION
            && flag_path_len as usize != actual_path_len
        {
            bail!(
                "index entry path length {:?} did not match actual path length {:?}",
                flag_path_len,
                actual_path_len
            )
        }

        let offset = entry.len().next_multiple_of(8) - entry.len();
        for _ in 0..offset {
            cursor.read_u8()?;
        }

        Ok(entry)
    }
}

impl Debug for IndexEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!(
            "path: {}\n\
            object hash: {}\n\
            creation time: {}:{}\n\
            modification time: {}:{}\n\
            device: {}\tinode: {}\n\
            mode: {:o}\tuid: {}\n\
            gid: {}\tfile size: {}\n\
            flags: {}",
            self.path().to_string_lossy(),
            self.object_hash,
            self.creation_time_sec,
            self.creation_time_nsec,
            self.modification_time_sec,
            self.modification_time_nsec,
            self.device,
            self.inode,
            self.mode,
            self.uid,
            self.gid,
            self.file_size,
            self.get_stage() as u16,
        );

        f.write_str(s.as_str())
    }
}

impl Display for IndexEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "{:o} {} {}\t{}",
            self.mode,
            self.object_hash(),
            self.get_stage() as u16,
            self.path().to_string_lossy()
        ))
    }
}
