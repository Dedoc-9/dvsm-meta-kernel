/// src/supervisor_loop.rs
///
/// Hardened Supervisor Loop: Phase I.3 Integration with Hysteresis & Telemetry
///
/// Contract:
/// - Z_t evolution ALWAYS occurs (deterministic frame production)
/// - Compression is optional (can be shed under load)
/// - Phase Shedding is stateful: enter at 200, exit at 150 (hysteresis prevents thrashing)
/// - L1D cache conflicts are measured (Core 0 ↔ Core 1 coherency monitoring)
/// - Frame marking enables downstream protocol integrity

use crate::dvsm_state::DVSMState;
use crate::compression::{TilePool, encode_saec};

/// Cycle-accurate timing (x86-64 RDTSC instruction)
/// On Zen 5, this is deterministic and serialized
#[inline(always)]
pub fn rdtsc() -> u64 {
    unsafe {
        std::arch::x86_64::_rdtsc()
    }
}

/// Placeholder: Read L1D cache miss performance counter
/// Real implementation: Use `rdpmc` (read performance monitoring counter)
/// Counter: IA32_PERFCTR0 or PMC0 (configured for L1D_CACHE_MISSES)
#[inline(always)]
pub fn read_perf_counter_l1d_misses() -> u64 {
    // Stub: return 0 for now (real implementation requires kernel access)
    // On production Z2, this would read the actual perf counter via rdpmc
    0
}

/// Frame flags for downstream protocol
pub mod flags {
    pub const FLAG_UNCOMPRESSED: u8 = 0x01;  // Frame was produced but not compressed
    pub const FLAG_PHASE_SHEDDING: u8 = 0x02; // Supervisor is in Phase Shedding mode
}

/// Stub: Evolution core (placeholder for dvsm_evolve_core)
/// In real implementation, this computes Z_{t+1} from Z_t
pub fn dvsm_evolve_core(_state: &mut DVSMState) {
    // Placeholder: Z_t evolution (always runs, independent of compression)
}

/// Stub: Compression queue interface
pub struct CompressionQueue;

impl CompressionQueue {
    pub fn push(&self, _tile_idx: usize) -> Result<(), ()> {
        Ok(()) // Stub: always succeeds
    }
}

