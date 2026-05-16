// src/core.rs
//
// ============================================================
// DVSM-π+++ / V17 Hardened Kernel
// Single-file deterministic execution pipeline
// ============================================================
//
// Author: Daniel J. Dillberg
// Contact: BigDilly95@gmail.com
//
// ORDERING INVARIANT:
// 1. Containment
// 2. Projection
// 3. Lie Evolution
// 4. EMA Memory
// 5. Basis Adaptation
// 6. RCE Folding (V17-E)
// 7. Manifold Maintenance
// 8. Velocity / Omega Drift
// 9. Ghost Classification
// 10. Acoustic Observation (V16)
// 11. Kinetic Probe (V17-K)  ← terminal diagnostic
// 12. Trace Emission
//
// ============================================================

use crate::{
    State,
    DVSM_TraceFrame,
    RMAX,
};

use crate::{containment, manifold, ghost, trace};
use crate::math::{norm_sq};

use crate::acoustic::{acoustic_observe, AcousticFrame};

// ============================================================
// V17-K: TERMINAL KINETIC PROBE (READ-ONLY OBSERVER)
// ============================================================
//
// PURPOSE:
// - Measure local Jacobian proxy of post-collapse manifold
// - Produce stiffness scalar without modifying state
// - Acts as Finsler metric estimator over latent dynamics
//
// INVARIANT:
// - NO mutation of state.z, state.w, state.omega
// - NO feedback into pipeline dynamics
//
// ============================================================

#[inline(always)]
fn kinetic_probe(state: &State, acoustic: &AcousticFrame) -> f32 {

    if acoustic.resonance_peak <= 1.0 {
        return 0.0;
    }

    let eps: f32 = 1e-3;

    // --------------------------------------------------------
    // Shadow perturbation (stack-local only)
    // --------------------------------------------------------
    let mut z_probe = state.z;

    for i in 0..RMAX {
        z_probe[i] += eps * (i as f32 * 0.1).cos();
    }

    let e_pre = norm_sq(&z_probe, RMAX);

    // --------------------------------------------------------
    // Local relaxation (infinitesimal decay operator)
    // --------------------------------------------------------
    for i in 0..RMAX {
        z_probe[i] *= 1.0 - (crate::LAMBDA * crate::DT);
    }

    let e_post = norm_sq(&z_probe, RMAX);

    // --------------------------------------------------------
    // stiffness = differential energy response
    // --------------------------------------------------------
    (e_pre - e_post).abs() / eps
}

// ============================================================
// CORE STEP (HARDENED ORDERING)
// ============================================================

pub fn step(state: &mut State) -> DVSM_TraceFrame {

    // 1. Containment
    containment::containment_step(state);

    // 2. Projection
    let (coeff, proj, residual) = crate::pipeline::project(state);

    // 3. Lie evolution
    crate::math::evolve_lie(state, &residual);

    // 4. EMA memory
    crate::math::ema_update(state);

    // 5. Basis adaptation
    crate::pipeline::adapt_basis(state, &coeff, &residual);

    // 6. RCE Folding (V17-E)
    crate::rce::resonant_fold(state);

    // 7. Manifold maintenance (Stiefel retraction)
    manifold::maintain_manifold(state);

    // 8. Velocity / Omega drift
    crate::math::update_drift(state, &residual);

    // 9. Ghost classification
    ghost::classify(state);

    // 10. Acoustic observation (V16 observer layer)
    let acoustic = acoustic_observe(state);

    // 11. V17-K: Kinetic stiffness probe (terminal diagnostic)
    let stiffness = kinetic_probe(state, &acoustic);

    // 12. Trace emission (observability closure)
    let frame = trace::emit_trace(state, acoustic, stiffness);

    state.frame += 1;

    frame
}
// crates/dvsm-core/src/arithmetic_api.rs
//
// ============================================================
// DVSM Arithmetic API Layer
// ============================================================
//
// PURPOSE:
// - Provide pure mathematical operators for DVSM kernel
// - Separate algebraic definitions from execution pipeline
// - Ensure deterministic, side-effect-free transformations
//
// DESIGN PRINCIPLE:
// "Arithmetic defines invariants. Core executes dynamics."
// ============================================================

use crate::{State, RMAX};

// ============================================================
// 1. INNER PRODUCT SPACE OPERATORS
// ============================================================

#[inline(always)]
pub fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
    let mut acc = 0.0;
    for i in 0..n {
        acc += a[i] * b[i];
    }
    acc
}

#[inline(always)]
pub fn norm_sq(x: &[f32], n: usize) -> f32 {
    dot(x, x, n)
}

// ============================================================
// 2. PROJECTION OPERATOR (WᵀZ SPACE MAPPING)
// ============================================================
//
// Z → coefficient space → reconstruction → residual
// ============================================================

pub fn project(
    state: &State,
    input: &[f32],
    coeff: &mut [f32; RMAX],
    proj: &mut [f32; RMAX],
    residual: &mut [f32; RMAX],
) {
    let r = state.params.r as usize;

    // spectral coefficients
    for k in 0..r {
        coeff[k] = dot(
            &state.w[k * RMAX..],
            input,
            r,
        );
    }

    // reconstruction + residual
    for i in 0..r {
        let mut acc = 0.0;

        for k in 0..r {
            acc += state.w[k * RMAX + i] * coeff[k];
        }

        proj[i] = acc;
        residual[i] = input[i] - acc;
    }
}

// ============================================================
// 3. LIE EVOLUTION OPERATOR (ANTI-SYMMETRIC FLOW)
// ============================================================
//
// Ż = [Z, S]κ − λZ
// ============================================================

