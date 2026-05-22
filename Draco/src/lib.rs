/// src/lib.rs
///
/// DVSM v3.3 Reference Implementation
/// Deterministic State-Evolution Manifold for Fixed-Point Computation
///
/// Module Structure (Phase I.3: Compression + RF/ELF Integration):
/// - dvsm_state:      Core state vector (z_manifold, w_basis, telemetry)
/// - supervisor_loop: Phase I.3 integration (TilePool hook, hysteresis, L1D monitoring)
/// - compression:
///   - free_list:     Lock-free ABA-safe tile free-list
///   - tile_pool:     TilePool management (session-immutable 1MB allocation)
///   - saec_math:     SAEC residual encoder (singularity detection, regime selection)
///   - huffman:       Huffman bitstream encoder (variable-length prefix codes)
/// - rf_elf:         RF/ELF external modality (Phase I.0.5 supervisor hook)
///
/// **Phase I.4 (GhostSnap):** Deferred to Phase 2
/// **Phase II.0 (Coupling Coefficient):** Deferred to Phase 2

pub mod dvsm_state;
pub mod supervisor_loop;
pub mod compression;
pub mod rf_elf;

// Re-export core types for ergonomics
pub use dvsm_state::{DVSMState, SupervisorFlags, CompressionTelemetry};
pub use supervisor_loop::{supervisor_tick, rdtsc};
pub use compression::{FreeListHead, LockFreeFreeList, encode_saec, encode_residuals_huffman};
pub use rf_elf::{RfElfSample, RfElfBuffer, RfElfError, LAYOUT_ID_RF_ELF, MAX_STALE_US};

/// Crate version (Phase I.3 baseline)
pub const CRATE_VERSION: &str = "3.3.0-phase-i3";
