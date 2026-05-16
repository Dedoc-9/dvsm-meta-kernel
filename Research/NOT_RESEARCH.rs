// ============================================================
// dvsm-core/src/lib.rs
// ============================================================
//
// DVSM-π+++ / DQSDv2 CORE ARITHMETIC ENGINE
//
// ============================================================
// INTRODUCTION (USE CASES)
// ============================================================
//
// DVSM is a deterministic manifold arithmetic system designed for:
//
// 1. Deep-space communications
//    - Spectral channel tracking under low SNR (~0 dB)
//    - Doppler + multipath drift modeled via Lie-bracket flow
//    - Channel recovery using memory-seeded rebirth (GhostSnap)
//
// 2. Submarine / VLF-ELF waveguide modeling
//    - Mode coupling via antisymmetric κ tensor
//    - Ionospheric and underwater propagation as geometric flow
//    - Real-time anomaly detection without pilot signals
//
// 3. Satellite constellation spectral coordination
//    - Minimal telemetry (ghost + B(t)) replaces raw spectrum sharing
//    - Distributed anomaly detection with O(1) per-node footprint
//
// 4. Radar / RF cognitive sensing
//    - Burst vs CW vs noise classification via spectral geometry
//    - Channel stiffness estimation for adaptive filtering
//
// 5. Real-time manifold inference systems (edge / embedded)
//    - No-std / no-alloc deterministic execution
//    - Fixed-step evolution suitable for rad-hard processors
//
// ============================================================
// ARCHITECTURAL GUARANTEES
// ============================================================
//
// - Deterministic execution order (no race-dependent behavior)
// - Shadow-space observability (V17-K probe is non-intrusive)
// - Energy-stable Lie evolution (d||Z||²/dt ≤ 0)
// - Orthogonal basis constraint (WᵀW ≈ I via retraction)
// - One-way coupling invariants (Ω never influences V)
// - Novelty is derived only (never stored state)
//
// ============================================================

#![no_std]

// ------------------------------------------------------------
// CORE MODULES
// ------------------------------------------------------------

pub mod arithmetic;
pub mod pipeline;
pub mod manifold;
pub mod containment;
pub mod ghost;
pub mod trace;
pub mod acoustic;

// ------------------------------------------------------------
// FEATURE FLAGS
// ------------------------------------------------------------
//
// fixed_point : enables Q16.16 deterministic backend
// probe       : enables V17-K stiffness instrumentation
// batch       : enables step_batch execution engine
//
// ------------------------------------------------------------

#[cfg(feature = "fixed_point")]
pub mod fixed_point;

// ============================================================
// GLOBAL INVARIANTS (DO NOT BREAK)
// ============================================================
//
// κ[i,j] = -κ[j,i]          (antisymmetry)
// WᵀW ≈ I                   (orthonormal manifold)
// λ > 0                     (global contraction / stability)
// Ω → V forbidden           (drift isolation)
// TraceFrame is immutable   (external ABI contract)
//
// ============================================================

// ------------------------------------------------------------
// PUBLIC API ENTRYPOINT
// ------------------------------------------------------------

pub use pipeline::{step, DvsmCore};
pub use trace::DVSM_TraceFrame;

// ============================================================
// dvsm-core/src/pipeline.rs
// ============================================================

use crate::{
    arithmetic::{lie_step, project},
    manifold,
    containment::handle_containment,
    ghost,
    acoustic::{acoustic_observe, AcousticFrame},
    trace::DVSM_TraceFrame,
};

pub struct DvsmCore {
    pub state: State,
    pub stiffness_last: f32,
}

