pub mod builder;
#[allow(clippy::module_inception)]
mod tree;

pub use builder::TreeBuilder;
pub use tree::TreeEntry;
pub use tree::{as_bytes, display, from_bytes};
