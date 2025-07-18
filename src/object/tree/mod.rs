pub mod builder;
mod tree;

pub use builder::TreeBuilder;
pub use tree::TreeEntry;
pub use tree::{as_bytes, from_bytes};
