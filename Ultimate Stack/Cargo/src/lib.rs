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

#![no_std]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

extern crate alloc;

use alloc::boxed::Box;
use core::{ptr, slice};

// ============================================================
// INVARIANTS
// ============================================================

pub const INVARIANTS: [&str; 5] = [
"μ_t immutable substrate",
"WᵀW = I enforced per step",
"d||Z||²/dt = -2λ||Z||²",
"no Ω → V backfeed",
"panic-free ABI boundary",
];

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
// OPAQUE HANDLE
// ============================================================

#[repr(C)]
pub enum DVSM_Handle {}

// ============================================================
// ABI STRUCTS
// ============================================================

#[repr(C)]
#[derive(Copy, Clone)]
pub struct DVSM_Params {
pub dt: f32,
pub alpha: f32,
pub lambda: f32,
pub u_max: f32,
pub r: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
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

```
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
```

}

// ============================================================
// HELPERS
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

fn rebirth(c: &mut CoreState) {
c.Z = [0.0; RMAX];
c.V = [0.0; RMAX];

```
c.fail_counter = 0;
c.contained = true;
c.ghost = DVSM_VACUUM;
```

}

// ============================================================
// MANIFOLD MAINTENANCE
// ============================================================

fn maintain_manifold(state: &mut CoreState) {
let r = state.params.r as usize;

```
// ---- MGS ----
for k in 0..r {
    let bk = k * RMAX;

    for j in 0..k {
        let bj = j * RMAX;

        let mut d = 0.0;

        for i in 0..r {
            d += state.W[bk + i] * state.W[bj + i];
        }

        for i in 0..r {
            state.W[bk + i] -= d * state.W[bj + i];
        }
    }

    let mut n2 = 0.0;

    for i in 0..r {
        n2 += state.W[bk + i] * state.W[bk + i];
    }

    let inv = 1.0 / n2.sqrt().max(EPS);

    for i in 0..r {
        state.W[bk + i] *= inv;
    }
}

// ---- SIGN LOCK ----
for k in 0..r {
    let b = k * RMAX;

    let mut d = 0.0;

    for i in 0..r {
        d += state.W[b + i] * state.W_prev[b + i];
    }

    if d < 0.0 {
        for i in 0..r {
            state.W[b + i] *= -1.0;
        }
    }
}

state.W_prev.copy_from_slice(&state.W);
```

}

// ============================================================
// DRIFT METRIC
// ============================================================

fn ortho_error(state: &CoreState) -> f32 {
let r = state.params.r as usize;

```
let mut err = 0.0;

for i in 0..r {
    for j in 0..r {
        let mut d = 0.0;

        for k in 0..r {
            d +=
                state.W[i * RMAX + k] *
                state.W[j * RMAX + k];
        }

        let target = if i == j { 1.0 } else { 0.0 };

        let e = d - target;

        err += e * e;
    }
}

err.sqrt()
```

}

// ============================================================
// STEP
// ============================================================

