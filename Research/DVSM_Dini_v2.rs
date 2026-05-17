// dvsm-core/src/dvsm_v2_fringe.rs
// DVSM-π+++ V2 · Geometric Coupling Kernel
// Includes: Klein / Dini / Rose / Lie + Stiefel + Ghost Snap
// Author: Daniel J. Dillberg
// Contact: BigDilly95@gmail.com

#![no_std]

use core::ops::{Add, Sub, Mul};

// ============================================================
// CONFIG
// ============================================================

pub const RMAX: usize = 16;
pub const Q: i32 = 16; // Q16.16 fixed-point

#[inline(always)]
fn qmul(a: i32, b: i32) -> i32 {
    ((a as i64 * b as i64) >> Q) as i32
}

#[inline(always)]
fn clamp_i32(x: i32, a: i32, b: i32) -> i32 {
    if x < a { a } else if x > b { b } else { x }
}

// ============================================================
// STATE
// ============================================================

pub struct State {
    pub z: [i32; RMAX],
    pub s: [i32; RMAX],
    pub kappa: [[i32; RMAX]; RMAX],
    pub alive: bool,
}

// ============================================================
// GEOMETRIC PRIMITIVES (CORE LAYERS)
// ============================================================

#[inline(always)]
fn lie_bracket(z: &[i32; RMAX], s: &[i32; RMAX], kappa: &[[i32; RMAX]; RMAX]) -> i32 {
    let mut acc = 0;
    for i in 0..RMAX {
        for j in 0..RMAX {
            acc += qmul(z[i].wrapping_sub(z[j]), s[j]) * kappa[i][j];
        }
    }
    acc
}

#[inline(always)]
fn klein_fold(z: &[i32; RMAX], s: &[i32; RMAX]) -> i32 {
    let mut acc = 0;
    for i in 0..RMAX {
        acc += z[i].wrapping_sub(s[RMAX - 1 - i]);
    }
    acc
}

#[inline(always)]
fn dini_damp(z: &[i32; RMAX]) -> i32 {
    let mut acc = 0;
    for i in 0..RMAX {
        acc -= (z[i].abs() >> 2);
    }
    acc
}

#[inline(always)]
fn rose_attractor(z: &[i32; RMAX]) -> i32 {
    let mut acc = 0;
    for i in 0..RMAX {
        acc += qmul(z[i], (i as i32) << 8);
    }
    acc
}

// ============================================================
// RETRACTION (STIEFEL-LIKE NORMALIZATION)
// ============================================================

#[inline(always)]
fn stiefel_retract(x: i32) -> i32 {
    // bounded normalization proxy
    clamp_i32(x, -1 << Q, 1 << Q)
}

// ============================================================
// STITCH GUARD (LYAPUNOV / SAFETY CHECK)
// ============================================================

#[inline(always)]
fn stitch_guard_check(x: i32) -> bool {
    x.abs() < (3 << Q)
}

// ============================================================
// GHOST SNAP REBIRTH
// ============================================================

fn handle_ghost_snap_rebirth(state: &mut State) {
    for i in 0..RMAX {
        state.z[i] = state.s[i]; // memory reseed
    }
}

// ============================================================
// ============================================================
//  MAIN DVSM V2 PIPELINE (EXACT STRUCTURE PRESERVED)
// ============================================================
// ============================================================

pub fn dvsm_step_v2(state: &mut State, dt: i32, lambda: i32) {
    for i in 0..RMAX {

        // --- 1. COUPLED GEOMETRIC FLOW ---
        let coupling: i32 =
            lie_bracket(&state.z, &state.s, &state.kappa)
          + klein_fold(&state.z, &state.s)
          + dini_damp(&state.z)
          + rose_attractor(&state.z);

        // --- 2. LIE + DISSIPATIVE EVOLUTION ---
        let raw_step: i32 =
            state.z[i]
          + qmul(dt, coupling - qmul(lambda, state.z[i]));

        // --- 3. STIEFEL RETRACTION (ORTHOGONAL CONSTRAINT) ---
        state.z[i] = stiefel_retract(raw_step);

        // --- 4. STITCH / LYAPUNOV / GHOST GUARD ---
        if !stitch_guard_check(state.z[i]) {
            handle_ghost_snap_rebirth(state);
            break;
        }
    }
}

// ----------------------------------------------------------------------------

// dvsm_core_v1b.rs
// Projection-Stabilized Recurrence Kernel
// C ABI · Deterministic · Reset-safe

#![no_std]

use core::ptr;

// ============================================================
// CONFIG
// ============================================================

pub const N: usize = 16;
pub const WN: usize = 256;

const EPS: f64 = 1e-12;

// Reset hysteresis (MTBR protection band)
const TH_HIGH: f64 = 10.0;
const TH_LOW: f64  = 6.0;

// ============================================================
// STATE (C ABI SAFE)
// ============================================================