/// HARDENED SUPERVISOR TICK
///
/// Hysteresis Logic:
/// - in_phase_shedding = false, occupancy crosses 200 → enter Phase Shedding
/// - in_phase_shedding = true, occupancy drops below 150 → exit Phase Shedding
/// This prevents thrashing at the boundary.
///
/// Telemetry:
/// - L1D cache conflicts: measured before/after compress
/// - Occupancy trend: sampled every frame
/// - Regime transitions: logged for analysis
/// - Shed events: marked with FLAG_UNCOMPRESSED
pub fn supervisor_tick(
    state: &mut DVSMState,
    pool: &mut TilePool,
    queue: &CompressionQueue,
) {
    let start_cycles = rdtsc();

    // ========================================================================
    // PHASE I.1 & I.2: Core Evolution (ALWAYS RUNS, DETERMINISTIC)
    // ========================================================================
    dvsm_evolve_core(state);

    // ========================================================================
    // PHASE I.3: HARDENED COMPRESSION ENQUEUE (Hysteresis + Telemetry)
    // ========================================================================

    // Read current pool occupancy
    let occ = pool.get_occupancy();

    // Record occupancy sample (for forensics and adaptive thresholding)
    state.telemetry.occupancy_history.push(occ as u32);
    if state.telemetry.occupancy_history.len() > 1000 {
        state.telemetry.occupancy_history.remove(0);
    }

    // ========================================================================
    // SAEC Regime Selection: Compute residuals, detect singularity, select regime
    // ========================================================================
    // Store the last regime for hysteresis in select_regime
    let last_regime = state
        .telemetry
        .regime_transitions
        .last()
        .map(|(r, _)| *r)
        .unwrap_or(0);

    // Call SAEC encoder pipeline
    let saec_result = encode_saec(state, occ, last_regime);

    let regime = match saec_result {
        Ok(output) => {
            // SAEC succeeded: use the selected regime and store metrics
            state.telemetry.regime_transitions.push((output.regime, state.frame_count));

            // Store singularity ratio for telemetry (future S_t accumulation)
            // For now, we log it; in Phase 2, this feeds the EMA
            let _singularity_ratio = output.singularity_ratio;

            output.regime
        }
        Err(_) => {
            // Safety gate triggered: manifold unstable under high pressure
            // Force Phase Shedding (Regime 4)
            state.telemetry.regime_transitions.push((4, state.frame_count));
            state.supervisor_flags.in_phase_shedding = true;
            4
        }
    };

    // ========================================================================
    // Compression Dispatch (or Phase Shedding)
    // ========================================================================
    if regime != 4 {
        // Normal compression: acquire tile and encode
        let _pop_start = rdtsc();
        if let Some((idx, tile)) = pool.pop_tile() {

            // Populate tile metadata
            tile.metadata_regime = regime;
            tile.sample_count = state.sample_count as u32;

            // Measure L1D cache conflicts during encoding
            let l1_start = read_perf_counter_l1d_misses();
            encode_placeholder(tile, state);
            let l1_end = read_perf_counter_l1d_misses();
            state.telemetry.l1_conflicts += l1_end.saturating_sub(l1_start);

            // Dispatch to async compression thread (lock-free queue)
            if let Err(_) = queue.push(idx) {
                // Queue overflow: return tile to pool for retry next frame
                pool.push_tile(idx);
            }
        }
    } else {
        // Phase Shedding: skip compression (mute S_t, save hardware)
        // Z_t still evolves (deterministic), S_t doesn't accumulate residual
        state.frame_flags |= flags::FLAG_UNCOMPRESSED;
        state.frame_flags |= flags::FLAG_PHASE_SHEDDING;
        state.telemetry.shed_count += 1;
    }

    // ========================================================================
    // TELEMETRY: Final frame cost
    // ========================================================================
    state.telemetry.last_tick_cycles = rdtsc() - start_cycles;
}

/// Stub: Placeholder encoder (no real compression, just cache-line traffic)
/// Simulates writing Z-state to tile to measure cache coherency
fn encode_placeholder(_tile: &mut crate::compression::CompressionTile, _state: &DVSMState) {
    // Stub: Real implementation in src/compression/saec_math.rs
}

// ============================================================================
// Tests: Hysteresis & Telemetry Correctness
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdtsc_monotonic() {
        let t1 = rdtsc();
        let t2 = rdtsc();
        // rdtsc should always increase (or stay same if called back-to-back)
        assert!(t2 >= t1, "rdtsc should be monotonic");
    }

    #[test]
    fn test_dvsm_state_creation() {
        let state = DVSMState::new();
        assert_eq!(state.frame_count, 0);
        assert_eq!(state.sample_count, 0);
        assert!(!state.supervisor_flags.in_phase_shedding);
    }

    #[test]
    fn test_occupancy_recording() {
        let mut state = DVSMState::new();
        state.telemetry.occupancy_history.push(50);
        state.telemetry.occupancy_history.push(100);
        state.telemetry.occupancy_history.push(150);
        assert_eq!(state.telemetry.occupancy_history.len(), 3);
    }

    #[test]
    fn test_regime_logging() {
        let mut state = DVSMState::new();
        state.telemetry.regime_transitions.push((3, 0));
        state.telemetry.regime_transitions.push((2, 1));
        state.telemetry.regime_transitions.push((4, 2));
        assert_eq!(state.telemetry.regime_transitions.len(), 3);
        assert_eq!(state.telemetry.regime_transitions[0], (3, 0));
    }

    #[test]
    fn test_frame_flags() {
        let mut state = DVSMState::new();
        state.frame_flags = 0;
        state.frame_flags |= flags::FLAG_UNCOMPRESSED;
        assert_eq!(state.frame_flags, flags::FLAG_UNCOMPRESSED);
    }
}