fn core_step(
state: &mut CoreState,
input: &[f32],
trace: &mut DVSM_TraceFrame,
) {
let r = state.params.r as usize;

```
// ========================================================
// 1. CONTAINMENT
// ========================================================

let z2 = norm2(&state.Z, r);

if z2 > state.params.u_max * state.params.u_max || !z2.is_finite() {
    state.fail_counter += 1;
} else {
    state.fail_counter = 0;
}

if state.fail_counter >= KILL_THRESHOLD {
    rebirth(state);
}

// ========================================================
// 2. PROJECTION
// ========================================================

let mut coeff = [0.0f32; RMAX];
let mut proj  = [0.0f32; RMAX];
let mut res   = [0.0f32; RMAX];

for k in 0..r {
    coeff[k] = dot(
        &state.W[k * RMAX..],
        input,
        r
    );
}

for i in 0..r {
    for k in 0..r {
        proj[i] += state.W[k * RMAX + i] * coeff[k];
    }

    res[i] = input[i] - proj[i];
}

let rnorm = norm(&res, r);

// ========================================================
// 3. LIE EVOLUTION
// ========================================================

for k in 0..r {
    let mut acc = 0.0;

    for j in 0..r {
        if j == k {
            continue;
        }

        let kappa =
            (k as f32 * 1.37 - j as f32 * 1.73).sin();

        acc +=
            (state.Z[k] * state.S[j]
            - state.Z[j] * state.S[k])
            * kappa;
    }

    state.Z[k] +=
        state.params.dt
        * (acc - state.params.lambda * state.Z[k]);
}

// ========================================================
// 4. EMA
// ========================================================

if state.fail_counter == 0 {
    for i in 0..r {
        state.S[i] =
            state.params.alpha * state.S[i]
            + (1.0 - state.params.alpha) * state.Z[i];
    }
}

// ========================================================
// 5. BASIS UPDATE
// ========================================================

if rnorm > EPS {
    let cn = norm(&coeff, r);

    for k in 0..r {
        let sc = coeff[k] / cn;

        for i in 0..r {
            state.W[k * RMAX + i] +=
                BASIS_ETA * res[i] * sc;
        }
    }
}

// ========================================================
// 6. MANIFOLD RETRACTION
// ========================================================

if ortho_error(state) > 1e-6 {
    maintain_manifold(state);
}

// ========================================================
// 7. VELOCITY + OMEGA
// ========================================================

let mut drift2 = 0.0;

for i in 0..r {
    state.V[i] =
        state.V[i] * VELOCITY_DAMP
        + res[i] * 0.01;

    state.V[i] =
        state.V[i]
        .clamp(
            -state.params.u_max,
             state.params.u_max
        );

    state.Omega[i] =
        (state.Omega[i]
        + state.Z[i]
        * state.params.alpha
        * state.params.dt)
        * OMEGA_DECAY;

    drift2 += state.Omega[i] * state.Omega[i];
}

// ========================================================
// 8. CLASSIFICATION
// ========================================================

let energy = norm2(&state.Z, r);

state.ghost =
    if !energy.is_finite() {
        DVSM_BURST
    } else if energy < 1e-6 {
        DVSM_COLLAPSE
    } else if drift2 > 10.0 {
        DVSM_TRAP
    } else if drift2 > 5.0 {
        DVSM_ECHO
    } else {
        DVSM_NOMINAL
    };

// ========================================================
// 9. TRACE
// ========================================================

trace.frame = state.frame;
trace.stress = norm(&state.S, r);
trace.novelty = rnorm;
trace.drift = drift2.sqrt().max(0.0);
trace.entropy = energy.ln().max(0.0);
trace.energy = energy;
trace.ghost = state.ghost;
trace.contained = state.contained as u8;

state.frame += 1;
```

}

// ============================================================
// ABI EXPORTS
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dvsm_init(
params: *const DVSM_Params
) -> *mut DVSM_Handle {
if params.is_null() {
return ptr::null_mut();
}

```
let p = *params;

if p.r == 0 || p.r as usize > RMAX {
    return ptr::null_mut();
}

let mut state = Box::new(CoreState {
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
    state.W[i * RMAX + i] = 1.0;
    state.W_prev[i * RMAX + i] = 1.0;
}

Box::into_raw(state) as *mut DVSM_Handle
```

}

#[no_mangle]
pub unsafe extern "C" fn dvsm_step(
handle: *mut DVSM_Handle,
input: *const f32,
trace_out: *mut DVSM_TraceFrame,
) -> i32 {
if handle.is_null() || input.is_null() {
return -1;
}

```
let state =
    &mut *(handle as *mut CoreState);

let r = state.params.r as usize;

let input_slice =
    slice::from_raw_parts(input, r);

let mut trace = DVSM_TraceFrame {
    frame: 0,
    stress: 0.0,
    novelty: 0.0,
    drift: 0.0,
    entropy: 0.0,
    energy: 0.0,
    ghost: DVSM_NOMINAL,
    contained: 0,
};

core_step(
    state,
    input_slice,
    &mut trace,
);

if !trace_out.is_null() {
    *trace_out = trace;
}

0
```

}

#[no_mangle]
pub unsafe extern "C" fn dvsm_recalibrate(
handle: *mut DVSM_Handle
) -> i32 {
if handle.is_null() {
return -1;
}

```
let state =
    &mut *(handle as *mut CoreState);

maintain_manifold(state);

0
```

}

#[no_mangle]
pub unsafe extern "C" fn dvsm_is_vacuum(
handle: *const DVSM_Handle
) -> u8 {
if handle.is_null() {
return 1;
}

```
let state =
    &*(handle as *const CoreState);

(state.ghost == DVSM_VACUUM) as u8
```

}

#[no_mangle]
pub unsafe extern "C" fn dvsm_free(
handle: *mut DVSM_Handle
) {
if !handle.is_null() {
let _ =
Box::from_raw(handle as *mut CoreState);
}
}

// ============================================================
// GPU CONTRACT
// ============================================================

pub const GPU_PARITY_CONTRACT: &str =
"dvsm_gpu.wgsl defines canonical Lie/EMA/Containment kernels";