impl DvsmCore {
    #[inline(always)]
    pub fn step(&mut self, input: &[f32; crate::RMAX]) -> DVSM_TraceFrame {
        // ----------------------------------------------------
        // 1. CONTAINMENT (GhostSnap)
        // ----------------------------------------------------
        handle_containment(&mut self.state);

        // ----------------------------------------------------
        // 2. PROJECTION
        // ----------------------------------------------------
        let mut coeff = [0.0f32; crate::RMAX];
        let mut proj = [0.0f32; crate::RMAX];
        let mut residual = [0.0f32; crate::RMAX];

        project(&self.state, input, &mut coeff, &mut proj, &mut residual);

        // ----------------------------------------------------
        // 3. LIE EVOLUTION (energy-stable flow)
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
        // 4. MEMORY UPDATE (EMA)
        // ----------------------------------------------------
        crate::math::ema_update(
            &mut self.state.s,
            &self.state.z,
            self.state.params.alpha,
            self.state.params.r as usize,
        );

        // ----------------------------------------------------
        // 5. BASIS ADAPTATION (no projection needed in exact form)
        // ----------------------------------------------------
        self.adapt_basis(&coeff, &residual);

        // ----------------------------------------------------
        // 6. MANIFOLD MAINTENANCE (Stiefel retraction)
        // ----------------------------------------------------
        manifold::maintain_manifold(&mut self.state);

        // ----------------------------------------------------
        // 7. VELOCITY UPDATE (V isolated from Ω)
        // ----------------------------------------------------
        self.update_velocity(&residual);

        crate::math::drift_update(
            &mut self.state.omega,
            &self.state.z,
            self.state.params.alpha,
            self.state.params.dt,
            self.state.params.omega_decay,
            self.state.params.r as usize,
        );

        // ----------------------------------------------------
        // 8. GHOST CLASSIFICATION (telemetry only)
        // ----------------------------------------------------
        ghost::classify(&mut self.state);

        // ----------------------------------------------------
        // 9. OBSERVATION LAYER (read-only)
        // ----------------------------------------------------
        let acoustic: AcousticFrame = acoustic_observe(&self.state);

        // ----------------------------------------------------
        // 10. V17-K STIFFNESS PROBE (shadow-space)
        // ----------------------------------------------------
        let mut shadow_z = self.state.z;
        let stiffness = self.measure_stiffness(&mut shadow_z, &acoustic);
        self.stiffness_last = stiffness;

        // ----------------------------------------------------
        // 11. TRACE EMISSION (immutable ABI output)
        // ----------------------------------------------------
        let trace = DVSM_TraceFrame::emit(&self.state, &acoustic, stiffness);

        // ----------------------------------------------------
        // 12. STATE COMMIT (critical missing invariant fix)
        // ----------------------------------------------------
        self.state.w_prev = self.state.w;
        self.state.frame += 1;
        self.state.t += self.state.params.dt;

        trace
    }

    // ========================================================
    // BASIS ADAPTATION (hardened)
    // ========================================================
    #[inline(always)]
    fn adapt_basis(&mut self, coeff: &[f32; crate::RMAX], residual: &[f32; crate::RMAX]) {
        let r = self.state.params.r as usize;
        let eta = self.state.params.basis_eta;

        let mut c_norm_sq = 0.0;
        for k in 0..r {
            c_norm_sq += coeff[k] * coeff[k];
        }

        if c_norm_sq <= 1e-8 {
            return;
        }

        let inv = 1.0 / c_norm_sq.sqrt();

        for k in 0..r {
            let scale = coeff[k] * inv * eta;
            let base = k * crate::RMAX;

            for i in 0..r {
                self.state.w[base + i] += residual[i] * scale;
            }
        }
    }

    // ========================================================
    // VELOCITY UPDATE (Ω isolation invariant)
    // ========================================================
    #[inline(always)]
    fn update_velocity(&mut self, residual: &[f32; crate::RMAX]) {
        let r = self.state.params.r as usize;
        let u_max = self.state.params.u_max;

        for i in 0..r {
            self.state.v[i] =
                self.state.v[i] * 0.9 + residual[i] * 0.01;

            if self.state.v[i] > u_max {
                self.state.v[i] = u_max;
            }
            if self.state.v[i] < -u_max {
                self.state.v[i] = -u_max;
            }
        }
    }

