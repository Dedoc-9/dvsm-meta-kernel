/// src/dvsm_state.rs
///
/// Core DVSMState: Lean & Mean for Compression-Only Plumbing (Phase I.3)
///
/// This is the "glue" that binds:
/// - Supervisor loop (producer of Z_t, consumer of telemetry)
/// - TilePool (consumer of occupancy, producer of frame metadata)
/// - SAEC encoder (consumer of z_manifold & w_basis, producer of G_t)
///
/// **Alignment Guarantee:** 64-byte alignment prevents false-sharing between
/// Core 0 (supervisor) and Core 1 (compression thread) on Zen 5.
///
/// **Manifold Size:** Fixed at 269D (matches DVSM v3.3 spec, Q31.32 source)
///
/// **Phase I.4 (GhostSnap):** Deferred to Phase 2. No ghostsnap fields here.

use std::default::Default;
use crate::rf_elf::RfElfSample;

/// Supervisor control flags (stateful hysteresis)
#[derive(Debug, Clone, Copy, Default)]
pub struct SupervisorFlags {
    /// True if occupancy > 200 (Phase Shedding enabled)
    pub in_phase_shedding: bool,

    /// Optional regime override (e.g., for testing or emergency mode)
    pub regime_override: Option<u8>,
}

/// Compression telemetry: forensic data for Zen 5 hardware validation
#[derive(Debug, Clone)]
pub struct CompressionTelemetry {
    /// Count of frames shed (occupancy exceeded 200)
    pub shed_count: u64,

    /// L1D cache conflicts during compression phase (cycles)
    /// Incremented by supervisor_loop when encode_placeholder detects misses
    pub l1_conflicts: u64,

    /// Cycle cost of last supervisor tick (rdtsc measurement)
    pub last_tick_cycles: u64,

    /// Circular buffer of occupancy samples (last 1000 frames)
    pub occupancy_history: Vec<u32>,

    /// Regime transitions log: (regime_id, frame_count)
    /// Enables forensic replay: which frames used which regime?
    pub regime_transitions: Vec<(u8, u64)>,

    // ====================================================================
    // RF/ELF MODALITY TELEMETRY (Phase I.0.5)
    // ====================================================================
    /// Count of RF/ELF samples detected as stale (age > 8333 μs)
    pub rf_elf_stale_count: u64,

    /// Count of frames where no RF/ELF sample was available
    pub rf_elf_empty_frames: u64,

    /// Count of RF/ELF buffer overflow events (producer too fast)
    pub rf_elf_overflow_count: u64,

    // ====================================================================
    // MOLECULAR MODALITY TELEMETRY (Phase V3.4)
    // ====================================================================
    /// Count of successful torsion array injections
    pub molecular_injections: u64,

    /// Count of CRC32 checksum mismatches (informational, non-fatal)
    pub checksum_warnings: u64,

    // ====================================================================
    // L5 ALLOSTERIC KERNEL TELEMETRY (Phase V3.4)
    // ====================================================================
    /// Cumulative cycles spent in L5 spectral resonance computation
    pub l5_resonance_cycles: u64,

    /// Count of successful allosteric coupling applications
    pub allosteric_activations: u64,

    /// Count of frames with strong activation (α > 0.5)
    pub strong_activation_frames: u64,
}

impl Default for CompressionTelemetry {
    fn default() -> Self {
        CompressionTelemetry {
            shed_count: 0,
            l1_conflicts: 0,
            last_tick_cycles: 0,
            occupancy_history: Vec::with_capacity(1000),
            regime_transitions: Vec::with_capacity(32),
            rf_elf_stale_count: 0,
            rf_elf_empty_frames: 0,
            rf_elf_overflow_count: 0,
            molecular_injections: 0,
            checksum_warnings: 0,
            l5_resonance_cycles: 0,
            allosteric_activations: 0,
            strong_activation_frames: 0,
        }
    }
}

/// Core Deterministic State Evolution Manifold (DVSM)
///
/// **Layout:** 64-byte aligned to prevent false-sharing on Zen 5.
/// All fields are strictly ordered for cache-line efficiency.
///
/// **Determinism:** All floating-point values are Q31.32 fixed-point
/// (stored as f32 for ergonomics, but quantized to integers in SAEC).
///
/// **Session-Immutable Contract:**
/// - `w_basis` is set at session_init and remains constant
/// - `z_manifold` evolves frame-to-frame (produced by dvsm_evolve_core)
/// - `supervisor_flags` change only at frame boundaries (hysteresis state)
/// - `telemetry` accumulates across the session
#[repr(C, align(64))]
pub struct DVSMState {
    // ====================================================================
    // PRIMARY MANIFOLD (Z_t)
    // ====================================================================
    /// 269-dimensional state vector on the primary manifold
    /// Source for residual computation: G_t = Z_t - Π_W(Z_t)
    /// Constraint: All values represent Q31.32 fixed-point (stored as f32)
    pub z_manifold: [f32; 269],

    // ====================================================================
    // METADATA
    // ====================================================================
    /// Sample count for this frame (used in tile metadata)
    pub sample_count: u32,

    /// Monotonically increasing frame counter (used in regime_log timestamps)
    pub frame_count: u64,

    /// Frame flags: FLAG_UNCOMPRESSED, FLAG_PHASE_SHEDDING, etc.
    pub frame_flags: u8,

    /// Current frame's timestamp in microseconds (for RF/ELF stale detection)
    /// Updated at the start of each supervisor tick
    pub current_timestamp_us: u64,

