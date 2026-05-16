// ============================================================
// src/core/zzman.rs
// Author: Daniel J. Dillberg
// ============================================================
//
// DVSM-π+++ / DQSDv2
// Zero-Z Manifold Manager (ZZMan)
//
// SYSTEM CLASS:
// Deterministic Manifold Controller
// (Level-4 Bioscience / Aerospace Runtime)
//
// ============================================================
//
// PURPOSE
// ------------------------------------------------------------
//
// ZZMan is the hardened execution boundary between:
//
// - arithmetic invariants
// - manifold evolution
// - observability layers
// - containment recovery
// - telemetry emission
//
// This file acts as:
//
//     deterministic HAL-adjacent runtime kernel
//
// ============================================================
//
// EXECUTION GUARANTEES
// ------------------------------------------------------------
//
// - strict sequential ordering
// - zero runtime heap allocation in hot path
// - stack-local diagnostics
// - observer isolation
// - invariant-preserving collapse recovery
// - lock-free trace emission compatibility
//
// ============================================================
//
// AIR-GAP DIAMOND RULES
// ------------------------------------------------------------
//
// Forbidden:
//
//     Ω → V
//     Trace → W
//     Acoustic → Dynamics
//     ν → λ
//
// Allowed:
//
//     residual → adaptation
//     Z → Ω
//     diagnostics → telemetry only
//
// ============================================================
//
// MEMORY MODEL
// ------------------------------------------------------------
//
// - static preallocated state
// - no Vec allocation in mission loop
// - no heap mutation after boot
// - stack-local shadow probing only
//
// ============================================================
//
// SAFETY NOTES
// ------------------------------------------------------------
//
// - panics should map to reset handler in deployment builds
// - FTZ/DAZ recommended for deterministic FPU behavior
// - Lie inner loop branch-free
// - Stiefel audit required periodically
//
// ============================================================

#![allow(non_snake_case)]

use crate::{
    AcousticFrame,
    DVSM_TraceFrame,
    State,
    RMAX,
    DT,
    LAMBDA,
    U_MAX_SQ,
};

use crate::acoustic::acoustic_observe;

use crate::arithmetic::{
    project,
    lie_step,
    ema_update,
    drift_update,
    norm_sq,
};

use crate::containment::handle_containment;

use crate::ghost;
use crate::manifold;

// ============================================================
// ZZMAN
// ============================================================

pub struct ZZMan {

    // --------------------------------------------------------
    // persistent deterministic runtime state
    // --------------------------------------------------------

    pub state: Box<State>,

    // --------------------------------------------------------
    // latest diagnostic scalar
    // --------------------------------------------------------

    pub stiffness_last: f32,
}

// ============================================================
// IMPLEMENTATION
// ============================================================

impl ZZMan {

    // ========================================================
    // MISSION CYCLE
    // ========================================================
    //
    // ORDER:
    //
    // 1. containment
    // 2. projection
    // 3. lie evolution
    // 4. ema memory
    // 5. basis adaptation
    // 6. manifold maintenance
    // 7. velocity / omega
    // 8. ghost classification
    // 9. acoustic observation
    // 10. kinetic probe
    // 11. trace emission
    //
    // ========================================================

