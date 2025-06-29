use std::rc::Rc;

use crate::Result;

/// Trait that ensures a type can be manipulated in a binary format.
pub trait Byteable {
    fn as_bytes(&self) -> Result<Rc<[u8]>>;
    fn from_bytes(bytes: &[u8]) -> Result<Self> where Self: Sized;
}
