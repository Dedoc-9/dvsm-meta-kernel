// src/lib.rs
// ============================================================
// DVSM-π+++ / DQSDv2 · FINALIZED CDYLIB CORE
// ============================================================
// BUILD TARGET: cdylib
// PURPOSE:
// Spectral Arbitration Kernel (Pre-Visual Temporal Governor)
//
// AUTHOR:
// Daniel J. Dillberg
// Contact: BigDilly95@gmail.com
// ============================================================
// DVSM-π+++ / DQSDv2 · FINALIZED CDYLIB CORE
// ============================================================

#![no_std]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

extern crate alloc;

use alloc::boxed::Box;
use core::{ptr, slice};

// ============================================================
// CONSTANTS
// ============================================================

const RMAX: usize = 16;
const EPS: f32 = 1e-6;
const KILL_THRESHOLD: u32 = 3;
const BASIS_ETA: f32 = 0.01;
const VELOCITY_DAMP: f32 = 0.98;
const OMEGA_DECAY: f32 = 0.9995;

// ============================================================
// GHOST SPACE
// ============================================================

pub const DVSM_NOMINAL:  u8 = 0;
pub const DVSM_COLLAPSE: u8 = 1;
pub const DVSM_DIFFUSE:  u8 = 2;
pub const DVSM_ECHO:     u8 = 3;
pub const DVSM_BURST:    u8 = 4;
pub const DVSM_TRAP:     u8 = 5;
pub const DVSM_VACUUM:   u8 = 6;

// ============================================================
// ABI TYPES
// ============================================================

#[repr(C)]
pub enum DVSM_Handle {}

#[repr(C, copy)]
pub struct DVSM_Params {
    pub dt: f32,
    pub alpha: f32,
    pub lambda: f32,
    pub u_max: f32,
    pub r: u32,
}

#[repr(C, copy)]
pub struct DVSM_TraceFrame {
    pub frame: u64,
    pub stress: f32,
    pub novelty: f32,
    pub drift: f32,
    pub entropy: f32,
    pub energy: f32,
    pub ghost: u8,
    pub contained: u8,
}

// ============================================================
// CORE STATE
// ============================================================

#[repr(C)]
pub struct CoreState {
    pub params: DVSM_Params,

    pub frame: u64,

    pub W: [f32; RMAX * RMAX],
    pub W_prev: [f32; RMAX * RMAX],

    pub Z: [f32; RMAX],
    pub S: [f32; RMAX],
    pub V: [f32; RMAX],
    pub Omega: [f32; RMAX],

    pub ghost: u8,
    pub contained: bool,
    pub fail_counter: u32,
}

// ============================================================
// MATH HELPERS
// ============================================================

#[inline(always)]
fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
    let mut s = 0.0;
    for i in 0..n {
        s += a[i] * b[i];
    }
    s
}

#[inline(always)]
fn norm2(a: &[f32], n: usize) -> f32 {
    dot(a, a, n)
}

#[inline(always)]
fn norm(a: &[f32], n: usize) -> f32 {
    norm2(a, n).sqrt().max(EPS)
}

// ============================================================
// REBIRTH
// ============================================================

fn rebirth(s: &mut CoreState) {
    s.Z = [0.0; RMAX];
    s.V = [0.0; RMAX];
    s.Omega = [0.0; RMAX];
    s.fail_counter = 0;
    s.contained = true;
    s.ghost = DVSM_VACUUM;
}

// ============================================================
// MANIFOLD MAINTENANCE (MGS + SIGN LOCK)
// ============================================================

fn maintain_manifold(s: &mut CoreState, r: usize) {

    // ---- MGS ----
    for k in 0..r {
        let bk = k * RMAX;

        for j in 0..k {
            let bj = j * RMAX;

            let mut d = 0.0;
            for i in 0..r {
                d += s.W[bk + i] * s.W[bj + i];
            }
            for i in 0..r {
                s.W[bk + i] -= d * s.W[bj + i];
            }
        }

        let mut n2 = 0.0;
        for i in 0..r {
            n2 += s.W[bk + i] * s.W[bk + i];
        }

        let inv = 1.0 / n2.sqrt().max(EPS);

        for i in 0..r {
            s.W[bk + i] *= inv;
        }
    }

    // ---- SIGN LOCK ----
    for k in 0..r {
        let b = k * RMAX;
        let mut d = 0.0;

        for i in 0..r {
            d += s.W[b + i] * s.W_prev[b + i];
        }

        if d < 0.0 {
            for i in 0..r {
                s.W[b + i] *= -1.0;
            }
        }
    }

    s.W_prev.copy_from_slice(&s.W);
}

// ============================================================
// ORTHO ERROR
// ============================================================

fn ortho_error(s: &CoreState, r: usize) -> f32 {
    let mut err = 0.0;

    for i in 0..r {
        for j in 0..r {
            let mut d = 0.0;
            for k in 0..r {
                d += s.W[i * RMAX + k] * s.W[j * RMAX + k];
            }

            let target = if i == j { 1.0 } else { 0.0 };
            let e = d - target;
            err += e * e;
        }
    }

    err.sqrt()
}

// ============================================================
// CORE STEP
// ============================================================