    #[inline(always)]
    pub fn execute_mission_cycle(
        &mut self,
        input: &[f32; RMAX],
    ) -> DVSM_TraceFrame {

        // ----------------------------------------------------
        // STEP 1
        // CONTAINMENT / GHOSTSNAP
        // ----------------------------------------------------

        handle_containment(
            &mut self.state,
        );

        // ----------------------------------------------------
        // STEP 2
        // PROJECTION
        // ----------------------------------------------------

        let mut coeff =
            [0.0f32; RMAX];

        let mut proj =
            [0.0f32; RMAX];

        let mut residual =
            [0.0f32; RMAX];

        project(
            &self.state,
            input,
            &mut coeff,
            &mut proj,
            &mut residual,
        );

        // ----------------------------------------------------
        // STEP 3
        // LIE EVOLUTION
        // ----------------------------------------------------

        lie_step(
            &mut self.state.z,
            &self.state.s,
            &self.state.kappa,
            self.state.params.lambda,
            self.state.params.dt,
            self.state.params.r as usize,
        );

        // ----------------------------------------------------
        // STEP 4
        // EMA MEMORY
        // ----------------------------------------------------

        ema_update(
            &mut self.state.s,
            &self.state.z,
            self.state.params.alpha,
            self.state.params.r as usize,
        );

        // ----------------------------------------------------
        // STEP 5
        // BASIS ADAPTATION
        // ----------------------------------------------------

        self.adapt_basis(
            &coeff,
            &residual,
        );

        // ----------------------------------------------------
        // STEP 6
        // STIEFEL RETRACTION
        // ----------------------------------------------------

        manifold::maintain_manifold(
            &mut self.state,
        );

        // ----------------------------------------------------
        // STEP 7
        // VAJRA ISOLATION
        // ----------------------------------------------------
        //
        // Ω isolated from V
        //
        // ----------------------------------------------------

        self.update_velocity(
            &residual,
        );

        drift_update(
            &mut self.state.omega,
            &self.state.z,
            self.state.params.alpha,
            DT,
            self.state.params.omega_decay,
            self.state.params.r as usize,
        );

        // ----------------------------------------------------
        // STEP 8
        // GHOST CLASSIFICATION
        // ----------------------------------------------------

        ghost::classify(
            &mut self.state,
        );

        // ----------------------------------------------------
        // STEP 9
        // V16 ACOUSTIC OBSERVER
        // ----------------------------------------------------
        //
        // read-only observability boundary
        //
        // ----------------------------------------------------

        let acoustic =
            acoustic_observe(
                &self.state,
            );

        // ----------------------------------------------------
        // STEP 10
        // V17-K FINSLER PROBE
        // ----------------------------------------------------
        //
        // stack-local shadow probing
        //
        // NO observer persistence
        //
        // ----------------------------------------------------

        let stiffness =
            self.measure_stiffness(
                &acoustic,
            );

        self.stiffness_last =
            stiffness;

        // ----------------------------------------------------
        // STEP 11
        // TRACE EMISSION
        // ----------------------------------------------------

        let trace =
            DVSM_TraceFrame::emit(
                &self.state,
                &acoustic,
                stiffness,
            );

        self.state.frame += 1;

        trace
    }

    // ========================================================
    // BASIS ADAPTATION
    // ========================================================

    #[inline(always)]
    fn adapt_basis(
        &mut self,
        coeff: &[f32; RMAX],
        residual: &[f32; RMAX],
    ) {

        let r =
            self.state.params.r as usize;

        let mut coeff_norm =
            0.0f32;

        for i in 0..r {
            coeff_norm += coeff[i] * coeff[i];
        }

        coeff_norm =
            coeff_norm.sqrt();

        if coeff_norm <= 1e-8 {
            return;
        }

        for k in 0..r {

            let scale =
                coeff[k] / coeff_norm;

            for i in 0..r {

                self.state.w[k * RMAX + i] +=
                    self.state.params.basis_eta
                    * residual[i]
                    * scale;
            }
        }
    }

    // ========================================================
    // VELOCITY UPDATE
    // ========================================================
    //
    // INVARIANT:
    //
    //     ∂V / ∂Ω = 0
    //
    // ========================================================

    #[inline(always)]
    fn update_velocity(
        &mut self,
        residual: &[f32; RMAX],
    ) {

        let r =
            self.state.params.r as usize;

        for i in 0..r {

            self.state.v[i] =
                self.state.v[i]
                * self.state.params.velocity_damp
                + residual[i] * 0.01;

            self.state.v[i] =
                self.state.v[i]
                .clamp(
                    -self.state.params.u_max,
                     self.state.params.u_max,
                );
        }
    }

    // ========================================================
    // V17-K
    // TERMINAL FINSLER PROBE
    // ========================================================
    //
    // PURPOSE:
    //
    // estimate local manifold stiffness
    // WITHOUT modifying active state
    //
    // ========================================================

    #[inline(always)]
    fn measure_stiffness(
        &self,
        acoustic: &AcousticFrame,
    ) -> f32 {

        if acoustic.resonance_peak <= 1.0 {
            return 0.0;
        }

        // ----------------------------------------------------
        // stack-local shadow copy
        // ----------------------------------------------------

        let mut shadow_z =
            self.state.z;

        let eps =
            1e-4f32;

        // ----------------------------------------------------
        // perturbation
        // ----------------------------------------------------

        for i in 0..RMAX {

            shadow_z[i] +=
                eps
                * acoustic.resonance_peak;
        }

        let e_pre =
            norm_sq(
                &shadow_z,
                RMAX,
            );

        // ----------------------------------------------------
        // infinitesimal relaxation
        // ----------------------------------------------------

        for i in 0..RMAX {

            shadow_z[i] *=
                1.0
                - (
                    LAMBDA
                    * DT
                );
        }

        let e_post =
            norm_sq(
                &shadow_z,
                RMAX,
            );

        // ----------------------------------------------------
        // local stiffness estimate
        // ----------------------------------------------------

        (
            e_pre
            - e_post
        )
        .abs()
        / eps
    }

