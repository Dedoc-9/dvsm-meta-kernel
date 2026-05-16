//! ============================================================
//! DVSM-π+++ / DQSDv2 · trace.rs
//! Deterministic Trace Emission Layer
//! ============================================================
//
// PURPOSE:
// - ABI-stable telemetry generation
// - deterministic frame reporting
// - forensic replay instrumentation
//
// ============================================================

use crate::{
    CoreState,
    DVSM_TraceFrame,
};

use crate::math::{
    norm2,
    norm_safe,
};

// ============================================================
// TRACE BUILD
// ============================================================

#[inline(always)]
pub fn emit_trace(
    state: &CoreState,
    novelty: f32,
) -> DVSM_TraceFrame {

    let r = state.params.r as usize;

    let energy =
        norm2(&state.Z, r);

    let drift_sq =
        norm2(&state.Omega, r);

    let stress =
        norm_safe(&state.S, r);

    // --------------------------------------------------------
    // SAFE ENTROPY
    // --------------------------------------------------------

    let entropy =
        if energy > 1e-8 {
            energy.ln().max(0.0)
        } else {
            0.0
        };

    // --------------------------------------------------------
    // TRACE FRAME
    // --------------------------------------------------------

    DVSM_TraceFrame {

        frame: state.frame,

        stress,

        novelty,

        drift: drift_sq.sqrt(),

        entropy,

        energy,

        ghost: state.ghost,

        contained:
            state.contained as u8,
    }
}
