// src/manifold.rs
//! ============================================================
//! DVSM-π+++ / DQSDv2
//! Stiefel Manifold Maintenance Layer
//!
//! PURPOSE:
//! - Enforce WᵀW = I
//! - Prevent phase flips (DLSS/TAA stability)
//! - Maintain geometric consistency
//! Authorr: Daniel J. Dillberg
//! Contact: BigDilly95@gmail.com
//! ============================================================

use crate::{CoreState, RMAX};
use crate::math::{dot, norm, EPS};

// ============================================================
// MODIFIED GRAM-SCHMIDT (MGS) RETRACTION
// ============================================================

#[inline(always)]
pub fn retract_stiefel(state: &mut CoreState) {
let r = state.params.r as usize;

```
// --------------------------------------------------------
// Modified Gram-Schmidt
// --------------------------------------------------------

for k in 0..r {
    let bk = k * RMAX;

    // Orthogonalize against all previous vectors
    for j in 0..k {
        let bj = j * RMAX;

        let mut d = 0.0f32;

        for i in 0..r {
            d +=
                state.W[bk + i]
                * state.W[bj + i];
        }

        for i in 0..r {
            state.W[bk + i] -=
                d * state.W[bj + i];
        }
    }

    // Normalize vector
    let n =
        norm(
            &state.W[bk..bk + r],
            r
        );

    let inv = 1.0 / n.max(EPS);

    for i in 0..r {
        state.W[bk + i] *= inv;
    }
}
```

}

// ============================================================
// PHASE SIGN-LOCK (H5)
//
// Prevents 180° basis flips that destabilize:
// - DLSS
// - TAA
// - Nanite displacement
// - temporal reprojection
// ============================================================

#[inline(always)]
pub fn sign_lock(state: &mut CoreState) {
let r = state.params.r as usize;

```
for k in 0..r {
    let b = k * RMAX;

    let mut d = 0.0f32;

    for i in 0..r {
        d +=
            state.W[b + i]
            * state.W_prev[b + i];
    }

    // Flip back into prior orientation
    if d < 0.0 {
        for i in 0..r {
            state.W[b + i] *= -1.0;
        }
    }
}

// Commit stabilized basis
state.W_prev.copy_from_slice(&state.W);
```

}

// ============================================================
// FROBENIUS ORTHOGONALITY ERROR
//
// ||WᵀW - I||_F
//
// Used to gate expensive manifold retraction.
// ============================================================

#[inline(always)]
pub fn ortho_error(state: &CoreState) -> f32 {
let r = state.params.r as usize;

```
let mut err_sq = 0.0f32;

for i in 0..r {
    for j in 0..r {

        let wi =
            &state.W[i * RMAX..];

        let wj =
            &state.W[j * RMAX..];

        let d =
            dot(wi, wj, r);

        let target =
            if i == j {
                1.0f32
            } else {
                0.0f32
            };

        let e = d - target;

        err_sq += e * e;
    }
}

err_sq.sqrt()
```

}

// ============================================================
// FULL MANIFOLD MAINTENANCE PIPELINE
// ============================================================

#[inline(always)]
pub fn maintain_manifold(state: &mut CoreState) {

```
// Drift-gated execution
if ortho_error(state) > 1e-6 {

    // Stage 1:
    // Retract onto Stiefel manifold
    retract_stiefel(state);

    // Stage 2:
    // Restore temporal phase continuity
    sign_lock(state);
}

}