    // ========================================================
    // STIEFEL AUDIT
    // ========================================================

    #[inline(always)]
    pub fn verify_orthogonality(
        &self,
    ) -> bool {

        let drift =
            manifold::stiefel_drift(
                &self.state.w,
                self.state.params.r as usize,
            );

        drift <= 1e-6
    }
}

// ============================================================
// FINAL HARDENING NOTE
// ============================================================
//
// ZZMan defines:
//
// - deterministic manifold execution
// - invariant-preserving evolution
// - non-invasive observability
// - stack-local stiffness probing
// - GhostSnap containment recovery
//
// The system is:
//
// observed
// measured
// perturbed
//
// but never recursively contaminated
// by its own diagnostics.
//
// ============================================================

// ========================================================
// BASIS ADAPTATION
// ========================================================
//
// PURPOSE:
// Residual-driven manifold shaping.
//
// FORM:
//
//     W ← W + η · R ⊗ (c / ||c||)
//
// where:
//
//     R ⟂ span(W)
//
// ========================================================
//
// RESOLVES:
// The Computational Redundancy Debate
//
// ========================================================
//
// ARITHMETIC RESULT
// --------------------------------------------------------
//
// Because:
//
//     WᵀR = 0
//
// the outer-product update already lies in the tangent
// space of the Stiefel manifold in exact arithmetic.
//
// Therefore:
//
//     Proj_TW(R ⊗ c) = R ⊗ c
//
// and the explicit tangent projection becomes a no-op.
//
// ========================================================
//
// HARDENING NOTES
// --------------------------------------------------------
//
// - coefficient norm computed once
// - reciprocal reused
// - no heap allocation
// - branch-free inner update loop
// - manifold correction deferred to Stiefel retraction
//
// ========================================================