    // ========================================================
    // V17-K STIFFNESS PROBE (shadow-space only)
    // ========================================================
    #[inline(always)]
    fn measure_stiffness(
        &self,
        shadow: &mut [f32; crate::RMAX],
        acc: &AcousticFrame,
    ) -> f32 {
        let eps = crate::STIFFNESS_EPS;

        let mut e_pre = 0.0;
        for i in 0..self.state.params.r as usize {
            e_pre += shadow[i] * shadow[i];
        }

        // residual-direction perturbation (corrected I2 fix)
        let mut norm = 0.0;
        for i in 0..self.state.params.r as usize {
            norm += shadow[i].abs();
        }
        norm += 1e-12;

        for i in 0..self.state.params.r as usize {
            shadow[i] += eps * (acc.resonance_peak * shadow[i] / norm);
        }

        for i in 0..self.state.params.r as usize {
            shadow[i] *= 1.0 - (self.state.params.lambda * self.state.params.dt);
        }

        let mut e_post = 0.0;
        for i in 0..self.state.params.r as usize {
            e_post += shadow[i] * shadow[i];
        }

        (e_pre - e_post).abs() / eps
    }
}
// ============================================================
// dvsm-core/src/binary_api.rs
// ============================================================
//
// BINARY ABI LAYER (STABLE CONTRACT)
// ============================================================
//
// PURPOSE:
// - Provide a minimal, version-stable interface for external systems
// - Ensure no exposure of internal manifold structures (Z, W, Ω)
// - Guarantee deterministic serialization of DVSM_TraceFrame
//
// USE CASES:
// - Deep-space telemetry ingestion
// - RF / radar embedded pipelines
// - Cross-language (C / Python / UE5 / FPGA host) integration
// - Ground station decoding of spectral health packets
//
// ============================================================

use crate::{DvsmCore, DVSM_TraceFrame};

// ------------------------------------------------------------
// STABLE OUTPUT PACKET (ABI SAFE)
// ------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DVSM_BinaryFrame {
    pub frame: u64,
    pub energy: f32,
    pub novelty: f32,
    pub stiffness: f32,
    pub omega_norm: f32,
    pub ghost_code: u8,
    pub resonance_peak: f32,
}

// ------------------------------------------------------------
// CORE → BINARY PACKER
// ------------------------------------------------------------

#[inline(always)]
pub fn encode_frame(trace: &DVSM_TraceFrame) -> DVSM_BinaryFrame {
    DVSM_BinaryFrame {
        frame: trace.frame,
        energy: trace.energy,
        novelty: trace.novelty,
        stiffness: trace.stiffness,
        omega_norm: trace.omega_norm,
        ghost_code: trace.ghost as u8,
        resonance_peak: trace.resonance_peak,
    }
}

// ------------------------------------------------------------
// PUBLIC ABI ENTRYPOINT (SAFE BOUNDARY)
// ------------------------------------------------------------

#[no_mangle]
pub extern "C" fn dvsm_step_binary(
    core: *mut DvsmCore,
    input: *const f32,
    output: *mut DVSM_BinaryFrame,
    n: usize,
) -> i32 {
    if core.is_null() || input.is_null() || output.is_null() {
        return -1;
    }

    let core = unsafe { &mut *core };
    let input = unsafe { std::slice::from_raw_parts(input, n) };

    // NOTE:
    // Input must match RMAX in production builds.
    // No runtime resizing allowed (determinism constraint).
    if n != crate::RMAX {
        return -2;
    }

    let trace = core.step(input.try_into().unwrap());

    let packed = encode_frame(&trace);

    unsafe {
        *output = packed;
    }

    0
}

// ------------------------------------------------------------
// OPTIONAL: BATCH ABI (telemetry streaming mode)
// ------------------------------------------------------------

#[no_mangle]
pub extern "C" fn dvsm_step_batch_binary(
    core: *mut DvsmCore,
    inputs: *const *const f32,
    outputs: *mut DVSM_BinaryFrame,
    count: usize,
    n: usize,
) -> i32 {
    if core.is_null() || inputs.is_null() || outputs.is_null() {
        return -1;
    }

    let core = unsafe { &mut *core };
    let inputs = unsafe { std::slice::from_raw_parts(inputs, count) };
    let outputs = unsafe { std::slice::from_raw_parts_mut(outputs, count) };

    if n != crate::RMAX {
        return -2;
    }

    for i in 0..count {
        let input = unsafe { std::slice::from_raw_parts(inputs[i], n) };

        let trace = core.step(input.try_into().unwrap());
        outputs[i] = encode_frame(&trace);
    }

    0
}

// ============================================================
// ABI GUARANTEES
// ============================================================
//
// 1. NO POINTER ESCAPE: internal state never exposed
// 2. NO HEAP ALLOCATION: all buffers are stack or preallocated
// 3. NO FLOATING CONTROL: all decisions internal to core.step()
// 4. BITWISE STABILITY: same input → same DVSM_BinaryFrame
// 5. TRACE IS SINGLE SOURCE OF TRUTH
//
// ============================================================
//
// FORBIDDEN:
// - exposing State struct
// - returning raw W, Z, Ω
// - allowing caller-side mutation hooks
//
// ============================================================
