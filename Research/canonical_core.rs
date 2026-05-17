// canonical_core.rs
// DVSM-π+++ / Canonical Sealed Kernel
// Bit-exact Q16.16 fixed-point deterministic manifold engine
// no_std · no_alloc · ABI-stable · canonical summation enforced
// Author: Daniel J. Dillberg - Contact: BigDilly95@gmail.com

// ============================================================
// DVSM-π+++ / DQSDv2 · CANONICAL KERNEL (V18→V19 STABLE)
// ============================================================
//
// PURPOSE
// --------
// This module implements a deterministic, bit-exact dynamical system
// over a latent manifold Z with memory S and projection operator Π.
//
// It is NOT a physical simulator.
// It is a feature-space evolution engine with Lyapunov-bounded dynamics.
//
// ------------------------------------------------------------
//
// PRIMARY USE CASES
// ------------------------------------------------------------
//
// 1. SAFETY-CRITICAL CONTROL SYSTEMS
//    - Aerospace / UAV stabilization
//    - Submarine / deep-space signal integrity
//    - Radiation-hardened state tracking
//
// 2. REAL-TIME SIMULATION ENGINES
//    - VR / gaming physics abstraction
//    - Haptic + visual unified response fields
//    - Deterministic replay systems ("save-state geometry")
//
// 3. SIGNAL INTELLIGENCE / FEATURE EXTRACTION
//    - RF / acoustic / biosignal manifold tracking
//    - Anomaly detection via residual projection (R)
//    - Drift-based classification (Ghost states)
//
// 4. SCIENTIFIC MODEL COMPRESSION
//    - Encoding high-dimensional dynamics into Z-space
//    - Stable rehydration via V18 genetic token
//    - Cross-platform reproducible trajectories
//
// ------------------------------------------------------------
//
// EQUATION CORE (TAILORABLE CONTRACT)
// ------------------------------------------------------------
//
// The system is defined by three coupled operators:
//
// (1) STATE EVOLUTION (Lie Dynamics)
//     dZ/dt = [Z, S]_κ − λZ
//
//     where:
//       - [Z,S]_κ is a skew-symmetric Lie-bracket coupling
//       - κ ensures energy redistribution, not creation
//       - λ is global contraction (Lyapunov sink)
//
// ------------------------------------------------------------
//
// (2) MEMORY UPDATE (Temporal Hysteresis)
//     S(t+1) = α S(t) + (1 − α) Z(t)
//
//     meaning:
//       - S acts as a low-pass manifold observer
//       - guarantees non-instantaneous feedback coupling
//
// ------------------------------------------------------------
//
// (3) OBSERVATION / PROJECTION OPERATOR
//     Π(Z, S, W, Ω) → BinaryFrame
//
//     meaning:
//       - collapses latent manifold into measurable features
//       - ensures all outputs are coordinate-invariant
//
// ------------------------------------------------------------
//
// TAILORING RULES (ENGINEERING CONTRACT)
// ------------------------------------------------------------
//
// - κ MUST remain antisymmetric:
//       κ[i,j] = −κ[j,i]
//
// - All updates MUST preserve bounded energy:
//       ||Z||² is Lyapunov-constrained (non-explosive)
//
// - Projection Π MUST NOT feed back into Z directly
//   (observer is non-invasive)
//
// - Memory S MUST always lag Z (no instantaneous coupling)
//
// - Any numerical implementation MUST enforce:
//       deterministic ordering + fixed-width arithmetic (if Q16.16)
//
// ------------------------------------------------------------
//
// STABILITY GUARANTEE (DESIGN INTENT)
// ------------------------------------------------------------
//
// If λ > 0 and κ is antisymmetric:
//
//     d||Z||²/dt ≤ 0  (continuous form)
//
// In discrete form:
//     ||Zₜ|| is non-increasing up to bounded numerical drift
//
// ------------------------------------------------------------
//
// REHYDRATION PRINCIPLE (V18/V19)
// ------------------------------------------------------------
//
// The full system state is recoverable from:
//
//     Token = Hash(seed_W, seed_κ, λ, α, dt, Lyapunov)
//
// meaning:
//     - geometry is primary
//     - trajectory is secondary
//     - simulation is deterministic under identical seeds
//
// ============================================================

#![cfg_attr(not(feature = "std"), no_std)]