#[repr(C)]
pub struct DVSMState {
    pub z: [f64; N],
    pub s: [f64; N],
    pub w: [f64; WN],

    pub dt: f64,
    pub lambda: f64,

    pub frame: u64,
    pub reset_flag: u8,
}

// ============================================================
// MATH PRIMITIVES
// ============================================================

#[inline(always)]
fn norm2(x: &[f64; N]) -> f64 {
    let mut s = 0.0;
    let mut i = 0;
    while i < N {
        s += x[i] * x[i];
        i += 1;
    }
    s
}

#[inline(always)]
fn clamp(x: f64, a: f64, b: f64) -> f64 {
    if x < a { a } else if x > b { b } else { x }
}

// ============================================================
// COUPLING FIELD
// ============================================================

#[inline(always)]
fn coupling(z: &[f64; N], s: &[f64; N]) -> f64 {
    let mut acc = 0.0;
    let mut i = 0;

    while i < N {
        acc += z[i] * s[N - 1 - i]; // Klein-like fold symmetry
        acc -= (z[i] - s[i]).abs(); // dissipative correction
        i += 1;
    }

    acc
}

// ============================================================
// CAULEY-LIKE ORTHOGONAL FLOW (SIMPLIFIED)
// ============================================================
//
// Replaces Stiefel retraction with stable bounded projection
// (ABI-safe approximation of orthogonal manifold flow)
//

#[inline(always)]
fn cayley_project(x: f64) -> f64 {
    // bounded Lie-algebra-inspired stabilization
    x / (1.0 + x.abs())
}

// ============================================================
// RESET MAP (GHOST SNAP WITH HYSTERESIS)
// ============================================================

#[inline(always)]
fn reset_map(state: &mut DVSMState) {
    let mut i = 0;
    while i < N {
        state.z[i] = state.s[i]; // memory reseed
        i += 1;
    }
}

// ============================================================
// STABILITY FUNCTIONAL (LYAPUNOV-LIKE)
// ============================================================

#[inline(always)]
fn phi(state: &DVSMState) -> f64 {
    norm2(&state.z)
}

// ============================================================
// CORE STEP (V1B GUARANTEE LAYER)
// ============================================================

#[no_mangle]
pub extern "C" fn dvsm_step(state: *mut DVSMState) {
    unsafe {
        if state.is_null() { return; }
        let state = &mut *state;

        // ----------------------------------------------------
        // 1. Compute stability functional
        // ----------------------------------------------------
        let p = phi(state);

        // ----------------------------------------------------
        // 2. RESET HYSTERESIS (MTBR PROTECTION)
        // ----------------------------------------------------
        if p > TH_HIGH {
            state.reset_flag = 1;
            reset_map(state);
        } else if p < TH_LOW {
            state.reset_flag = 0;
        }

        // ----------------------------------------------------
        // 3. COUPLING FIELD
        // ----------------------------------------------------
        let c = coupling(&state.z, &state.s);

        let dt = clamp(state.dt, 0.0, 0.05);

        // ----------------------------------------------------
        // 4. DYNAMICAL UPDATE
        // ----------------------------------------------------
        let mut i = 0;
        while i < N {

            let drift =
                c
                - state.lambda * state.z[i];

            let raw =
                state.z[i] + dt * drift;

            // ------------------------------------------------
            // 5. CAULEY PROJECTION (STABILIZED FLOW)
            // ------------------------------------------------
            state.z[i] = cayley_project(raw);

            i += 1;
        }

        // ----------------------------------------------------
        // 6. MEMORY UPDATE (EMA-LIKE)
        // ----------------------------------------------------
        let mut j = 0;
        while j < N {
            state.s[j] = 0.98 * state.s[j] + 0.02 * state.z[j];
            j += 1;
        }

        // ----------------------------------------------------
        // 7. FRAME ADVANCE
        // ----------------------------------------------------
        state.frame += 1;
    }
}

// ============================================================
// C ABI HELPERS
// ============================================================

#[no_mangle]
pub extern "C" fn dvsm_init(state: *mut DVSMState) {
    unsafe {
        if state.is_null() { return; }
        let s = &mut *state;

        let mut i = 0;
        while i < N {
            s.z[i] = 0.0;
            s.s[i] = 0.0;
            i += 1;
        }

        s.dt = 0.01;
        s.lambda = 0.1;
        s.frame = 0;
        s.reset_flag = 0;
    }
}

#[no_mangle]
pub extern "C" fn dvsm_get_frame(state: *const DVSMState) -> u64 {
    unsafe {
        if state.is_null() { return 0; }
        (*state).frame
    }
}

#[no_mangle]
pub extern "C" fn dvsm_get_reset(state: *const DVSMState) -> u8 {
    unsafe {
        if state.is_null() { return 0; }
        (*state).reset_flag
    }
}
