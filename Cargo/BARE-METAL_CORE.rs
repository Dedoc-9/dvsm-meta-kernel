// DVSM-π+++ / DQSDv2 · BARE-METAL CORE
// no_std · no alloc · static buffers · SIMD-ready · ABI-stable
// Author: Daniel J. Dillberg
// Contact: BigDilly95@gmail.com

#![no_std]

// ── constants ───────────────────────────────────────────────
pub const R: usize = 16;
pub const R2: usize = R * R;
pub const DT: f32 = 4.166_667e-3;
pub const ALPHA: f32 = 0.98;
pub const LAMBDA: f32 = 0.05;
pub const ETA: f32 = 0.01;
pub const DAMPING: f32 = 0.98;
pub const U_MAX: f32 = 100.0;
pub const U_MAX2: f32 = U_MAX * U_MAX;
pub const EPS: f32 = 1e-8;
pub const OMEGA_DECAY: f32 = 0.999;
pub const KILL_K: u8 = 3;
pub const RAMP_FRAMES: u32 = 120;
pub const TRACE_DELTA_EPS: f32 = 1e-4;

// ── ghost ───────────────────────────────────────────────────
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ghost {
    Nominal=0, Collapse=1, Diffuse=2, Echo=3,
    Burst=4, Trap=5, Vacuum=6, Denatured=7,
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RebirthMode { Structured=0, HighEntropy=1, GhostSnap=2 }

// ── trace ───────────────────────────────────────────────────
#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct TraceFrame {
    pub frame: u64, pub stress: f32, pub novelty: f32,
    pub drift: f32, pub entropy: f32, pub energy: f32,
    pub omega_norm: f32, pub ghost: u8, pub contained: u8,
    pub emitted: u8, _pad: u8,
}

impl TraceFrame {
    pub const ZERO: Self = Self {
        frame:0, stress:0.0, novelty:0.0, drift:0.0,
        entropy:0.0, energy:0.0, omega_norm:0.0,
        ghost:0, contained:0, emitted:0, _pad:0,
    };
}

// ── portable math (auto-vectorizable, no libm) ─────────────
// all loops are stride-1 over contiguous f32 — compiler emits
// AVX-512/NEON/RVV with -C target-cpu=native

#[inline(always)]
fn dot(a: &[f32; R], b: &[f32; R], n: usize) -> f32 {
    let mut s = 0.0f32;
    let mut i = 0;
    while i < n { s += a[i] * b[i]; i += 1; }
    s
}

#[inline(always)]
fn norm2(a: &[f32; R], n: usize) -> f32 { dot(a, a, n) }

#[inline(always)]
fn norm_s(a: &[f32; R], n: usize) -> f32 {
    let v = norm2(a, n);
    // no libm: fast inverse sqrt approximation + 1 Newton step
    if v < EPS { return EPS; }
    let x = f32::from_bits(0x5f37_5a86 - (v.to_bits() >> 1));
    let x = x * (1.5 - 0.5 * v * x * x); // Newton refinement
    v * x // v * (1/√v) = √v
}

// sin approximation (no libm): Bhaskara I, max error 0.18%
#[inline(always)]
fn sin_approx(x: f32) -> f32 {
    let pi = core::f32::consts::PI;
    let x = x - (x / (2.0 * pi)).floor() * 2.0 * pi; // mod 2π
    let x = if x > pi { x - 2.0 * pi } else { x };    // [-π, π]
    let num = 16.0 * x * (pi - x.abs());
    let den = 5.0 * pi * pi - 4.0 * x.abs() * (pi - x.abs());
    num / den
}

