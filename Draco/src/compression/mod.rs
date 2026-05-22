/// src/compression/mod.rs
///
/// Compression subsystem for DVSM v3.3
/// Exports: free-list, tile pool, SAEC encoder, Huffman bitstream
///
/// Module Structure:
/// - free_list.rs:   Lock-free ABA-safe free-list (generic atomic primitive)
/// - tile_pool.rs:   Session-immutable pool management (64-byte aligned)
/// - placeholder.rs: Stub encoder for baseline testing
/// - saec_math.rs:   SAEC encoder (residual singularity detection, regime-adaptive) [Phase I.3]
/// - huffman.rs:     Huffman bitstream encoder (variable-length prefix codes) [Track A, Phase I.3]

pub mod free_list;
pub mod tile_pool;
pub mod placeholder;
pub mod saec_math;
pub mod huffman;

pub use free_list::{FreeListHead, LockFreeFreeList};
pub use tile_pool::{CompressionTile, TilePool};
pub use saec_math::{encode_saec, SAECOutput};
pub use huffman::{encode_residuals_huffman, decode_residuals_huffman, HuffmanOutput, BitstreamError};