    // ====================================================================
    // CONTROL & MONITORING
    // ====================================================================
    /// Hysteresis state for Phase Shedding (occupancy > 200 / < 150)
    pub supervisor_flags: SupervisorFlags,

    /// Forensic telemetry: L1D conflicts, occupancy, regime log
    /// Accumulated throughout the session for post-run analysis
    pub telemetry: CompressionTelemetry,

    // ====================================================================
    // ORTHOGONAL PROJECTION BASIS (W)
    // ====================================================================
    /// Whitened basis: top 8 principal components
    /// Used by SAEC for orthogonal projection: Π_W(Z) = sum_k (Z·W_k) W_k
    /// Set at session_init, immutable throughout session lifetime
    /// Each column W_k is normalized (||W_k|| = 1)
    pub w_basis: [[f32; 269]; 8],

    // ====================================================================
    // RF/ELF EXTERNAL MODALITY (Phase I.0.5)
    // ====================================================================
    /// Last valid RF/ELF sample from external producer (via try_pop)
    /// Used for Z_t coupling in Phase 2 (currently stored, not yet used)
    pub rf_elf_sample: RfElfSample,

    /// Whether the last RF/ELF sample is valid and fresh (age <= MAX_STALE_US)
    pub rf_elf_valid: bool,

    // ====================================================================
    // MOLECULAR MODALITY (Phase V3.4)
    // ====================================================================
    /// Whether the last torsion array injection was successful
    pub molecular_coordinates_valid: bool,

    /// Timestamp of the last successful torsion array injection (microseconds)
    pub molecular_timestamp_us: u64,

    // ====================================================================
    // L5 ALLOSTERIC STATE (Phase V3.4)
    // ====================================================================
    /// Current allosteric coefficient (α)
    /// Range: [0, 1], computed from ΔG scaling
    pub alpha_allosteric: f32,
}

impl DVSMState {
    /// Construct a new DVSMState with all fields zeroed
    /// Called at session_init
    pub fn new() -> Self {
        DVSMState {
            z_manifold: [0.0; 269],
            sample_count: 0,
            frame_count: 0,
            frame_flags: 0,
            current_timestamp_us: 0,
            supervisor_flags: SupervisorFlags::default(),
            telemetry: CompressionTelemetry::default(),
            w_basis: [[0.0; 269]; 8],
            rf_elf_sample: RfElfSample::new(),
            rf_elf_valid: false,
            molecular_coordinates_valid: false,
            molecular_timestamp_us: 0,
            alpha_allosteric: 0.0,
        }
    }

    /// Advance frame counter (called at start of each supervisor tick)
    pub fn advance_frame(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
    }

    /// Clear frame flags for the next frame
    pub fn clear_frame_flags(&mut self) {
        self.frame_flags = 0;
    }
}

impl Default for DVSMState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// MEMORY LAYOUT VALIDATION (compile-time assertions)
// ============================================================================

#[cfg(test)]
mod layout_tests {
    use super::*;
    use std::mem::{size_of, align_of};

    #[test]
    fn test_dvsm_state_alignment() {
        // DVSMState must be 64-byte aligned for Zen 5 cache-line coherency
        assert_eq!(align_of::<DVSMState>(), 64, "DVSMState must be 64-byte aligned");
    }

    #[test]
    fn test_dvsm_state_size() {
        // Sanity check: size should be reasonable (not bloated)
        let size = size_of::<DVSMState>();
        // 269*4 + 8 (count/frame) + 8 (flags/padding) + 64 (supervisor_flags + tel vec header)
        // + 8*269*4 (w_basis) = ~10KB (rough estimate)
        assert!(size < 16384, "DVSMState size {} seems excessive", size);
    }

    #[test]
    fn test_z_manifold_offset() {
        // z_manifold should be at the start of DVSMState (offset 0)
        let state = DVSMState::new();
        let state_ptr = &state as *const DVSMState as usize;
        let z_ptr = &state.z_manifold as *const [f32; 269] as usize;
        assert_eq!(z_ptr - state_ptr, 0, "z_manifold should be first field");
    }

    #[test]
    fn test_w_basis_accessible() {
        let mut state = DVSMState::new();
        state.w_basis[0][0] = 1.0;
        assert_eq!(state.w_basis[0][0], 1.0, "w_basis should be writable");
    }
}

// ============================================================================
// INTEGRATION TESTS
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_state_advance() {
        let mut state = DVSMState::new();
        assert_eq!(state.frame_count, 0);
        state.advance_frame();
        assert_eq!(state.frame_count, 1);
    }

    #[test]
    fn test_flag_management() {
        let mut state = DVSMState::new();
        state.frame_flags = 0x03;
        assert_eq!(state.frame_flags, 0x03);
        state.clear_frame_flags();
        assert_eq!(state.frame_flags, 0);
    }

    #[test]
    fn test_telemetry_accumulation() {
        let mut state = DVSMState::new();
        state.telemetry.shed_count = 5;
        state.telemetry.l1_conflicts = 1000;
        assert_eq!(state.telemetry.shed_count, 5);
        assert_eq!(state.telemetry.l1_conflicts, 1000);
    }

    #[test]
    fn test_occupancy_history() {
        let mut state = DVSMState::new();
        state.telemetry.occupancy_history.push(50);
        state.telemetry.occupancy_history.push(100);
        state.telemetry.occupancy_history.push(150);
        assert_eq!(state.telemetry.occupancy_history.len(), 3);
        assert_eq!(state.telemetry.occupancy_history[0], 50);
    }
}
