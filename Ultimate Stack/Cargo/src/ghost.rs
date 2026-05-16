//! ============================================================
//! DVSM-π+++ / DQSDv2 · ghost.rs
//! Ghost-Space Classification Layer
//! ============================================================
//
// PURPOSE:
// - classify spectral state
// - stabilize temporal interpretation
// - provide deterministic symbolic state
//
// ============================================================

use crate::{
    CoreState,

    DVSM_NOMINAL,
    DVSM_COLLAPSE,
    DVSM_DIFFUSE,
    DVSM_ECHO,
    DVSM_BURST,
    DVSM_TRAP,
    DVSM_VACUUM,
};

use crate::math::norm2;

// ============================================================
// CLASSIFICATION THRESHOLDS
// ============================================================

pub const COLLAPSE_EPS: f32 = 1e-6;
pub const DIFFUSE_THRESHOLD: f32 = 0.25;
pub const ECHO_THRESHOLD: f32 = 5.0;
pub const TRAP_THRESHOLD: f32 = 10.0;
pub const BURST_THRESHOLD: f32 = 100.0;

// ============================================================
// CLASSIFY GHOST SPACE
// ============================================================
//
// ORDER MATTERS.
//
// ============================================================

#[inline(always)]
pub fn classify(
    state: &mut CoreState
) -> u8 {

    let r = state.params.r as usize;

    let energy =
        norm2(&state.Z, r);

    let drift =
        norm2(&state.Omega, r);

    // --------------------------------------------------------
    // INVALID / NUMERIC FAILURE
    // --------------------------------------------------------

    if !energy.is_finite()
    || !drift.is_finite() {

        state.ghost = DVSM_BURST;
        return state.ghost;
    }

    // --------------------------------------------------------
    // VACUUM
    // --------------------------------------------------------

    if state.fail_counter > 0 {

        state.ghost = DVSM_VACUUM;
        return state.ghost;
    }

    // --------------------------------------------------------
    // COLLAPSE
    // --------------------------------------------------------

    if energy < COLLAPSE_EPS {

        state.ghost = DVSM_COLLAPSE;
        return state.ghost;
    }

    // --------------------------------------------------------
    // BURST
    // --------------------------------------------------------

    if energy > BURST_THRESHOLD {

        state.ghost = DVSM_BURST;
        return state.ghost;
    }

    // --------------------------------------------------------
    // TRAP
    // --------------------------------------------------------

    if drift > TRAP_THRESHOLD {

        state.ghost = DVSM_TRAP;
        return state.ghost;
    }

    // --------------------------------------------------------
    // ECHO
    // --------------------------------------------------------

    if drift > ECHO_THRESHOLD {

        state.ghost = DVSM_ECHO;
        return state.ghost;
    }

    // --------------------------------------------------------
    // DIFFUSE
    // --------------------------------------------------------

    if energy < DIFFUSE_THRESHOLD {

        state.ghost = DVSM_DIFFUSE;
        return state.ghost;
    }

    // --------------------------------------------------------
    // NOMINAL
    // --------------------------------------------------------

    state.ghost = DVSM_NOMINAL;

    state.ghost
}