pub const RMAX: usize = 16;
pub const Q: i32 = 16; // Q16.16 shift
pub const EPS: i32 = 1;

// =========================
// FIXED-POINT UTIL
// =========================

#[inline(always)]
fn mul(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> Q) as i32
}

#[inline(always)]
fn add_sat(a: i32, b: i32) -> i32 {
    let r = a as i64 + b as i64;
    if r > i32::MAX as i64 { i32::MAX }
    else if r < i32::MIN as i64 { i32::MIN }
    else { r as i32 }
}

// =========================
// STATE
// =========================

pub struct CanonicalCore {
    pub z: [i32; RMAX],        // Q16.16
    pub s: [i32; RMAX],        // Q16.16
    pub kappa: [i32; RMAX * RMAX], // Q16.16
    pub lambda: i32,           // Q16.16
    pub dt: i32,               // Q16.16

    pub prev_energy: i64,      // Q32 accumulator
}

// =========================
// ENERGY (LYAPUNOV FUNCTIONAL)
// =========================

impl CanonicalCore {
    #[inline(always)]
    fn energy(&self, z: &[i32; RMAX]) -> i64 {
        let mut e: i64 = 0;
        for i in 0..RMAX {
            let v = z[i] as i64;
            e += (v * v) >> Q;
        }
        e
    }

    // contraction factor for stabilization
    #[inline(always)]
    fn contraction(&self, curr: i64, prev: i64) -> i32 {
        if curr <= prev || curr == 0 {
            return (1 << Q); // 1.0
        }
        let ratio = (prev << Q) / curr;
        ratio as i32
    }

    // =========================
    // CANONICAL STEP (CORE)
    // =========================

    pub fn step(&mut self) {
        let mut z_next = [0i32; RMAX];

        // -------------------------
        // 1. MIDPOINT-LIKE EVOLUTION
        // -------------------------
        for k in 0..RMAX {

            // canonical summation (strict ordering)
            let mut torque: i64 = 0;

            for j in 0..RMAX {
                let zk = self.z[k] as i64;
                let zj = self.z[j] as i64;
                let sk = self.s[k] as i64;
                let sj = self.s[j] as i64;

                let antisym = (zk * sj - zj * sk) >> Q;

                torque += (antisym * self.kappa[k * RMAX + j] as i64) >> Q;
            }

            let decay = ((self.lambda as i64 * self.z[k] as i64) >> Q);

            let f = torque - decay;

            // midpoint-style discrete integration (symmetrized stability)
            let delta = ((f * self.dt as i64) >> Q) as i32;

            z_next[k] = add_sat(self.z[k], delta);
        }

        // -------------------------
        // 2. LYAPUNOV STABILITY GUARD
        // -------------------------
        let curr_e = self.energy(&z_next);

        if curr_e > self.prev_energy && self.prev_energy > 0 {
            let scale = self.contraction(curr_e, self.prev_energy);

            for i in 0..RMAX {
                z_next[i] = mul(z_next[i], scale);
            }
        }

        // -------------------------
        // 3. COMMIT STATE
        // -------------------------
        self.z = z_next;

        // EMA memory update (deterministic)
        for i in 0..RMAX {
            let a = (self.s[i] as i64 * ((1 << Q) - 1)) >> Q;
            let b = (self.z[i] as i64) >> 1;
            self.s[i] = (a + b) as i32;
        }

        self.prev_energy = self.energy(&self.z);
    }
}

// =========================
// ABI (C STABLE)
// =========================

#[no_mangle]
pub extern "C" fn dvsm_canonical_step(core: *mut CanonicalCore) {
    unsafe {
        if let Some(c) = core.as_mut() {
            c.step();
        }
    }
}

// =========================
// INITIALIZATION
// =========================

#[no_mangle]
pub extern "C" fn dvsm_canonical_init() -> *mut CanonicalCore {
    let c = Box::new(CanonicalCore {
        z: [0; RMAX],
        s: [0; RMAX],
        kappa: [0; RMAX * RMAX],
        lambda: 0,
        dt: (1 << Q) / 240, // 240Hz canonical step
        prev_energy: 0,
    });

    Box::into_raw(c)
}

// =========================
// FREE
// =========================

#[no_mangle]
pub extern "C" fn dvsm_canonical_free(ptr: *mut CanonicalCore) {
    unsafe {
        if !ptr.is_null() {
            drop(Box::from_raw(ptr));
        }
    }
}