// ── core state (static, page-aligned, L1-friendly) ─────────
#[repr(C, align(4096))]
pub struct DvsmCore {
    // hot data (fits 2 cache lines at R=16)
    pub z: [f32; R],
    pub s: [f32; R],
    pub v: [f32; R],
    pub x: [f32; R],
    pub omega: [f32; R],
    // basis (R*R = 1KB at R=16)
    pub w:      [f32; R2],
    pub kappa:  [f32; R2],
    w_prev:     [f32; R2],
    // scratch (never crosses ABI)
    c:   [f32; R],
    p:   [f32; R],
    res: [f32; R],
    // scalars
    pub n: u16, pub r: u16,
    pub frame: u64,
    pub alive: u8,
    pub contain_fails: u8,
    pub rebirth_mode: RebirthMode,
    pub frames_since_rebirth: u32,
    // delta-trace state
    prev_novelty: f32,
}

// ── fnv1a hash (portable, no deps) ──────────────────────────
pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut i = 0;
    while i < data.len() { h ^= data[i] as u64; h = h.wrapping_mul(0x100000001b3); i += 1; }
    h
}

// ── orthonormalize (MGS, stride=R, auto-vectorizable) ───────
fn orthonormalize(w: &mut [f32; R2], r: usize) {
    let mut k = 0;
    while k < r {
        let kb = k * R;
        let mut j = 0;
        while j < k {
            let jb = j * R;
            let mut d = 0.0f32;
            let mut i = 0;
            while i < R { d += w[kb+i] * w[jb+i]; i += 1; }
            i = 0;
            while i < R { w[kb+i] -= d * w[jb+i]; i += 1; }
            j += 1;
        }
        let mut n2 = 0.0f32;
        let mut i = 0;
        while i < R { n2 += w[kb+i] * w[kb+i]; i += 1; }
        let inv = {
            if n2 < EPS { 1.0 / EPS } else {
                let x = f32::from_bits(0x5f37_5a86 - (n2.to_bits() >> 1));
                x * (1.5 - 0.5 * n2 * x * x)
            }
        };
        i = 0;
        while i < R { w[kb+i] *= inv; i += 1; }
        k += 1;
    }
}

fn stiefel_drift(w: &[f32; R2], r: usize) -> f32 {
    let mut d = 0.0f32;
    let mut k1 = 0;
    while k1 < r {
        let mut k2 = 0;
        while k2 < r {
            let mut g = 0.0f32;
            let mut i = 0;
            while i < R { g += w[k1*R+i] * w[k2*R+i]; i += 1; }
            let t = if k1 == k2 { 1.0 } else { 0.0 };
            let e = g - t;
            d += e * e;
            k2 += 1;
        }
        k1 += 1;
    }
    // approx sqrt via fast inverse sqrt
    if d < EPS { return 0.0; }
    let x = f32::from_bits(0x5f37_5a86 - (d.to_bits() >> 1));
    let x = x * (1.5 - 0.5 * d * x * x);
    d * x
}

// ── rebirth ─────────────────────────────────────────────────
fn rebirth(c: &mut DvsmCore) {
    let r = c.r as usize;
    match c.rebirth_mode {
        RebirthMode::Structured => {
            let mut k = 0;
            while k < r { c.z[k] = EPS * c.w[k*R + (k % R)]; k += 1; }
        }
        RebirthMode::HighEntropy => {
            let mut k = 0;
            while k < r {
                let mut i = 0;
                while i < R {
                    let seed = (c.frame as f32) * 0.618 + (k * R + i) as f32;
                    c.w[k*R+i] = sin_approx(seed);
                    i += 1;
                }
                k += 1;
            }
            orthonormalize(&mut c.w, r);
            k = 0;
            while k < r { c.z[k] = EPS * c.w[k*R + (k % R)]; k += 1; }
        }
        RebirthMode::GhostSnap => {
            // snap W back using last stable S as seed direction
            let s_n2 = norm2(&c.s, r);
            if s_n2 > EPS {
                let inv = {
                    let x = f32::from_bits(0x5f37_5a86 - (s_n2.to_bits() >> 1));
                    x * (1.5 - 0.5 * s_n2 * x * x)
                };
                let mut i = 0;
                while i < R { c.w[i] = c.s[i] * inv; i += 1; } // first column ← Ŝ
            }
            orthonormalize(&mut c.w, r);
            let mut k = 0;
            while k < r { c.z[k] = EPS * c.w[k*R + (k % R)]; k += 1; }
        }
    }
    c.v = [0.0; R]; c.omega = [0.0; R];
    c.alive = 1; c.contain_fails = 0; c.frames_since_rebirth = 0;
    // NOTE: s NOT zeroed in GhostSnap — preserves memory continuity
    if c.rebirth_mode != RebirthMode::GhostSnap { c.s = [0.0; R]; }
}

