pub(crate) mod io;
mod v1;
mod v2;

pub use v1::hash_v1_pieces;
pub use v2::{compute_merkle_root, hash_v2_files, V2HashResult};
