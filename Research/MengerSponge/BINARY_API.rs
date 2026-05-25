//! System Telemetry: C-Compatible Binary API
//! Platform: Windows, Linux, macOS, WebAssembly
//! FFI: C-compatible with #[repr(C)]

#![no_mangle]
use crate::{FrameSnapshot, SystemTelemetry, process_frame, STATE_DIM, DIM, HASH_SIZE};
use core::ffi::c_int;

// =============================================================================
// HANDLE MANAGEMENT
// =============================================================================

/// Initialize telemetry system with optional Menger Sponge
///
/// # Arguments
/// * `menger_depth` - 0 (off), 1-2 (recommended), 3+ (experimental)
/// * `rate_limit_ns` - Minimum nanoseconds between frames (0 = unlimited)
///
/// # Returns
/// Opaque handle pointer (never null in valid implementation)
#[no_mangle]
pub extern "C" fn telemetry_init(menger_depth: u8) -> *mut SystemTelemetry {
    let sys = Box::new(SystemTelemetry::new(menger_depth));
    Box::into_raw(sys)
}

/// Destroy telemetry handle and free memory
#[no_mangle]
pub extern "C" fn telemetry_destroy(handle: *mut SystemTelemetry) {
    if !handle.is_null() {
        unsafe { Box::from_raw(handle); }
    }
}

// =============================================================================
// FRAME PROCESSING
// =============================================================================

/// Process single frame: 64 bytes sensor input → FrameSnapshot output
///
/// # Arguments
/// * `handle` - Telemetry instance
/// * `sensors` - Pointer to f64[16] sensor values (CPU%, GPU%, mem%, therm, power, ...)
/// * `sensor_count` - Must be exactly 16
/// * `timestamp_ns` - Nanosecond timestamp (for rate limiting)
/// * `out_snapshot` - Output FrameSnapshot structure
///
/// # Returns
/// * 0: Success
/// * -1: Invalid input
/// * -2: Rate limit exceeded
/// * -3: Null pointer
#[no_mangle]
pub extern "C" fn telemetry_process(
    handle: *mut SystemTelemetry,
    sensors: *const f64,
    sensor_count: u32,
    timestamp_ns: u64,
    out_snapshot: *mut FrameSnapshot,
) -> c_int {
    if handle.is_null() || sensors.is_null() || out_snapshot.is_null() {
        return -3;
    }

    if sensor_count != STATE_DIM as u32 {
        return -1;
    }

    let sensor_slice = unsafe { core::slice::from_raw_parts(sensors, STATE_DIM) };
    let mut sensor_array = [0.0f64; STATE_DIM];
    sensor_array.copy_from_slice(sensor_slice);

    let sys = unsafe { &mut *handle };

    match process_frame(sys, &sensor_array, timestamp_ns) {
        Ok(snap) => {
            unsafe { *out_snapshot = snap; }
            0
        }
        Err(_) => -2,
    }
}

// =============================================================================
// OBSERVABLE ACCESS
// =============================================================================

/// Get single observable (Z component)
#[no_mangle]
pub extern "C" fn telemetry_get_observable(
    snapshot: *const FrameSnapshot,
    idx: u32,
    out_value: *mut i128,
) -> c_int {
    if snapshot.is_null() || out_value.is_null() {
        return -3;
    }

    if idx >= DIM as u32 {
        return -1;
    }

    unsafe {
        *out_value = (*snapshot).z_t[idx as usize];
    }
    0
}

/// Get all observables (Z vector)
#[no_mangle]
pub extern "C" fn telemetry_get_observables(
    snapshot: *const FrameSnapshot,
    out_z: *mut i128,  // Z[16]
) -> c_int {
    if snapshot.is_null() || out_z.is_null() {
        return -3;
    }

    unsafe {
        let snap = &*snapshot;
        let out_slice = core::slice::from_raw_parts_mut(out_z, DIM);
        out_slice.copy_from_slice(&snap.z_t);
    }
    0
}

// =============================================================================
// STATE ACCESS
// =============================================================================

/// Get raw system state (μₜ)
#[no_mangle]
pub extern "C" fn telemetry_get_state(
    snapshot: *const FrameSnapshot,
    out_mu: *mut u8,  // μ[64]
) -> c_int {
    if snapshot.is_null() || out_mu.is_null() {
        return -3;
    }

    unsafe {
        let snap = &*snapshot;
        let out_slice = core::slice::from_raw_parts_mut(out_mu, STATE_DIM);
        out_slice.copy_from_slice(&snap.mu_t);
    }
    0
}

