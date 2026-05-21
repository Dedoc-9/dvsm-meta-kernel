/// src/compression/tile_pool.rs
///
/// Stub: CompressionTile and TilePool definitions (Phase I.3 Baseline)
/// (Real implementation follows COMPRESSION_CODEC_IMPL.md §1)
///
/// Purpose: Provide the tile structure and pool management for the supervisor loop.
/// This stub is sufficient for baseline L1D cache conflict testing.

use std::sync::atomic::{AtomicUsize, Ordering};
use super::free_list::LockFreeFreeList;

pub const TILE_COUNT: usize = 256;
pub const TILE_SIZE: usize = 4096;

/// 64-byte aligned compression tile (prevents false-sharing on Zen 5)
#[repr(C, align(64))]
pub struct CompressionTile {
    pub data: [u8; TILE_SIZE],
    pub metadata_regime: u8,
    pub sample_count: u32,
}

impl Default for CompressionTile {
    fn default() -> Self {
        CompressionTile {
            data: [0u8; TILE_SIZE],
            metadata_regime: 0,
            sample_count: 0,
        }
    }
}

/// TilePool: Session-immutable 1MB allocation (256 × 4KB tiles)
pub struct TilePool {
    tiles: Box<[CompressionTile; TILE_COUNT]>,
    free_list: LockFreeFreeList,
    occupancy: AtomicUsize,
}

impl TilePool {
    /// Create a new tile pool with all 256 tiles available
    pub fn new() -> Self {
        // Pre-allocate 256 tiles on heap
        let mut tiles_vec = Vec::with_capacity(TILE_COUNT);
        for _ in 0..TILE_COUNT {
            tiles_vec.push(CompressionTile::default());
        }
        let tiles = tiles_vec.into_boxed_slice();
        let tiles: Box<[CompressionTile; TILE_COUNT]> = unsafe {
            Box::from_raw(Box::into_raw(tiles) as *mut [CompressionTile; TILE_COUNT])
        };

        let free_list = LockFreeFreeList::new(TILE_COUNT);

        TilePool {
            tiles,
            free_list,
            occupancy: AtomicUsize::new(0),
        }
    }

    /// Pop a free tile from the pool (ABA-safe, non-blocking)
    pub fn pop_tile(&mut self) -> Option<(usize, &mut CompressionTile)> {
        if let Some(idx) = self.free_list.pop() {
            self.occupancy.fetch_add(1, Ordering::Relaxed);
            // Safety: We hold &mut self, so we can safely mutate tiles
            Some((idx, &mut self.tiles[idx]))
        } else {
            None
        }
    }

    /// Push a tile back to the pool
    pub fn push_tile(&mut self, idx: usize) {
        self.free_list.push(idx);
        self.occupancy.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get current pool occupancy (tiles in use)
    pub fn get_occupancy(&self) -> usize {
        self.occupancy.load(Ordering::Relaxed)
    }

    /// Recommend compression regime based on occupancy
    pub fn get_recommended_regime(&self) -> u8 {
        let occ = self.get_occupancy();
        if occ > 200 { 4 }      // CRITICAL: Phase Shedding
        else if occ > 128 { 2 } // HIGH: Aggressive Q16
        else if occ > 64 { 1 }  // MED: Moderate Q31
        else { 3 }              // LOW: Maximum Singularity (SAEC)
    }
}

impl Default for TilePool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_alignment() {
        assert_eq!(std::mem::align_of::<CompressionTile>(), 64);
    }

    #[test]
    fn test_pool_creation() {
        let pool = TilePool::new();
        assert_eq!(pool.get_occupancy(), 0);
    }

    #[test]
    fn test_regime_selection() {
        let pool = TilePool::new();

        // At 0 occupancy, should recommend Regime 3
        assert_eq!(pool.get_recommended_regime(), 3);
    }
}
