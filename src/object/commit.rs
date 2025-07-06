use std::rc::Rc;
use std::time::SystemTime;

use anyhow::Result;
use time_tz::Tz;

use crate::hashing::Hash;
use crate::user::User;

use super::Object;

#[derive(Debug)]
pub enum CommitUserKind {
    Author,
    Commiter,
}

#[derive(Debug)]
pub struct CommitUser {
    kind: CommitUserKind,
    /// Generally name and email
    identifier: String,
    timestamp: SystemTime,
    timezone: Tz,
}

pub fn as_bytes(tree_hash: &Hash, author: &User, commiter: &User, message: &str) -> Result<Rc<[u8]>> {
    todo!()
}

pub fn from_bytes(bytes: &[u8]) -> Result<Object> {
    todo!()
}
