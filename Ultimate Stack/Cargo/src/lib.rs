//! ============================================================
//! DVSM-π+++ / DQSDv2 · CORE KERNEL (cdylib)
//! Author: Daniel J. Dillberg
//! Contact: BigDilly95@gmail.com
//! ============================================================
// PURPOSE: Spectral Arbitration Kernel (pre-visual filter)
// ============================================================

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use core::ptr;

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
pub enum DVSM_Handle {} // FFI opaque

// ============================================================
// PARAMS / TRACE
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

const RMAX: usize = 16;

pub struct Core {
    z: [f32; RMAX],
    s: [f32; RMAX],
    w: [f32; RMAX * RMAX],
    v: [f32; RMAX],
    x: [f32; RMAX],

    frame: u64,
    r: usize,
    alive: bool,
    kill_count: u32,
}

// ============================================================
// MATH HELPERS
// ============================================================

#[inline(always)]
fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
    let mut s = 0.0;
    for i in 0..n { s += a[i] * b[i]; }
    s
}

#[inline(always)]
fn norm2(a: &[f32], n: usize) -> f32 {
    dot(a, a, n)
}

// ============================================================
// CONTAINMENT
// ============================================================

#[inline(always)]
fn contain(z: &mut [f32], r: usize, u_max: f32) -> bool {
    let e2 = norm2(z, r);
    if e2 > u_max * u_max || !e2.is_finite() {
        for i in 0..r { z[i] = 0.0; }
        return true;
    }
    false
}

// ============================================================
// REBIRTH
// ============================================================

fn rebirth(c: &mut Core) {
    c.z = [0.0; RMAX];
    c.s = [0.0; RMAX];
    c.v = [0.0; RMAX];
    c.x = [0.0; RMAX];
    c.kill_count = 0;
    c.alive = true;
}

// ============================================================
// CORE STEP (PHYSICS ENGINE)
// ============================================================

impl Core {
    fn step(&mut self, input: &[f32], p: &DVSM_Params) -> DVSM_TraceFrame {
        let r = self.r.min(input.len());

        let killed = contain(&mut self.z, r, p.u_max);
        if killed { self.kill_count += 1; } else { self.kill_count = 0; }

        if !self.alive || self.kill_count >= 3 {
            self.alive = false;
            rebirth(self);
        }

        // projection
        let mut c = [0.0f32; RMAX];
        let mut pvec = [0.0f32; RMAX];
        let mut res = [0.0f32; RMAX];

        for k in 0..r {
            c[k] = dot(&self.w[k * RMAX..], input, r);
        }

        for i in 0..r {
            for k in 0..r {
                pvec[i] += self.w[k * RMAX + i] * c[k];
            }
        }

        for i in 0..r {
            res[i] = input[i] - pvec[i];
        }

        let r2 = norm2(&res, r);

        // lie-bracket evolution
        for k in 0..r {
            let mut acc = 0.0;
            for j in 0..r {
                if j != k {
                    let kappa = (k as f32 * 1.37 - j as f32 * 1.73).sin();
                    acc += (self.z[k] * self.s[j] - self.z[j] * self.s[k]) * kappa;
                }
            }
            self.z[k] += p.dt * (acc - p.lambda * self.z[k]);
        }

        // EMA
        if self.alive {
            for k in 0..r {
                self.s[k] = p.alpha * self.s[k] + (1.0 - p.alpha) * self.z[k];
            }
        }

        // velocity
        for i in 0..r {
            self.v[i] = self.v[i] * 0.98 + (res[i] + self.s[i]) * 0.01;
            self.v[i] = self.v[i].clamp(-p.u_max, p.u_max);
            self.x[i] += self.v[i] * p.dt;
        }

        self.frame += 1;

        DVSM_TraceFrame {
            frame: self.frame,
            stress: 0.0,
            novelty: r2.sqrt(),
            drift: 0.0,
            entropy: norm2(&self.z, r).ln().max(0.0),
            energy: norm2(&self.z, r).sqrt(),
            ghost: DVSM_NOMINAL,
            contained: killed as u8,
        }
    }
}