// ── 11-stage pipeline ───────────────────────────────────────
impl DvsmCore {
    pub const fn new(n: u16, r: u16) -> Self {
        let n = if n > R as u16 { R as u16 } else { n };
        let r = if r > n { n } else { r };
        let mut w = [0.0f32; R2];
        let mut kappa = [0.0f32; R2];
        // const init: identity basis + κ precomputed
        let mut k = 0;
        while k < r as usize { w[k*R+k] = 1.0; k += 1; }
        // κ must be computed at runtime (sin not const) — zeroed here, call init_kappa()
        Self {
            z:[0.0;R], s:[0.0;R], v:[0.0;R], x:[0.0;R], omega:[0.0;R],
            w, kappa, w_prev:w, c:[0.0;R], p:[0.0;R], res:[0.0;R],
            n, r, frame:0, alive:1, contain_fails:0,
            rebirth_mode:RebirthMode::Structured,
            frames_since_rebirth: u32::MAX, prev_novelty: 0.0,
        }
    }

    /// Must call once after new() — sin not available in const context
    pub fn init_kappa(&mut self) {
        let r = self.r as usize;
        let mut i = 0;
        while i < r {
            let mut j = 0;
            while j < r {
                self.kappa[i*R+j] = sin_approx((i as f32)*1.37 - (j as f32)*1.73);
                j += 1;
            }
            i += 1;
        }
    }

