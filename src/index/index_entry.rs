use std::io::Cursor;
use std::{ffi::OsString, io::Write};

use byteorder::{BigEndian, WriteBytesExt};

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

impl IndexEntry {
    pub fn new(
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
        object_hash: [u8; 20],
        flags: u16,
        path: OsString,
    ) -> Self {
        Self {
            creation_time_sec,
            creation_time_nsec,
            modification_time_sec,
            modification_time_nsec,
            device,
            inode,
            mode,
            uid,
            gid,
            file_size,
            object_hash,
            flags,
            path,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        // 62 fixed bytes, variable path and null byte
        let data_len = 62 + self.path.len() + 1;
        let bytes: Vec<u8> = Vec::with_capacity(data_len);

        let mut cursor = Cursor::new(bytes);

        cursor.write_u32::<BigEndian>(self.creation_time_sec).unwrap();
        cursor.write_u32::<BigEndian>(self.creation_time_nsec).unwrap();
        cursor.write_u32::<BigEndian>(self.modification_time_sec).unwrap();
        cursor.write_u32::<BigEndian>(self.modification_time_nsec).unwrap();
        cursor.write_u32::<BigEndian>(self.device).unwrap();
        cursor.write_u32::<BigEndian>(self.inode).unwrap();
        cursor.write_u32::<BigEndian>(self.mode).unwrap();
        cursor.write_u32::<BigEndian>(self.uid).unwrap();
        cursor.write_u32::<BigEndian>(self.gid).unwrap();
        cursor.write_u32::<BigEndian>(self.file_size).unwrap();
        cursor.write_all(&self.object_hash).unwrap();
        cursor.write_u16::<BigEndian>(self.flags).unwrap();

        cursor.write_all(self.path.as_encoded_bytes()).unwrap();
        cursor.write_u8(b'\0').unwrap();

        cursor.into_inner()
    }
}
