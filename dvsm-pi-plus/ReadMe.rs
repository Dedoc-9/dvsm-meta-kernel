// Author: Daniel J. Dillberg
// Contact: BigDilly95@@gmail.com
// ------------------------------------------------------------------------------

// STEP  EQUATION / OPERATION                              MODULE          O(·)
// ──────────────────────────────────────────────────────────────────────────────
// 1.    ‖Z‖²>U²_MAX for K frames → kill; mode←f(H,‖S‖)  containment     O(r)
//       Z,S,V,Ω ← rebirth(W, mode)                       containment     O(nr)

// 2.    c = WᵀZ; p = Wc; R = Z−p; r_norm = ‖R‖          pipeline        O(nr)

// 3.    Z += dt·(Σⱼ(ZₖSⱼ−ZⱼSₖ)κₖⱼ − λZₖ)              pipeline        O(r²)

// 4.    if !frozen: S = αS + (1−α)Z                       pipeline        O(r)

// 5.    if r_norm>ε: W += η·R⊗(c/‖c‖)                    pipeline        O(nr)

// 6.    if drift>1e-6: MGS(W); sign_lock(W, W_prev)       manifold        O(nr²)

// 7.    V = clamp(V·γ + (R+S)·η, ±U_MAX); X += V·dt      pipeline        O(n)

// 8.    Ω = (Ω + Z·α·dt)·decay                            pipeline        O(r)

// 9.    ghost = f(B, ν, δ, H, ‖Ω‖/‖Z‖)                   ghost           O(1)

// 10.   W_prev ← W; frame += 1; t += dt                   state_commit    O(nr)

// 11.   emit if |Δν| > ε                                   trace           O(1)

let c = W.t() * Z;                                    // O(nr)
let res = Z - W * c;                                   // O(nr)
let r = norm(res);                                     // O(n)
Z += dt * (lie_bracket(Z, S, kappa) - λ * Z);          // O(r²)
if r > ε { W += η * outer(res, c / (norm(c) + ε)); }  // O(nr)

// otherwise:

let (c, res, r) = project(W, Z);                       // c=WᵀZ, res=Z-Wc, r=‖res‖
Z += dt * (lie_bracket(Z, S, kappa) - λ * Z);
if r > ε { W += η * outer(res, c / (norm(c) + ε)); }

// -------------------------------------------------------------------------------

//! DVSM-π+++ corrected hot-path step (no heap, correct Lie dimension, full telemetry)

use crate::{
    containment,
    manifold,
    ghost,
    trace,
    math,
    consts::{U_MAX, EPS, RMAX},
};

/// Flat kappa: [r * r]
#[inline(always)]
fn kappa_index(r: usize, k: usize, j: usize) -> usize {
    k * r + j
}

#[inline(always)]
pub fn step(
    state: &mut State,
    input: &[f32],
    dt: f32,
    alpha: f32,
    eta: f32,
    gamma: f32,
    decay: f32,
) -> TraceFrame {

    let r = state.r as usize;
    let n = state.n as usize;

    // =========================================================
    // 1. CONTAINMENT
    // =========================================================
    containment::kill_check(state, r);

    if containment::needs_rebirth(state) {
        containment::rebirth(state);
    }

    // =========================================================
    // STACK BUFFERS (NO HEAP)
    // =========================================================
    let mut c: [f32; RMAX] = [0.0; RMAX];
    let mut residual: [f32; RMAX] = [0.0; RMAX];

    // =========================================================
    // 2. PROJECTION
    // =========================================================
    for k in 0..r {
        c[k] = math::dot(&state.W[k * n..], &state.Z, n);
    }

    let mut r_norm = 0.0;

    for i in 0..n {
        let mut acc = 0.0;

        for k in 0..r {
            acc += state.W[k * n + i] * c[k];
        }

        residual[i] = state.Z[i] - acc;
        r_norm += residual[i] * residual[i];
    }
    r_norm = r_norm.sqrt();

    // =========================================================
    // 3. LIE EVOLUTION (CORRECT r×r DOMAIN)
    // =========================================================
    for k in 0..r {
        let mut sum = 0.0;

        for j in 0..r {
            let idx_kj = kappa_index(r, k, j);

            let skew =
                state.Z[k] * state.S[j]
                - state.Z[j] * state.S[k];

            sum += skew * state.kappa[idx_kj];
        }

        state.Z[k] += dt * (sum - state.lambda * state.Z[k]);
    }

    // =========================================================
    // 4. EMA MEMORY
    // =========================================================
    if !containment::frozen(state) {
        for i in 0..r {
            state.S[i] = alpha * state.S[i] + (1.0 - alpha) * state.Z[i];
        }
    }

    // =========================================================
    // 5. BASIS ADAPTATION
    // =========================================================
    if r_norm > EPS {
        let c_norm = math::norm_safe(&c, r).max(1e-8);

        for k in 0..r {
            let scale = c[k] / c_norm;

            for i in 0..n {
                state.W[k * n + i] += eta * residual[i] * scale;
            }
        }
    }

    // =========================================================
    // 6. MANIFOLD DRIFT (NOW COMPUTED)
    // =========================================================
    let drift = manifold::stiefel_drift(&state.W, r, n);
    state.drift = drift;

    if drift > 1e-6 {
        manifold::modified_gram_schmidt(&mut state.W, r, n);
        manifold::sign_lock(&mut state.W, &state.W_prev, r, n);
    }

    // =========================================================
    // 7. VELOCITY
    // =========================================================
    for i in 0..n {
        state.V[i] =
            state.V[i] * gamma
            + (residual[i] + state.S.get(i % r).copied().unwrap_or(0.0)) * eta;

        state.V[i] = state.V[i].clamp(-U_MAX, U_MAX);
        state.X[i] += state.V[i] * dt;
    }

    // =========================================================
    // 8. OMEGA
    // =========================================================
    for i in 0..r {
        state.Omega[i] =
            (state.Omega[i] + state.Z[i] * alpha * dt) * decay;
    }

    // =========================================================
    // 9. DIAGNOSTICS (FIXED: NO STALE FIELDS)
    // =========================================================
    state.novelty = r_norm / (math::norm_safe(input, n) + 1e-8);
    state.B = math::norm_safe(&state.S, r) / (math::norm_safe(&state.Z, r) + 1e-8);
    state.entropy = math::spectral_entropy(&state.Z, r, state.frames_since_rebirth);

    // =========================================================
    // 10. GHOST CLASSIFICATION
    // =========================================================
    state.ghost = ghost::classify(
        &state.BasisB,
        state.novelty,
        state.delta,
        state.entropy,
        state.B,
    );

    // =========================================================
    // 11. COMMIT
    // =========================================================
    state.W_prev.copy_from_slice(&state.W);
    state.frame += 1;
    state.t += dt;

    // =========================================================
    // 12. TRACE (ALWAYS RETURN FULL FRAME)
    // =========================================================
    trace::emit(
        state,
        r_norm,
        true, // emitted flag
    )
}
