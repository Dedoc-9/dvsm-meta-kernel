/// src/lib.rs
///
/// DVSM v3.3 Reference Implementation
/// Deterministic State-Evolution Manifold for Fixed-Point Computation
///
/// Module Structure (Phase I.3: Compression-Only Plumbing):
/// - dvsm_state:      Core state vector (z_manifold, w_basis, telemetry)
/// - supervisor_loop: Phase I.3 integration (TilePool hook, hysteresis, L1D monitoring)
/// - compression:
///   - free_list:     Lock-free ABA-safe tile free-list
///   - tile_pool:     TilePool management (session-immutable 1MB allocation)
///   - placeholder:   Stub encoder (cache-line traffic simulation)
///
/// **Phase I.4 (GhostSnap):** Deferred to Phase 2
/// **Phase II.0 (SAEC Math):** Ready once infrastructure compiles & tests pass

pub mod dvsm_state;
pub mod supervisor_loop;
pub mod compression;

// Re-export core types for ergonomics
pub use dvsm_state::{DVSMState, SupervisorFlags, CompressionTelemetry};
pub use supervisor_loop::{supervisor_tick, rdtsc};
pub use compression::{FreeListHead, LockFreeFreeList};

/// Crate version (Phase I.3 baseline)
pub const CRATE_VERSION: &str = "3.3.0-phase-i3";