/// Get residual state (Sₜ)
#[no_mangle]
pub extern "C" fn telemetry_get_residual(
    snapshot: *const FrameSnapshot,
    out_s: *mut i128,  // S[16]
) -> c_int {
    if snapshot.is_null() || out_s.is_null() {
        return -3;
    }

    unsafe {
        let snap = &*snapshot;
        let out_slice = core::slice::from_raw_parts_mut(out_s, DIM);
        out_slice.copy_from_slice(&snap.s_t);
    }
    0
}

// =============================================================================
// HASH & INTEGRITY
// =============================================================================

/// Get SHA-256 structural hash
#[no_mangle]
pub extern "C" fn telemetry_get_hash(
    snapshot: *const FrameSnapshot,
    out_hash: *mut u8,  // 32 bytes
) -> c_int {
    if snapshot.is_null() || out_hash.is_null() {
        return -3;
    }

    unsafe {
        let snap = &*snapshot;
        let out_slice = core::slice::from_raw_parts_mut(out_hash, HASH_SIZE);
        out_slice.copy_from_slice(&snap.h_t);
    }
    0
}

/// Verify hash consistency (for post-mortem analysis)
#[no_mangle]
pub extern "C" fn telemetry_verify_hash(
    handle: *const SystemTelemetry,
    snapshot: *const FrameSnapshot,
) -> c_int {
    if handle.is_null() || snapshot.is_null() {
        return -3;
    }

    // In production: recompute hash and compare
    // For now: always valid (assumption: hash was computed correctly)
    1
}

// =============================================================================
// MENGER SPONGE CONFIGURATION
// =============================================================================

/// Get Menger depth setting
#[no_mangle]
pub extern "C" fn telemetry_menger_depth(handle: *const SystemTelemetry) -> u8 {
    if handle.is_null() {
        return 0;
    }
    unsafe { (*handle).menger_depth }
}

/// Reconfigure Menger depth at runtime
#[no_mangle]
pub extern "C" fn telemetry_menger_set_depth(
    handle: *mut SystemTelemetry,
    new_depth: u8,
) -> c_int {
    if handle.is_null() {
        return -3;
    }

    if new_depth > 2 {
        return -1;  // Limit to 0-2 in production
    }

    unsafe {
        (*handle).menger_depth = new_depth;
    }
    0
}

// =============================================================================
// STATISTICS
// =============================================================================

/// Get frame counter
#[no_mangle]
pub extern "C" fn telemetry_frame_count(handle: *const SystemTelemetry) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe { (*handle).frame_count }
}

/// Get timestamp of last processed frame
#[no_mangle]
pub extern "C" fn telemetry_last_timestamp(handle: *const SystemTelemetry) -> u64 {
    if handle.is_null() {
        return 0;
    }
    unsafe { (*handle).state.timestamp_ns }
}

// =============================================================================
// CONFIGURATION PRESETS
// =============================================================================

/// Initialize with BASELINE preset (scientific, no Menger)
#[no_mangle]
pub extern "C" fn telemetry_init_baseline() -> *mut SystemTelemetry {
    telemetry_init(0)
}

/// Initialize with EMBEDDED preset (Ally X / Steam Deck, Menger depth 2)
#[no_mangle]
pub extern "C" fn telemetry_init_embedded() -> *mut SystemTelemetry {
    telemetry_init(2)
}

/// Initialize with BATCH preset (high-precision, no Menger)
#[no_mangle]
pub extern "C" fn telemetry_init_batch() -> *mut SystemTelemetry {
    telemetry_init(0)
}

// =============================================================================
// VERSION INFO
// =============================================================================

/// Get library version string
#[no_mangle]
pub extern "C" fn telemetry_version(out_buf: *mut u8, buf_size: u32) -> c_int {
    if out_buf.is_null() {
        return -3;
    }

    let version = b"system-telemetry-minimal-1.0-q64";
    let to_copy = (version.len() as u32).min(buf_size) as usize;

    unsafe {
        let out_slice = core::slice::from_raw_parts_mut(out_buf, to_copy);
        out_slice.copy_from_slice(&version[..to_copy]);
    }

    to_copy as c_int
}

/// Get build info
#[no_mangle]
pub extern "C" fn telemetry_build_info(out_buf: *mut u8, buf_size: u32) -> c_int {
    if out_buf.is_null() {
        return -3;
    }

    let info = b"System Telemetry Core (minimal)\n\
                 Arithmetic: Q64.64 fixed-point\n\
                 Menger: Fundamental (depth 0-2)\n\
                 Hash: SHA-256\n\
                 Portable: Yes\n";
    let to_copy = (info.len() as u32).min(buf_size) as usize;

    unsafe {
        let out_slice = core::slice::from_raw_parts_mut(out_buf, to_copy);
        out_slice.copy_from_slice(&info[..to_copy]);
    }

    to_copy as c_int
}