    pub fn step(&mut self, input: &[f32]) -> TraceFrame {
        let n = self.n as usize;
        let r = self.r as usize;
        let in_n = if input.len() < n { input.len() } else { n };

        // 1. CONTAINMENT
        let e2 = norm2(&self.z, r);
        if e2 > U_MAX2 || e2 != e2 { // NaN check: e2 != e2
            self.contain_fails += 1;
        } else {
            self.contain_fails = 0;
        }
        let killed = self.contain_fails >= KILL_K;
        if killed {
            let pre_ent = {
                let tot = e2 + EPS;
                let mut h = 0.0f32; let mut k = 0;
                while k < r {
                    let pk = self.z[k]*self.z[k]/tot;
                    if pk > EPS { h -= pk * ln_approx(pk); }
                    k += 1;
                }
                h
            };
            let ln_r = ln_approx(r as f32);
            self.rebirth_mode = if pre_ent > ln_r * 0.8 {
                RebirthMode::HighEntropy
            } else if norm2(&self.s, r) > EPS {
                RebirthMode::GhostSnap
            } else {
                RebirthMode::Structured
            };
            let mut k = 0;
            while k < r { self.z[k] = 0.0; k += 1; }
            self.alive = 0;
        }
        if self.alive == 0 { rebirth(self); }

        // 2. PROJECTION (SIMD-friendly: stride-1 f32 loops)
        let mut k = 0;
        while k < r {
            let mut s = 0.0f32; let mut i = 0;
            while i < in_n { s += self.w[k*R+i] * input[i]; i += 1; }
            self.c[k] = s;
            k += 1;
        }
        let mut i = 0;
        while i < in_n { self.p[i] = 0.0; i += 1; }
        k = 0;
        while k < r {
            i = 0;
            while i < in_n { self.p[i] += self.w[k*R+i] * self.c[k]; i += 1; }
            k += 1;
        }
        let mut r_n2 = 0.0f32;
        i = 0;
        while i < in_n {
            self.res[i] = input[i] - self.p[i];
            r_n2 += self.res[i] * self.res[i];
            i += 1;
        }
        let r_norm = {
            if r_n2 < EPS { 0.0 } else {
                let x = f32::from_bits(0x5f37_5a86 - (r_n2.to_bits() >> 1));
                r_n2 * x * (1.5 - 0.5 * r_n2 * x * x)
            }
        };

        // 3. LIE EVOLUTION
        k = 0;
        while k < r {
            let mut acc = 0.0f32; let mut j = 0;
            while j < r {
                if j != k {
                    acc += (self.z[k]*self.s[j] - self.z[j]*self.s[k]) * self.kappa[k*R+j];
                }
                j += 1;
            }
            self.z[k] += DT * (acc - LAMBDA * self.z[k]);
            k += 1;
        }

        // 4. EMA (frozen during containment)
        if self.contain_fails == 0 {
            k = 0;
            while k < r { self.s[k] = ALPHA*self.s[k] + (1.0-ALPHA)*self.z[k]; k += 1; }
        }

        // 5. BASIS ADAPT
        if r_norm > EPS {
            let cn = norm_s(&self.c, r);
            k = 0;
            while k < r {
                let sc = self.c[k] / cn;
                i = 0;
                while i < in_n { self.w[k*R+i] += ETA * self.res[i] * sc; i += 1; }
                k += 1;
            }
        }

        // 6. MANIFOLD MAINTAIN
        let drift = stiefel_drift(&self.w, r);
        if drift > 1e-6 { orthonormalize(&mut self.w, r); }
        k = 0;
        while k < r {
            let kb = k*R;
            let mut dp = 0.0f32; i = 0;
            while i < n { dp += self.w[kb+i] * self.w_prev[kb+i]; i += 1; }
            if dp < 0.0 { i = 0; while i < n { self.w[kb+i] *= -1.0; i += 1; } }
            k += 1;
        }
        self.w_prev = self.w;

        // 7. VELOCITY
        i = 0;
        while i < in_n {
            let dx = self.res[i] + self.s[i];
            let nv = self.v[i] * DAMPING + dx * ETA;
            self.v[i] = if nv > U_MAX { U_MAX } else if nv < -U_MAX { -U_MAX } else { nv };
            self.x[i] += self.v[i] * DT;
            i += 1;
        }

        // 8. OMEGA
        k = 0;
        while k < r { self.omega[k] = (self.omega[k] + self.z[k]*ALPHA*DT) * OMEGA_DECAY; k += 1; }

        // 9. CLASSIFY
        let z_n = norm_s(&self.z, r);
        let s_n = norm_s(&self.s, r);
        let stress = s_n / z_n;
        let in_norm = { let mut s=0.0f32; i=0; while i<in_n { s+=input[i]*input[i]; i+=1; } norm_s_val(s) };
        let novelty = r_norm / in_norm;
        let drift_safe = if drift != drift { 0.0 } else if drift < 0.0 { 0.0 } else { drift };
        let entropy = {
            let tot = norm2(&self.z, r) + EPS;
            let mut h = 0.0f32; k = 0;
            while k < r { let pk = self.z[k]*self.z[k]/tot; if pk>EPS { h -= pk * ln_approx(pk); } k += 1; }
            if self.frames_since_rebirth < RAMP_FRAMES {
                let ramp = self.frames_since_rebirth as f32 / RAMP_FRAMES as f32;
                ramp * h + (1.0-ramp) * ln_approx(r as f32)
            } else { h }
        };
        let o_n = norm_s(&self.omega, r);
        let omega_ratio = o_n / z_n;
        let ghost =
            if killed { Ghost::Vacuum }
            else if self.rebirth_mode == RebirthMode::HighEntropy && self.frames_since_rebirth < RAMP_FRAMES { Ghost::Denatured }
            else if stress > 1.5 { Ghost::Burst }
            else if novelty < EPS && entropy < 0.1 { Ghost::Collapse }
            else if novelty > 0.9 && entropy > 2.0 { Ghost::Diffuse }
            else if entropy < 0.3 && stress < 0.1 { Ghost::Echo }
            else if omega_ratio > 1.0 || drift_safe > 0.01 { Ghost::Trap }
            else { Ghost::Nominal };

        // 10. ADVANCE
        self.frame += 1;
        if self.frames_since_rebirth < u32::MAX { self.frames_since_rebirth += 1; }

        // 11. EMIT (delta-encoded: skip if Δnovelty < ε)
        let delta = novelty - self.prev_novelty;
        let emit = delta > TRACE_DELTA_EPS || delta < -TRACE_DELTA_EPS || killed || self.frame < 2;
        self.prev_novelty = novelty;

        TraceFrame {
            frame: self.frame, stress, novelty, drift: drift_safe,
            entropy, energy: z_n, omega_norm: o_n,
            ghost: ghost as u8, contained: killed as u8,
            emitted: emit as u8, _pad: 0,
        }
    }