pub fn lie_step(
    z: &mut [f32],
    s: &[f32],
    kappa: &[f32],
    lambda: f32,
    dt: f32,
    r: usize,
) {
    for k in 0..r {
        let mut torque = 0.0;

        for j in 0..r {
            let idx = k * RMAX + j;
            torque += (z[k] * s[j] - z[j] * s[k]) * kappa[idx];
        }

        z[k] += dt * (torque - lambda * z[k]);
    }
}

// ============================================================
// 4. EMA MEMORY OPERATOR
// ============================================================

pub fn ema_update(
    s: &mut [f32],
    z: &[f32],
    alpha: f32,
    r: usize,
) {
    for i in 0..r {
        s[i] = alpha * s[i] + (1.0 - alpha) * z[i];
    }
}

// ============================================================
// 5. DRIFT OPERATOR (Ω EVOLUTION)
// ============================================================

pub fn drift_update(
    omega: &mut [f32],
    z: &[f32],
    alpha: f32,
    dt: f32,
    decay: f32,
    r: usize,
) {
    for i in 0..r {
        omega[i] = (omega[i] + z[i] * alpha * dt) * decay;
    }
}
// ============================================================
// NOVELTY NOTES (DVSM-π+++)
// ============================================================
//
// DEFINITION:
// Novelty is the magnitude of information not representable
// by the current manifold basis W.
//
// FORM:
//
//     ν = ||R||
//
// where:
//
//     R = Z - W(WᵀZ)
//
// ------------------------------------------------------------
// INTERPRETATION
// ------------------------------------------------------------
//
// Low ν:
// - input already lies on manifold
// - system is coherent / phase-aligned
// - basis W sufficiently explains state Z
//
// High ν:
// - incoming structure is not represented
// - manifold underfits current excitation
// - triggers adaptation pressure
//
// ------------------------------------------------------------
// GEOMETRIC MEANING
// ------------------------------------------------------------
//
// ν is NOT "randomness".
//
// ν measures:
//
//     distance from latent state Z
//     to manifold subspace span(W)
//
// Thus:
//
//     ν = orthogonal information energy
//
// ------------------------------------------------------------
// PIPELINE ROLE
// ------------------------------------------------------------
//
// Step 2:
//     projection → residual extraction
//
// Step 5:
//     basis adaptation driven by ν
//
// Step 11:
//     trace emission thresholding
//
// ------------------------------------------------------------
// HARDENING RULES
// ------------------------------------------------------------
//
// Novelty MUST:
//
// - be computed AFTER projection
// - be computed BEFORE basis adaptation
// - NEVER directly modify Ω
// - NEVER directly classify ghosts
// - NEVER bypass containment
//
// ------------------------------------------------------------
// STABILITY INSIGHT
// ------------------------------------------------------------
//
// If:
//
//     ν → 0
//
// then:
//
//     Z ∈ span(W)
//
// meaning the manifold fully encodes the state.
//
// If:
//
//     ν grows monotonically
//
// then:
//
//     manifold drift or under-capacity exists
//
// ------------------------------------------------------------
// V16 / V17 RELATIONSHIP
// ------------------------------------------------------------
//
// V16 Acoustic Layer:
//     ν contributes spectral excitation
//
// V17-E Folding:
//     collapse may redistribute ν geometrically
//
// V17-K Probe:
//     stiffness measures how ν dissipates
//     under infinitesimal perturbation
//
// ------------------------------------------------------------
// IMPORTANT:
//
// Novelty is an OBSERVABLE.
//
// It is NOT:
//
// - entropy
// - resonance
// - drift
// - instability
//
// It is purely:
//
//     unresolved manifold information
//
// ============================================================
// ============================================================
// NOVELTY GUARDRAIL (AIR-GAP DIAMOND)
// ============================================================
//
// CRITICAL INVARIANT:
//
// Novelty ν is a DERIVED OBSERVABLE,
// never a persistent dynamical state.
//
// ------------------------------------------------------------
// FORM
// ------------------------------------------------------------
//
//     ν = ||Z - W(WᵀZ)||
//
// ν is computed transiently from:
//
// - current latent state Z
// - current manifold basis W
//
// and MUST NOT:
//
// - accumulate over time
// - feed back into velocity V
// - modify Ω drift directly
// - alter Lie evolution coefficients
// - persist as manifold memory
//
// ------------------------------------------------------------
// INTERPRETATION
// ------------------------------------------------------------
//
// Novelty is:
//
//     a thermometer, not the heat
//
// It measures unresolved manifold distance,
// but is NOT itself a causal force.
//
// ------------------------------------------------------------
// AIR-GAP DIAMOND RULE
// ------------------------------------------------------------
//
// Forbidden:
//
//     Ω ← Ω + ν
//     V ← V + ν
//     Z ← Z + ν
//
// Allowed:
//
//     ν = observe(Z, W)
//
//     if ν > ε:
//         adapt_basis(...)
//
//
// ν may TRIGGER adaptation logic,
// but never become part of the state algebra.
//
// ------------------------------------------------------------
// WHY THIS MATTERS
// ------------------------------------------------------------
//
// If ν becomes stateful:
//
// - observability contaminates dynamics
// - measurement becomes feedback
// - the manifold self-amplifies novelty
// - resonance collapses into instability
//
// This violates:
//
//     ∂V / ∂Ω = 0
//
// and breaks the Air-Gap Diamond.
//
// ------------------------------------------------------------
// CORRECT MENTAL MODEL
// ------------------------------------------------------------
//
// Z  = energy
// Ω  = memory drift
// W  = geometry
// ν  = unresolved geometric distance
//
// ν observes the mismatch.
//
// It does NOT become the mismatch.
//
// ============================================================