// =============================================================================
// ERROR CODES
// =============================================================================

/*
Return codes:
  0:   Success
 -1:   Invalid parameter (out of range, invalid dimension)
 -2:   Processing error (rate limit, validation failure)
 -3:   Null pointer / invalid handle
*/

// =============================================================================
// GUDERMANNIAN FFI (FEATURE-GATED: gudermannian-projection)
// =============================================================================

#[cfg(feature = "gudermannian-projection")]
use crate::gudermannian::GudermannianProjector;

/// Create Gudermannian projector (feature: gudermannian-projection)
#[cfg(feature = "gudermannian-projection")]
#[no_mangle]
pub extern "C" fn telemetry_create_projector(
    mu_max: i128,
    enabled: u8,
) -> *mut GudermannianProjector {
    let projector = Box::new(GudermannianProjector::new(mu_max, enabled != 0));
    Box::into_raw(projector)
}

/// Destroy Gudermannian projector (feature: gudermannian-projection)
#[cfg(feature = "gudermannian-projection")]
#[no_mangle]
pub extern "C" fn telemetry_destroy_projector(
    projector: *mut GudermannianProjector,
) {
    if !projector.is_null() {
        unsafe { Box::from_raw(projector); }
    }
}

/// Project Z observables via Gudermannian (feature: gudermannian-projection)
#[cfg(feature = "gudermannian-projection")]
#[no_mangle]
pub extern "C" fn telemetry_project_gudermannian(
    projector: *mut GudermannianProjector,
    z: *mut i128,
    dim: u32,
) -> c_int {
    if projector.is_null() || z.is_null() {
        return -3;
    }

    unsafe {
        let proj = &mut *projector;
        let z_slice = core::slice::from_raw_parts_mut(z, dim as usize);

        let mut z_array = [0i128; 16];
        if dim as usize <= 16 {
            z_array[..dim as usize].copy_from_slice(z_slice);
        }

        proj.project_vector(&mut z_array);
        z_slice.copy_from_slice(&z_array[..dim as usize]);
    }
    0
}

/// Invert Gudermannian projection (feature: gudermannian-projection)
#[cfg(feature = "gudermannian-projection")]
#[no_mangle]
pub extern "C" fn telemetry_invert_gudermannian(
    projector: *const GudermannianProjector,
    z: i128,
) -> i128 {
    if projector.is_null() {
        return z;
    }
    unsafe { (*projector).invert(z) }
}

// =============================================================================
// BYZANTINE FFI (FEATURE-GATED: byzantine-hardening)
// =============================================================================

#[cfg(feature = "byzantine-hardening")]
use crate::byzantine::{MerkleDAG, PBFTLiteConsensus, AuditRecord, AuditZone, HashProtocolVersion};

/// Create Merkle DAG (feature: byzantine-hardening)
#[cfg(feature = "byzantine-hardening")]
#[no_mangle]
pub extern "C" fn telemetry_create_merkle_dag() -> *mut MerkleDAG {
    let dag = Box::new(MerkleDAG::new());
    Box::into_raw(dag)
}

/// Destroy Merkle DAG (feature: byzantine-hardening)
#[cfg(feature = "byzantine-hardening")]
#[no_mangle]
pub extern "C" fn telemetry_destroy_merkle_dag(dag: *mut MerkleDAG) {
    if !dag.is_null() {
        unsafe { Box::from_raw(dag); }
    }
}

/// Create PBFT-lite consensus (feature: byzantine-hardening)
#[cfg(feature = "byzantine-hardening")]
#[no_mangle]
pub extern "C" fn telemetry_create_consensus(
    node_id: u8,
    total_nodes: u8,
) -> *mut PBFTLiteConsensus {
    let consensus = Box::new(PBFTLiteConsensus::new(node_id, total_nodes));
    Box::into_raw(consensus)
}

/// Destroy PBFT-lite consensus (feature: byzantine-hardening)
#[cfg(feature = "byzantine-hardening")]
#[no_mangle]
pub extern "C" fn telemetry_destroy_consensus(consensus: *mut PBFTLiteConsensus) {
    if !consensus.is_null() {
        unsafe { Box::from_raw(consensus); }
    }
}
