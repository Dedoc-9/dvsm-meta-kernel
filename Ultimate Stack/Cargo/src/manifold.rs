// src/manifold.rs
//! ============================================================
//! DVSM-π+++ / DQSDv2 Stiefel Manifold Maintenance Layer
//! Authorr: Daniel J. Dillberg
//! Contact: BigDilly95@gmail.com
//! ============================================================
// ============================================================
// DVSM-π+++ / DQSDv2 · MANIFOLD LAYER
// ============================================================
//! PURPOSE:
//! - Enforce Stiefel manifold constraint WᵀW ≈ I
//! - Provide deterministic retraction utilities
//! - Prevent phase drift in long UE5 runtime sessions
//! ============================================================

use crate::{CoreState, RMAX};

// ============================================================
// EPS SAFETY
// ============================================================

const EPS: f32 = 1e-6;

// ============================================================
// DOT PRODUCT (local fast path)
// ============================================================

#[inline(always)]
fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
    let mut s = 0.0;
    for i in 0..n {
        s += a[i] * b[i];
    }
    s
}

// ============================================================
// MODIFIED GRAM-SCHMIDT RETRACTION
// ============================================================

#[inline(always)]
pub fn retract_stiefel(state: &mut CoreState) {
    let r = state.params.r as usize;

    for k in 0..r {
        let bk = k * RMAX;

        // ---- orthogonalize ----
        for j in 0..k {
            let bj = j * RMAX;

            let mut proj = 0.0;

            for i in 0..r {
                proj += state.W[bk + i] * state.W[bj + i];
            }

            for i in 0..r {
                state.W[bk + i] -= proj * state.W[bj + i];
            }
        }

        // ---- normalize ----
        let mut norm2 = 0.0;
        for i in 0..r {
            norm2 += state.W[bk + i] * state.W[bk + i];
        }

        let inv = 1.0 / norm2.sqrt().max(EPS);

        for i in 0..r {
            state.W[bk + i] *= inv;
        }
    }
}

// ============================================================
// PHASE SIGN LOCK
// ============================================================
// Prevents 180° basis flips that break:
// - temporal coherence (UE5/DLSS)
// - motion vector consistency
// ============================================================

#[inline(always)]
pub fn sign_lock(state: &mut CoreState) {
    let r = state.params.r as usize;

    for k in 0..r {
        let b = k * RMAX;

        let mut d = 0.0;

        for i in 0..r {
            d += state.W[b + i] * state.W_prev[b + i];
        }

        if d < 0.0 {
            for i in 0..r {
                state.W[b + i] *= -1.0;
            }
        }
    }

    state.W_prev.copy_from_slice(&state.W);
}

// ============================================================
// FROBENIUS ORTHOGONALITY ERROR
// ============================================================
// ||WᵀW - I||_F
// ============================================================

#[inline(always)]
pub fn ortho_error(state: &CoreState) -> f32 {
    let r = state.params.r as usize;

    let mut err = 0.0;

    for i in 0..r {
        for j in 0..r {
            let mut d = 0.0;

            for k in 0..r {
                d += state.W[i * RMAX + k] * state.W[j * RMAX + k];
            }

            let target = if i == j { 1.0 } else { 0.0 };
            let e = d - target;

            err += e * e;
        }
    }

    err.sqrt()
}

// ============================================================
// FULL MANIFOLD PIPELINE (PUBLIC ENTRY)
// ============================================================

#[inline(always)]
pub fn maintain_manifold(state: &mut CoreState) {
    if ortho_error(state) > 1e-6 {
        retract_stiefel(state);
        sign_lock(state);
    }
}
