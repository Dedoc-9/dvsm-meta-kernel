/// src/compression/mod.rs
///
/// Compression subsystem for DVSM v3.3
/// Exports: free-list, tile pool, SAEC encoder/decoder
///
/// Module Structure:
/// - free_list.rs:   Lock-free ABA-safe free-list (generic atomic primitive)
/// - tile_pool.rs:   Ally X-specific pool management (domain-specific)
/// - encoder.rs:     SAEC encoder (residual singularity detection, regime-adaptive)
/// - decoder.rs:     SAEC decoder (frame reconstruction)

pub mod free_list;
// pub mod tile_pool;
// pub mod encoder;
// pub mod decoder;

pub use free_list::{FreeListHead, LockFreeFreeList};
// pub use tile_pool::{CompressionTile, TilePool};