fn core_step(s: &mut CoreState, input: &[f32], out: &mut DVSM_TraceFrame) {
    let r = s.params.r as usize;

    // ---- CONTAINMENT ----
    let z2 = norm2(&s.Z, r);

    if !z2.is_finite() || z2 > s.params.u_max * s.params.u_max {
        s.fail_counter += 1;
    } else {
        s.fail_counter = 0;
    }

    if s.fail_counter >= KILL_THRESHOLD {
        rebirth(s);
    }

    // ---- PROJECTION ----
    let mut coeff = [0.0f32; RMAX];
    let mut proj  = [0.0f32; RMAX];
    let mut res   = [0.0f32; RMAX];

    for k in 0..r {
        coeff[k] = dot(&s.W[k * RMAX..], input, r);
    }

    for i in 0..r {
        for k in 0..r {
            proj[i] += s.W[k * RMAX + i] * coeff[k];
        }
        res[i] = input[i] - proj[i];
    }

    let rnorm = norm(&res, r);

    // ---- LIE EVOLUTION ----
    for k in 0..r {
        let mut acc = 0.0;

        for j in 0..r {
            if j == k { continue; }

            let kappa = (k as f32 * 1.37 - j as f32 * 1.73).sin();

            acc += (s.Z[k] * s.S[j] - s.Z[j] * s.S[k]) * kappa;
        }

        s.Z[k] += s.params.dt * (acc - s.params.lambda * s.Z[k]);
    }

    // ---- EMA ----
    if s.fail_counter == 0 {
        for i in 0..r {
            s.S[i] = s.params.alpha * s.S[i]
                + (1.0 - s.params.alpha) * s.Z[i];
        }
    }

    // ---- BASIS UPDATE ----
    if rnorm > EPS {
        let cn = norm(&coeff, r);

        for k in 0..r {
            let sc = coeff[k] / cn;
            for i in 0..r {
                s.W[k * RMAX + i] += BASIS_ETA * res[i] * sc;
            }
        }
    }

    // ---- MANIFOLD ----
    if ortho_error(s, r) > 1e-6 {
        maintain_manifold(s, r);
    }

    // ---- VELOCITY + OMEGA ----
    let mut drift2 = 0.0;

    for i in 0..r {
        s.V[i] = s.V[i] * VELOCITY_DAMP + res[i] * 0.01;
        s.V[i] = s.V[i].clamp(-s.params.u_max, s.params.u_max);

        s.Omega[i] =
            (s.Omega[i] + s.Z[i] * s.params.alpha * s.params.dt)
            * OMEGA_DECAY;

        drift2 += s.Omega[i] * s.Omega[i];
    }

    let energy = norm2(&s.Z, r);

    // ---- CLASSIFY ----
    s.ghost =
        if !energy.is_finite() { DVSM_BURST }
        else if energy < 1e-6 { DVSM_COLLAPSE }
        else if drift2 > 10.0 { DVSM_TRAP }
        else if drift2 > 5.0 { DVSM_ECHO }
        else { DVSM_NOMINAL };

    // ---- TRACE ----
    out.frame = s.frame;
    out.stress = norm(&s.S, r);
    out.novelty = rnorm;
    out.drift = drift2.sqrt().max(0.0);
    out.entropy = energy.ln().max(0.0);
    out.energy = energy;
    out.ghost = s.ghost;
    out.contained = s.contained as u8;

    s.frame += 1;
}

// ============================================================
// ABI EXPORTS
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dvsm_init(p: *const DVSM_Params) -> *mut DVSM_Handle {
    if p.is_null() { return ptr::null_mut(); }

    let p = *p;

    if p.r == 0 || p.r as usize > RMAX {
        return ptr::null_mut();
    }

    let mut s = Box::new(CoreState {
        params: p,
        frame: 0,

        W: [0.0; RMAX * RMAX],
        W_prev: [0.0; RMAX * RMAX],

        Z: [0.0; RMAX],
        S: [0.0; RMAX],
        V: [0.0; RMAX],
        Omega: [0.0; RMAX],

        ghost: DVSM_NOMINAL,
        contained: true,
        fail_counter: 0,
    });

    for i in 0..p.r as usize {
        s.W[i * RMAX + i] = 1.0;
        s.W_prev[i * RMAX + i] = 1.0;
    }

    Box::into_raw(s) as *mut DVSM_Handle
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_step(
    h: *mut DVSM_Handle,
    input: *const f32,
    trace: *mut DVSM_TraceFrame,
) -> i32 {

    if h.is_null() || input.is_null() {
        return -1;
    }

    let s = &mut *(h as *mut CoreState);
    let r = s.params.r as usize;

    let inp = slice::from_raw_parts(input, r);

    let mut out = DVSM_TraceFrame {
        frame: 0,
        stress: 0.0,
        novelty: 0.0,
        drift: 0.0,
        entropy: 0.0,
        energy: 0.0,
        ghost: DVSM_NOMINAL,
        contained: 0,
    };

    core_step(s, inp, &mut out);

    if !trace.is_null() {
        *trace = out;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_recalibrate(h: *mut DVSM_Handle) -> i32 {
    if h.is_null() { return -1; }

    let s = &mut *(h as *mut CoreState);
    maintain_manifold(s, s.params.r as usize);

    0
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_is_vacuum(h: *const DVSM_Handle) -> u8 {
    if h.is_null() { return 1; }
    let s = &*(h as *const CoreState);
    (s.ghost == DVSM_VACUUM) as u8
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_free(h: *mut DVSM_Handle) {
    if !h.is_null() {
        let _ = Box::from_raw(h as *mut CoreState);
    }
}
