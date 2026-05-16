//! ============================================================
//! DVSM-π+++ / DQSDv2 · containment.rs
//! Deterministic Containment + Rebirth Layer
//! ============================================================
//
// PURPOSE:
// - H2 hysteresis enforcement
// - vacuum containment
// - deterministic rebirth
// - EMA freeze policy
// - soft-clamp stabilization
//
// ============================================================

use crate::{
    CoreState,
    DVSM_VACUUM,
    RMAX,
};

use crate::math::norm2;

// ============================================================
// CONSTANTS
// ============================================================

pub const KILL_THRESHOLD: u32 = 3;
pub const SOFT_CLAMP_RATIO: f32 = 0.98;

// ============================================================
// SOFT CLAMP
// ============================================================
//
// Prevents single-frame spikes from causing
// immediate hard vacuum collapse.
//
// ============================================================

#[inline(always)]
pub fn soft_clamp(state: &mut CoreState) {

    let r = state.params.r as usize;
    let max_sq = state.params.u_max * state.params.u_max;

    let z2 = norm2(&state.Z, r);

    if z2 > max_sq {

        let scale =
            (max_sq / z2)
            .sqrt()
            * SOFT_CLAMP_RATIO;

        for i in 0..r {
            state.Z[i] *= scale;
        }
    }
}

// ============================================================
// FAILURE DETECTION
// ============================================================

#[inline(always)]
pub fn evaluate_instability(
    state: &mut CoreState
) -> bool {

    let r = state.params.r as usize;

    let z2 = norm2(&state.Z, r);

    let unstable =
        !z2.is_finite()
        || z2 > state.params.u_max * state.params.u_max;

    if unstable {
        state.fail_counter += 1;
    } else {
        state.fail_counter = 0;
    }

    unstable
}

// ============================================================
// EMA FREEZE CHECK
// ============================================================
//
// H4:
//
// During instability windows,
// EMA memory must NOT decay.
//
// ============================================================

#[inline(always)]
pub fn ema_frozen(
    state: &CoreState
) -> bool {

    state.fail_counter > 0
}

// ============================================================
// HARD VACUUM + REBIRTH
// ============================================================

#[inline(always)]
pub fn rebirth(
    state: &mut CoreState
) {

    state.Z = [0.0; RMAX];
    state.V = [0.0; RMAX];
    state.Omega = [0.0; RMAX];

    // preserve S memory intentionally
    // for temporal continuity

    state.fail_counter = 0;

    state.contained = true;
    state.ghost = DVSM_VACUUM;
}

// ============================================================
// CONTAINMENT PIPELINE
// ============================================================
//
// ORDER:
// 1. evaluate instability
// 2. soft clamp transient spikes
// 3. hard vacuum if threshold exceeded
//
// ============================================================

#[inline(always)]
pub fn containment_step(
    state: &mut CoreState
) {

    let unstable =
        evaluate_instability(state);

    if unstable {

        soft_clamp(state);

        if state.fail_counter >= KILL_THRESHOLD {
            rebirth(state);
        }
    }
}
