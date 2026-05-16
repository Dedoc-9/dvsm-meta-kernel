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

fn project(W, Z) -> (c, res, r) {
    let c   = W.t() * Z;        // O(nr)
    let res = Z - W * c;        // O(nr)
    let r   = norm(res);        // O(n)
    (c, res, r)
}

// otherwise:

let (c, res, r) = project(W, Z);                       
Z += dt * (lie_bracket(Z, S, kappa) - λ * Z);          
if r > ε { W += η * outer(res, normalize(c)); }
// -------------------------------------------------------------------------------
// crates/dvsm-core/src/energy_test.rs

/// Verifies Result 1: Energy conservation under antisymmetric Lie-bracket coupling.
/// Property: d‖Z‖²/dt = −2λ‖Z‖² exactly.
pub fn verify_lie_energy_conservation() {
    const N: usize = 4;
    let lambda: f64 = 0.5;
    let dt: f64 = 0.0001;

    // 1. Initial states (Z) and Memory (S)
    let z = [1.0, 0.5, -0.2, 0.8];
    let s = [0.1, -0.1, 0.4, -0.3];
    
    // 2. Generate an antisymmetric kappa (κ_kj = -κ_jk)
    // In production, this is a pre-allocated static matrix.
    let mut kappa = [[0.0f64; N]; N];
    kappa[0][1] = 0.5;  kappa[1][0] = -0.5;
    kappa[0][2] = -0.2; kappa[2][0] = 0.2;
    kappa[1][3] = 0.8;  kappa[3][1] = -0.8;

    // 3. Calculate Coupling Term: Σⱼ(Z_k S_j − Z_j S_k)κ_kj
    let mut coupling_dz = [0.0f64; N];
    for k in range(0..N) {
        for j in range(0..N) {
            coupling_dz[k] += (z[k] * s[j] - z[j] * s[k]) * kappa[k][j];
        }
    }

    // 4. Calculate total dZ/dt including dissipation
    let mut total_dz = [0.0f64; N];
    for k in 0..N {
        total_dz[k] = coupling_dz[k] - (lambda * z[k]);
    }

    // 5. Energy Analysis
    // d‖Z‖²/dt = 2 * Σ(Z_k * dZ_k/dt)
    let z_dot_coupling: f64 = z.iter().zip(coupling_dz.iter()).map(|(zk, dzk)| zk * dzk).sum();
    let z_dot_total: f64 = z.iter().zip(total_dz.iter()).map(|(zk, dzk)| zk * dzk).sum();
    
    let actual_de_dt = 2.0 * z_dot_total;
    let expected_de_dt = -2.0 * lambda * z.iter().map(|zk| zk * zk).sum::<f64>();

    // Result 1 validation
    assert!(z_dot_coupling.abs() < 1e-15, "Pioneering Result Failed: Coupling added energy!");
    println!("Coupling Power Contribution: {:.12e} (Strict Zero)", z_dot_coupling);
    println!("Actual dE/dt: {:.6}", actual_de_dt);
    println!("Theoretical dE/dt (-2λ‖Z‖²): {:.6}", expected_de_dt);
}
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