#[inline(always)]
fn adapt_basis(
    &mut self,
    coeff: &[f32; RMAX],
    residual: &[f32; RMAX],
) {

    let r =
        self.state.params.r as usize;

    let eta =
        self.state.params.basis_eta;

    // ----------------------------------------------------
    // coefficient norm
    // ----------------------------------------------------

    let mut c_norm_sq =
        0.0f32;

    for k in 0..r {

        c_norm_sq +=
            coeff[k]
            * coeff[k];
    }

    // ----------------------------------------------------
    // degeneracy guard
    // ----------------------------------------------------

    if c_norm_sq <= 1e-8 {
        return;
    }

    // ----------------------------------------------------
    // normalization factor
    // ----------------------------------------------------

    let inv_c_norm =
        1.0
        / c_norm_sq.sqrt();

    // ----------------------------------------------------
    // residual-driven adaptation
    // ----------------------------------------------------
    //
    // Result 2:
    //
    // tangent projection omitted
    // because:
    //
    //     WᵀR = 0
    //
    // in exact arithmetic.
    //
    // Stiefel retraction later restores numerical
    // orthogonality under finite precision.
    //
    // ----------------------------------------------------

    for k in 0..r {

        let scale =
            coeff[k]
            * inv_c_norm
            * eta;

        let w_offset =
            k * RMAX;

        for i in 0..r {

            self.state.w[w_offset + i] +=
                residual[i]
                * scale;
        }
    }
}
// ============================================================
// DVSM-π+++ ARITHMETIC API
// ============================================================
//
// FILE ROLE:
// Deterministic arithmetic kernel contract.
//
// This module defines the invariant-preserving arithmetic
// primitives used by:
//
// - zzman.rs
// - manifold.rs
// - containment.rs
// - v16 acoustic observer
// - v17-k kinetic probe
//
// ============================================================
//
// USER NOTES
// ============================================================
//
// 1. NOVELTY IS OBSERVATIONAL
// ------------------------------------------------------------
//
// Novelty (ν) is a derived scalar:
//
//     ν = ||R||
//
// It measures deviation from the current manifold basis.
//
// Novelty MUST NEVER:
//
// - become persistent state
// - modulate λ
// - directly mutate W
// - directly mutate Ω
//
// Novelty is:
//
//     a thermometer, not the heat
//
// ============================================================
//
// 2. SUCHNESS DECAY IS FIXED
// ------------------------------------------------------------
//
// λ is a geometric dissipation constant.
//
// λ MUST remain invariant during runtime:
//
//     ∂λ / ∂ν = 0
//
// Forbidden:
//
//     lambda = f(residual)
//     lambda = f(acoustic)
//     lambda = f(trace)
//
// ============================================================
//
// 3. OBSERVERS ARE READ-ONLY
// ------------------------------------------------------------
//
// V16 and V17 layers may:
//
// - inspect
// - measure
// - emit telemetry
//
// but MUST NEVER:
//
// - mutate Z
// - mutate W
// - alter basis adaptation
// - trigger containment directly
//
// ============================================================
//
// 4. GHOSTSNAP IS THE ONLY LEGAL RESET
// ------------------------------------------------------------
//
// Active excitation Z may only be annihilated through:
//
//     containment::handle_containment()
//
// This preserves:
//
// - EMA memory S
// - manifold continuity
// - orthogonality recovery
//
// ============================================================
//
// 5. MANIFOLD MAINTENANCE IS DEFERRED
// ------------------------------------------------------------
//
// Basis adaptation intentionally permits small numerical drift.
//
// Orthogonality is restored afterward through:
//
//     manifold::maintain_manifold()
//
// This separation:
//
// - improves SIMD efficiency
// - reduces projection overhead
// - preserves deterministic ordering
//
// ============================================================
//
// 6. SHADOW-SPACE DIAGNOSTICS
// ------------------------------------------------------------
//
// V17-K stiffness probing MUST operate on:
//
//     stack-local shadow buffers
//
// Probe perturbations MUST NEVER survive frame boundaries.
//
// ============================================================
//
// 7. DETERMINISTIC EXECUTION ORDER
// ------------------------------------------------------------
//
// Canonical ordering:
//
// 1. containment
// 2. projection
// 3. novelty extraction
// 4. lie evolution
// 5. ema memory
// 6. basis adaptation
// 7. manifold maintenance
// 8. velocity / omega
// 9. ghost classification
// 10. observability
// 11. trace emission
//
// Reordering these steps may invalidate invariants.
//
// ============================================================
//
// 8. NO HEAP ACTIVITY IN HOT PATH
// ------------------------------------------------------------
//
// Mission-cycle execution MUST avoid:
//
// - Vec allocation
// - heap growth
// - dynamic dispatch
// - runtime shape changes
//
// Use:
//
// - stack-local arrays
// - fixed-rank arithmetic
// - preallocated state buffers
//
// ============================================================
//
// 9. STABILITY MODEL
// ------------------------------------------------------------
//
// Stability emerges from:
//
// - skew-symmetric Lie coupling
// - positive dissipation
// - orthogonality maintenance
// - observability isolation
//
// The runtime does not "enforce" stability externally.
//
// The arithmetic itself constrains instability.
//
// ============================================================
//
// 10. DEPLOYMENT CLASS
// ------------------------------------------------------------
//
// Intended deployment targets:
//
// - air-gapped systems
// - embedded deterministic runtimes
// - biosignal geometry experiments
// - long-duration telemetry systems
//
// This is an experimental dynamical architecture,
// not a validated scientific instrument.
//
// ============================================================

// V17-K: Terminal Finsler Probe (Shadow-space isolation)
//
// NOTE:
// shadow_z is intentionally a full stack copy to guarantee
// zero interaction with active manifold state.

#[inline(always)]
fn measure_stiffness(
    &self,
    shadow: &[f32; RMAX],
    acc: &AcousticFrame,
) -> f32 {

    let eps = 1e-4;

    // ----------------------------------------------------
    // Baseline energy (pre-perturbation)
    // ----------------------------------------------------

    let mut z = *shadow;
    let e_pre = norm_sq(&z, RMAX);

    // ----------------------------------------------------
    // Tangent perturbation (observer-driven excitation)
    // ----------------------------------------------------

    let amp = eps * acc.resonance_peak;

    for i in 0..RMAX {
        z[i] += amp;
    }

    // ----------------------------------------------------
    // Local relaxation (linearized flow operator)
    // ----------------------------------------------------

    let decay = 1.0 - (
        self.state.params.lambda
        * self.state.params.dt
    );

    for i in 0..RMAX {
        z[i] *= decay;
    }

    // ----------------------------------------------------
    // Post-energy
    // ----------------------------------------------------

    let e_post = norm_sq(&z, RMAX);

    // ----------------------------------------------------
    // stiffness = finite-difference response magnitude
    // ----------------------------------------------------

    (e_pre - e_post).abs() / eps
}
