//! System Telemetry Minimal: Complete Integration Point
//!
//! Generic, deterministic, portable system monitoring with:
//! - Q64.64 fixed-point arithmetic (deterministic across platforms)
//! - Menger Sponge fractal tensor sparsification (fundamental, not optional)
//! - 7-layer immutable pipeline (L1-L7 with SHA-256 commitment)
//! - C-compatible FFI (safe, no-std Rust)
//!
//! Optional features:
//! - `gudermannian-projection`: Smooth conformal observable mapping (pioneering)
//! - `byzantine-hardening`: Merkle DAG + PBFT-lite consensus + replay validation
//!
//! Feature: `full` enables all optional layers.

#![no_std]
#![allow(non_snake_case)]

extern crate alloc;

// Core modules (always compiled)
pub mod kernel;
pub mod binary_api;

#[cfg(test)]
pub mod test_suite;

// Optional modules (feature-gated)
#[cfg(feature = "gudermannian-projection")]
pub mod gudermannian;

#[cfg(feature = "gudermannian-projection")]
pub use gudermannian::{
    gd_q64, gd_inv_q64, tanh_q64, sech_q64, sinh_q64,
    GudermannianProjector, FrameSnapshotGudermannian,
    verify_invertibility, verify_conformality,
};

#[cfg(feature = "byzantine-hardening")]
pub mod byzantine;

#[cfg(feature = "byzantine-hardening")]
pub use byzantine::{
    MerkleDAG, ShardChain, PBFTLiteConsensus,
    DeterministicReplayValidator, AuditRecord, AuditZone,
    HashProtocolVersion, FrameIntegrityProof,
};

// Core re-exports (unconditional)
pub use kernel::{
    SystemTelemetry, FrameSnapshot, process_frame, ProcessError,
    STATE_DIM, DIM, HASH_SIZE, quantize_q64, dequantize_q64,
};

pub use binary_api::{
    telemetry_init, telemetry_destroy, telemetry_process,
    telemetry_init_baseline, telemetry_init_embedded, telemetry_init_batch,
    telemetry_get_observable, telemetry_get_observables,
    telemetry_get_state, telemetry_get_residual, telemetry_get_hash,
    telemetry_frame_count, telemetry_last_timestamp,
    telemetry_menger_depth, telemetry_menger_set_depth,
    telemetry_version, telemetry_build_info,
};

/// Library version
pub const VERSION: &str = "1.0.0-minimal-complete";
/// Protocol version (bumped with Byzantine feature)
#[cfg(feature = "byzantine-hardening")]
pub const PROTOCOL_VERSION: &str = "v2-byzantine";
#[cfg(not(feature = "byzantine-hardening"))]
pub const PROTOCOL_VERSION: &str = "v1-baseline";