    #[inline] pub fn is_vacuum(&self) -> bool { self.alive == 0 }
}

// ── no-std math helpers ─────────────────────────────────────
#[inline(always)]
fn norm_s_val(v: f32) -> f32 {
    if v < EPS { return EPS; }
    let x = f32::from_bits(0x5f37_5a86 - (v.to_bits() >> 1));
    v * x * (1.5 - 0.5 * v * x * x)
}

// ln approximation (no libm): log2 via bit-cast + polynomial
#[inline(always)]
fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 { return -20.0; } // floor for safety
    let bits = x.to_bits() as i32;
    let exp = ((bits >> 23) & 0xff) - 127;
    let frac = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000); // [1,2)
    let log2 = exp as f32 + (frac - 1.0) * (2.0 - 0.333 * (frac - 1.0));
    log2 * 0.693_147_2 // log2 * ln(2)
}

// ── C ABI ───────────────────────────────────────────────────

#[no_mangle] pub extern "C" fn dvsm_init(n: u32, r: u32) -> *mut DvsmCore {
    // only heap allocation in entire codebase
    let mut c = unsafe {
        let layout = core::alloc::Layout::new::<DvsmCore>();
        let ptr = core::alloc::alloc_zeroed(layout) as *mut DvsmCore;
        &mut *ptr
    };
    *c = DvsmCore::new(n as u16, r as u16);
    c.init_kappa();
    c as *mut DvsmCore
}

#[no_mangle] pub unsafe extern "C" fn dvsm_step(
    core: *mut DvsmCore, input: *const f32, len: u32, out: *mut TraceFrame,
) -> i32 {
    let c = match core.as_mut() { Some(c) => c, None => return -1 };
    let n = if (c.n as u32) < len { c.n as usize } else { len as usize };
    if input.is_null() || n == 0 { return -2; }
    let inp = core::slice::from_raw_parts(input, n);
    let tf = c.step(inp);
    if let Some(o) = out.as_mut() { *o = tf; }
    0
}

#[no_mangle] pub unsafe extern "C" fn dvsm_is_vacuum(core: *const DvsmCore) -> u8 {
    match core.as_ref() { Some(c) => (c.alive == 0) as u8, None => 1 }
}

#[no_mangle] pub unsafe extern "C" fn dvsm_get_trace(
    _c: *const DvsmCore, f: *const TraceFrame, o: *mut TraceFrame,
) -> i32 {
    match (f.as_ref(), o.as_mut()) { (Some(f), Some(o)) => { *o = *f; 0 }, _ => -1 }
}

#[no_mangle] pub unsafe extern "C" fn dvsm_free(core: *mut DvsmCore) {
    if !core.is_null() {
        let layout = core::alloc::Layout::new::<DvsmCore>();
        core::alloc::dealloc(core as *mut u8, layout);
    }
}

// extern alloc required for dvsm_init/dvsm_free
// on bare-metal: provide #[global_allocator] or replace with static instance
extern crate alloc;
use alloc::alloc::{alloc_zeroed, dealloc};
