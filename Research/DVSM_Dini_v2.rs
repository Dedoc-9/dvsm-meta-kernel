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
