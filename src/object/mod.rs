#[allow(clippy::module_inception)]
mod object;

mod blob;
mod commit;
mod tree;

pub use blob::BlobExt;
pub use object::Object;
pub use tree::{TreeBuilder, TreeEntry};
