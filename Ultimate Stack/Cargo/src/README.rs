Author: Daniel J. Dillberg
Contact: BigDilly95@gmail.com
------------------------------------------------------------------------
The core.rs execution order becomes extremely clean:

RUNTIME PIPELINE (11 steps, canonical order)
════════════════════════════════════════════

1. containment        │ norm2(Z) > U_MAX² for K frames → kill → rebirth
2. projection         │ c = WᵀZ; p = Wc; res = Z - p; r_norm = ‖res‖
3. lie_evolution      │ Z[k] += dt·(Σⱼ(Z[k]S[j]-Z[j]S[k])·κ[k,j] - λZ[k])
4. ema_memory         │ if !frozen: S = αS + (1-α)Z
5. basis_adaptation   │ if r_norm > ε: W += η·res⊗(c/‖c‖)
6. manifold_maintain  │ if drift > 1e-6: orthonormalize(W); sign_lock(W, W_prev)
7. velocity_update    │ V = clamp(V·DAMP + (res+S)·η, ±U_MAX); X += V·dt
8. omega_drift        │ Ω = (Ω + Z·α·dt)·OMEGA_DECAY  [no Ω→V backfeed]
9. ghost_classify     │ classify(stress, novelty, drift, entropy, Ω_ratio)
10. frame_advance     │ frame += 1
11. trace_emit        │ → TraceFrame { frame,stress,novelty,drift,entropy,energy,ghost,contained }

// src/pipeline.rs
//! ============================================================
//! DVSM-π+++ / DQSDv2 · CANONICAL RUNTIME PIPELINE
//! ============================================================
//!
//! PURPOSE:
//! Deterministic execution ordering for:
//!   - containment
//!   - projection
//!   - Lie evolution
//!   - EMA memory
//!   - basis adaptation
//!   - manifold maintenance
//!   - velocity integration
//!   - Ω drift accumulation
//!   - ghost classification
//!   - trace emission
//!
//! DESIGN:
//!   - strict execution order
//!   - no allocations in hot loop
//!   - deterministic arithmetic
//!   - panic-free runtime
//!   - UE5/DLSS stable
//!
//! ============================================================

#![allow(non_snake_case)]

use crate::{
    constants::*,
    containment::*,
    ghost::*,
    manifold::*,
    math::*,
    trace::*,
    CoreState,
    DVSM_TraceFrame,
};

// ============================================================
// CANONICAL EXECUTION PIPELINE
// ============================================================

#[inline(always)]
pub fn execute_pipeline(
    state: &mut CoreState,
    input: &[f32],
    trace: &mut DVSM_TraceFrame,
) {
    let r = state.params.r as usize;

    // ========================================================
    // 1. CONTAINMENT
    // ========================================================

    containment_step(state, r);

    // ========================================================
    // 2. PROJECTION
    // ========================================================

    let mut coeff = [0.0f32; RMAX];
    let mut proj  = [0.0f32; RMAX];
    let mut res   = [0.0f32; RMAX];

    // c = Wᵀx
    for k in 0..r {
        coeff[k] = dot(
            &state.W[k * RMAX..],
            input,
            r,
        );
    }

    // p = Wc
    for i in 0..r {

        for k in 0..r {
            proj[i] +=
                state.W[k * RMAX + i]
                * coeff[k];
        }

        // residual
        res[i] = input[i] - proj[i];
    }

    let r_norm = norm_safe(&res, r);

    // ========================================================
    // 3. LIE EVOLUTION
    // ========================================================

    evolve_lie::<RMAX>(
        &mut state.Z,
        &state.S,
        &state.kappa,
        state.params.dt,
        state.params.lambda,
        r,
    );

    // ========================================================
    // 4. SUCHNESS DECAY
    // ========================================================

    let decay =
        (-state.params.lambda
        * state.params.dt)
        .exp();

    for i in 0..r {
        state.Z[i] *= decay;
    }

    // ========================================================
    // 5. EMA MEMORY
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
    // 6. BASIS ADAPTATION
    // ========================================================

    if r_norm > EPS {

        let c_norm =
            norm_safe(&coeff, r);

        for k in 0..r {

            let scale =
                coeff[k] / c_norm;

            for i in 0..r {

                state.W[k * RMAX + i] +=
                    BASIS_ETA
                    * res[i]
                    * scale;
            }
        }
    }

    // ========================================================
    // 7. MANIFOLD MAINTENANCE
    // ========================================================

    let drift =
        frobenius_ortho_error::<RMAX>(
            &state.W,
            r,
        );

    if drift > ORTHO_DRIFT_LIMIT {

        maintain_manifold(state);
    }

    // ========================================================
    // 8. VELOCITY UPDATE
    // ========================================================

    for i in 0..r {

        let drive =
            (res[i] + state.S[i])
            * BASIS_ETA;

        state.V[i] =
            state.V[i]
            * VELOCITY_DAMP
            + drive;

        state.V[i] =
            state.V[i]
            .clamp(
                -state.params.u_max,
                 state.params.u_max,
            );

        state.X[i] +=
            state.V[i]
            * state.params.dt;
    }

    // ========================================================
    // 9. Ω DRIFT
    // ========================================================

    let mut omega_energy = 0.0f32;

    for i in 0..r {

        state.Omega[i] =
            (
                state.Omega[i]
                + state.Z[i]
                * state.params.alpha
                * state.params.dt
            )
            * OMEGA_DECAY;

        omega_energy +=
            state.Omega[i]
            * state.Omega[i];
    }

    // ========================================================
    // 10. GHOST CLASSIFICATION
    // ========================================================

    let stress =
        norm_safe(&state.S, r);

    let novelty =
        r_norm;

    let entropy =
        norm2(&state.Z, r)
        .ln()
        .max(0.0);

    let energy =
        norm2(&state.Z, r);

    let ghost =
        classify_ghost(
            stress,
            novelty,
            drift,
            entropy,
            omega_energy,
        );

    state.ghost = ghost;

    // ========================================================
    // 11. FRAME ADVANCE
    // ========================================================

    state.frame += 1;

    // ========================================================
    // 12. TRACE EMISSION
    // ========================================================

    emit_trace(
        state,
        trace,
        stress,
        novelty,
        drift,
        entropy,
        energy,
    );
}
