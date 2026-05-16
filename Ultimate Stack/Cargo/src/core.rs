#![allow(non_snake_case)]

//! ============================================================
//! DVSM-π+++ / DQSDv2 · CORE ENGINE
//! Author: Daniel J. Dillberg
//! ============================================================
//! PURPOSE:
//! - Single-step deterministic evolution engine
//! - Orchestrates math + manifold + state machine
//! - No FFI, no allocation, no ownership logic
//! ============================================================

use crate::{
    CoreState,
    DVSM_TraceFrame,
    DVSM_NOMINAL,
    DVSM_COLLAPSE,
    DVSM_TRAP,
    DVSM_BURST,
    DVSM_ECHO,
    DVSM_VACUUM,
    RMAX,
};

use crate::math::{dot, norm2, norm_safe, ema_update, evolve_lie};
use crate::manifold::maintain_manifold;

// ============================================================
// CORE STEP (single deterministic tick)
// ============================================================

#[inline(always)]
pub fn core_step(
    state: &mut CoreState,
    input: &[f32],
    trace: &mut DVSM_TraceFrame,
) {
    let r = state.params.r as usize;

    // ========================================================
    // 1. INPUT PROJECTION (W-space encoding)
    // ========================================================

    let mut coeff = [0.0f32; RMAX];
    let mut proj  = [0.0f32; RMAX];
    let mut resid = [0.0f32; RMAX];

    for k in 0..r {
        coeff[k] = dot(&state.W[k * RMAX..], input, r);
    }

    for i in 0..r {
        for k in 0..r {
            proj[i] += state.W[k * RMAX + i] * coeff[k];
        }
        resid[i] = input[i] - proj[i];
    }

    let residual_energy = norm2(&resid, r);

    // ========================================================
    // 2. LIE-BRACKET EVOLUTION (Z dynamics)
    // ========================================================

    let prev_z = state.Z;

    for k in 0..r {
        let mut acc = 0.0f32;

        for j in 0..r {
            if j == k { continue; }

            // inline κ-like coupling (deterministic)
            let kappa =
                (k as f32 * 1.37 - j as f32 * 1.73).sin();

            acc += (
                prev_z[k] * state.S[j]
                - prev_z[j] * state.S[k]
            ) * kappa;
        }

        state.Z[k] += state.params.dt
            * (acc - state.params.lambda * state.Z[k]);
    }

    // ========================================================
    // 3. EMA MEMORY UPDATE
    // ========================================================

    if state.fail_counter == 0 {
        ema_update(
            &mut state.S,
            &state.Z,
            state.params.alpha,
            r,
        );
    }

    // ========================================================
    // 4. VELOCITY + OMEGA DRIFT
    // ========================================================

    let mut drift_acc = 0.0f32;

    for i in 0..r {
        state.V[i] =
            state.V[i] * 0.98
            + resid[i] * 0.01;

        state.V[i] = state.V[i]
            .clamp(
                -state.params.u_max,
                 state.params.u_max
            );

        state.Omega[i] =
            (state.Omega[i]
                + state.Z[i]
                * state.params.alpha
                * state.params.dt) * 0.9995;

        drift_acc += state.Omega[i] * state.Omega[i];
    }

    // ========================================================
    // 5. BASIS UPDATE (learning step)
    // ========================================================

    let coeff_norm = norm_safe(&coeff, r);

    if residual_energy > 1e-8 {
        for k in 0..r {
            let scale = coeff[k] / coeff_norm;

            for i in 0..r {
                state.W[k * RMAX + i] +=
                    0.01 * resid[i] * scale;
            }
        }
    }

    // ========================================================
    // 6. MANIFOLD CORRECTION (Stiefel enforcement)
    // ========================================================

    maintain_manifold(state);

    // ========================================================
    // 7. STATE CLASSIFICATION (Ghost logic)
    // ========================================================

    let energy = norm2(&state.Z, r);

    state.ghost =
        if !energy.is_finite() {
            DVSM_BURST
        } else if energy < 1e-6 {
            DVSM_COLLAPSE
        } else if drift_acc > 10.0 {
            DVSM_TRAP
        } else if drift_acc > 5.0 {
            DVSM_ECHO
        } else {
            DVSM_NOMINAL
        };

    // ========================================================
    // 8. TRACE OUTPUT (ABI-safe snapshot)
    // ========================================================

    trace.frame = state.frame;
    trace.stress = norm_safe(&state.S, r);
    trace.novelty = residual_energy.sqrt();
    trace.drift = drift_acc.sqrt().max(0.0);
    trace.entropy = energy.ln().max(0.0);
    trace.energy = energy;
    trace.ghost = state.ghost;
    trace.contained = state.contained as u8;

    // ========================================================
    // 9. FRAME ADVANCE
    // ========================================================

    state.frame += 1;
}