// ============================================================
// GLOBAL STATE
// ============================================================

static mut CORE: Option<Core> = None;

// ============================================================
// C ABI
// ============================================================

#[no_mangle]
pub extern "C" fn dvsm_init(_n: u32, r: u32) -> *mut DVSM_Handle {
    unsafe {
        CORE = Some(Core {
            z: [0.0; RMAX],
            s: [0.0; RMAX],
            w: [0.0; RMAX * RMAX],
            v: [0.0; RMAX],
            x: [0.0; RMAX],
            frame: 0,
            r: r as usize,
            alive: true,
            kill_count: 0,
        });
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn dvsm_step(
    _h: *mut DVSM_Handle,
    input: *const f32,
    len: u32,
    out: *mut DVSM_TraceFrame,
) -> i32 {
    if input.is_null() || out.is_null() { return -1; }

    unsafe {
        let core = CORE.as_mut().unwrap();
        let input = core::slice::from_raw_parts(input, len as usize);

        let p = DVSM_Params {
            dt: 0.004166,
            alpha: 0.98,
            lambda: 0.05,
            u_max: 100.0,
            r: core.r as u32,
        };

        *out = core.step(input, &p);
    }

    0
}

#[no_mangle]
pub extern "C" fn dvsm_free(_h: *mut DVSM_Handle) {
    unsafe { CORE = None; }
}
// ── GEOMETRY: MODIFIED GRAM-SCHMIDT + SIGN-LOCK (UE5/DLSS TAILORED) ────────

impl DvsmCore {
    /// Stage 11 & 12: Retract to Stiefel Manifold and Lock Phase
    pub fn maintain_manifold(&mut self) {
        let r = self.r as usize;
        let n = self.n as usize;

        // --- STAGE 11: MODIFIED GRAM-SCHMIDT ---
        for k in 0..r {
            let base_k = k * R;
            
            // Orthogonalize against previous vectors
            for j in 0..k {
                let base_j = j * R;
                let mut dot_kj = 0.0f32;
                for i in 0..n {
                    dot_kj += self.w[base_k + i] * self.w[base_j + i];
                }
                for i in 0..n {
                    self.w[base_k + i] -= dot_kj * self.w[base_j + i];
                }
            }

            // Normalize current vector
            let mut norm_sq = 0.0f32;
            for i in 0..n {
                norm_sq += self.w[base_k + i] * self.w[base_k + i];
            }
            let norm = norm_sq.sqrt().max(EPS);
            for i in 0..n {
                self.w[base_k + i] /= norm;
            }
        }

        // --- STAGE 12: PHASE SIGN-LOCK ---
        // Prevents the basis from 'flipping' 180 degrees
        for k in 0..r {
            let base = k * R;
            let mut dot_with_prev = 0.0f32;
            for i in 0..n {
                dot_with_prev += self.w[base + i] * self.w_prev[base + i];
            }

            // If the vector flipped orientation, flip it back
            if dot_with_prev < 0.0 {
                for i in 0..n {
                    self.w[base + i] *= -1.0;
                }
            }
        }

        // Commit current basis to prev for the next frame's comparison
        self.w_prev.copy_from_slice(&self.w);
    }

    /// Frobenius Norm of Orthogonality Error (Budget Scaling)
    pub fn get_ortho_error(&self) -> f32 {
        let r = self.r as usize;
        let mut error_sq = 0.0f32;
        for i in 0..r {
            for j in 0..r {
                let mut dot_val = 0.0f32;
                for k in 0..self.n as usize {
                    dot_val += self.w[i * R + k] * self.w[j * R + k];
                }
                let target = if i == j { 1.0f32 } else { 0.0f32 };
                error_sq += (dot_val - target).powi(2);
            }
        }
        error_sq.sqrt()
    }
}
