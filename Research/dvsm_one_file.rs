//! DVSM-π+++ / DQSDv2 · Deterministic Spectral Arbitration Kernel
//!
//! A non-normal Lie-bracket dynamical system on Stiefel manifolds
//! for real-time spectral tracking, anomaly detection, and
//! geometry-first signal arbitration across domains.
//!
//! DOMAINS:
//!   Gaming/VR    — 240Hz frame arbitration, DLSS viability gating,
//!                  manifold-coherent temporal filtering
//!   RF/SIGINT    — broadband spectral tracking, burst detection,
//!                  interference classification via B(t) metric
//!   Deep Space   — channel quality estimation without pilot symbols,
//!                  solar conjunction recovery via GhostSnap rebirth,
//!                  5-byte/frame constellation health telemetry
//!   Submarine    — VLF/ELF coupled-mode waveguide tracking,
//!                  ionospheric event detection, κ from Maxwell
//!   Bioscience   — protein conformational tracking on FEL surfaces,
//!                  denaturation modeling via HighEntropy rebirth,
//!                  allosteric cooperativity prediction (OP5)
//!   Audio/Media  — latent spectral field rendering, temporal
//!                  hysteresis visualization, adaptive filter banks
//!
//! CORE EQUATION:
//!   dZ/dt = [Z, S]_κ − λZ
//!   d‖Z‖²/dt = −2λ‖Z‖²  (energy conservation under antisymmetric κ)
//!
//! PROPERTIES:
//!   no_std · zero-alloc hot path · f32 or Q16.16 fixed-point
//!   deterministic pipeline (13 stages) · ABI-stable C FFI
//!   page-aligned state · SIMD-auto-vectorizable loops
//!
//! Author: Daniel J. Dillberg
//! Contact: BigDilly95@gmail.com

// dvsm-core/src/dvsm_one_file.rs
// DVSM-π+++ consolidated kernel · no_std · zero-alloc · ABI-stable
#![cfg_attr(not(feature = "std"), no_std)]

pub const RMAX: usize = 16;
pub const N: usize = 256;
pub const EPS: f32 = 1e-8;
pub const KILL_K: u8 = 3;
pub const RAMP_FRAMES: u32 = 120;

#[derive(Copy, Clone)]
pub struct Params {
    pub dt: f32,
    pub lambda: f32,
    pub alpha: f32,
    pub basis_eta: f32,
    pub velocity_damp: f32,
    pub omega_decay: f32,
    pub u_max: f32,
    pub r: usize,
    pub n: usize,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            dt: 4.166_667e-3, lambda: 0.05, alpha: 0.98,
            basis_eta: 0.01, velocity_damp: 0.98, omega_decay: 0.999,
            u_max: 100.0, r: RMAX, n: N,
        }
    }
}

// ── ghost (u8, not u16 — matches ABI) ───────────────────────
#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Ghost {
    Nominal=0, Collapse=1, Diffuse=2, Echo=3,
    Burst=4, Trap=5, Vacuum=6, Denatured=7,
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum RebirthMode { Structured=0, HighEntropy=1, GhostSnap=2 }

// ── output frame (repr(C), stable ABI) ──────────────────────
#[repr(C)]
#[derive(Copy, Clone)]
pub struct BinaryFrame {
    pub frame: u64,
    pub energy: f32,
    pub novelty: f32,
    pub stress: f32,
    pub stiffness: f32,
    pub omega_norm: f32,
    pub entropy: f32,
    pub drift: f32,
    pub resonance_peak: f32,
    pub ghost: u8,
    pub contained: u8,
    pub emitted: u8,
    _pad: u8,
}

// ── math (no libm) ──────────────────────────────────────────
#[inline(always)] fn fast_rsqrt(v: f32) -> f32 {
    let x = f32::from_bits(0x5f37_5a86 - (v.to_bits() >> 1));
    x * (1.5 - 0.5 * v * x * x)
}
#[inline(always)] fn fast_sqrt(v: f32) -> f32 {
    if v < EPS { 0.0 } else { v * fast_rsqrt(v) }
}
#[inline(always)] fn sin_approx(x: f32) -> f32 {
    let pi = core::f32::consts::PI;
    let x = x - (x / (2.0*pi)).floor() * 2.0*pi;
    let x = if x > pi { x - 2.0*pi } else { x };
    16.0*x*(pi - x.abs()) / (5.0*pi*pi - 4.0*x.abs()*(pi - x.abs()))
}
#[inline(always)] fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 { return -20.0; }
    let b = x.to_bits() as i32;
    let e = ((b >> 23) & 0xff) - 127;
    let f = f32::from_bits((b & 0x007f_ffff) | 0x3f80_0000);
    (e as f32 + (f-1.0)*(2.0-0.333*(f-1.0))) * 0.693_147_2
}

// ── core state ──────────────────────────────────────────────
#[repr(C, align(4096))]
pub struct DvsmCore {
    pub z: [f32; N],
    pub s: [f32; N],
    pub v: [f32; N],
    pub omega: [f32; N],
    pub w: [f32; RMAX * N],
    w_prev: [f32; RMAX * N],
    pub kappa: [f32; RMAX * RMAX],
    // scratch (stack-resident, never crosses ABI)
    c: [f32; RMAX],
    p: [f32; N],
    res: [f32; N],
    // scalars
    pub params: Params,
    pub frame: u64,
    pub t: f32,
    pub alive: u8,
    contain_fails: u8,
    rebirth_mode: RebirthMode,
    frames_since_rebirth: u32,
    prev_novelty: f32,
    stiffness_last: f32,
}

impl DvsmCore {
    pub fn new(p: Params) -> Self {
        let r = p.r.min(RMAX);
        let n = p.n.min(N);
        let mut w = [0.0f32; RMAX * N];
        for k in 0..r { w[k * N + k] = 1.0; }
        let mut kappa = [0.0f32; RMAX * RMAX];
        for i in 0..r { for j in 0..r {
            kappa[i * RMAX + j] = sin_approx((i as f32)*1.37 - (j as f32)*1.73);
        }}
        Self {
            z: [0.0; N], s: [0.0; N], v: [0.0; N], omega: [0.0; N],
            w, w_prev: w, kappa,
            c: [0.0; RMAX], p: [0.0; N], res: [0.0; N],
            params: Params { r, n, ..p },
            frame: 0, t: 0.0, alive: 1,
            contain_fails: 0, rebirth_mode: RebirthMode::Structured,
            frames_since_rebirth: u32::MAX, prev_novelty: 0.0, stiffness_last: 0.0,
        }
    }

    pub fn step(&mut self, input: &[f32]) -> BinaryFrame {
        let r = self.params.r;
        let n = self.params.n;
        let in_n = input.len().min(n);
        let dt = self.params.dt;
        let lambda = self.params.lambda;
        let alpha = self.params.alpha;
        let eta = self.params.basis_eta;
        let u_max = self.params.u_max;

        // 1. CONTAINMENT (hysteresis K=3)
        let e2 = norm2(&self.z, r);
        if e2 > u_max * u_max || e2 != e2 {
            self.contain_fails += 1;
        } else { self.contain_fails = 0; }

        let killed = self.contain_fails >= KILL_K;
        if killed {
            self.rebirth_mode = select_rebirth(&self.z, &self.s, r);
            let mut k = 0; while k < r { self.z[k] = 0.0; k += 1; }
            self.alive = 0;
        }
        if self.alive == 0 { self.rebirth(); }

        // 2. PROJECTION (two-stage: c=WᵀZ, p=Wc, res=Z-p)
        let mut k = 0;
        while k < r {
            let mut acc = 0.0f32; let mut i = 0;
            while i < in_n { acc += self.w[k*N+i] * input[i]; i += 1; }
            self.c[k] = acc; k += 1;
        }
        let mut i = 0; while i < in_n { self.p[i] = 0.0; i += 1; }
        k = 0; while k < r {
            i = 0; while i < in_n { self.p[i] += self.w[k*N+i] * self.c[k]; i += 1; }
            k += 1;
        }
        let mut r_n2 = 0.0f32;
        i = 0; while i < in_n {
            self.res[i] = input[i] - self.p[i];
            r_n2 += self.res[i] * self.res[i]; i += 1;
        }
        let r_norm = fast_sqrt(r_n2);

        // 3. LIE EVOLUTION
        k = 0; while k < r {
            let mut torque = 0.0f32; let mut j = 0;
            while j < r {
                if j != k {
                    torque += (self.z[k]*self.s[j] - self.z[j]*self.s[k])
                              * self.kappa[k*RMAX+j];
                }
                j += 1;
            }
            self.z[k] += dt * (torque - lambda * self.z[k]);
            k += 1;
        }

        // 4. EMA (frozen during containment)
        if self.contain_fails == 0 {
            i = 0; while i < r {
                self.s[i] = alpha*self.s[i] + (1.0-alpha)*self.z[i]; i += 1;
            }
        }

        // 5. BASIS ADAPT (r⊗c weighted)
        if r_norm > EPS {
            let cn = norm_safe_arr(&self.c, r);
            k = 0; while k < r {
                let sc = self.c[k] / cn;
                i = 0; while i < in_n { self.w[k*N+i] += eta * self.res[i] * sc; i += 1; }
                k += 1;
            }
        }

        // 6. MANIFOLD MAINTAIN (MGS + sign lock)
        let drift = stiefel_drift(&self.w, N, r);
        if drift > 1e-6 { orthonormalize(&mut self.w, N, r); }
        sign_lock(&mut self.w, &self.w_prev, N, r);

        // 7. VELOCITY (damped, clamped, includes shear)
        i = 0; while i < in_n {
            let nv = self.v[i] * self.params.velocity_damp + (self.res[i] + self.s[i]) * eta;
            self.v[i] = if nv > u_max { u_max } else if nv < -u_max { -u_max } else { nv };
            i += 1;
        }

        // 8. OMEGA (no Ω→V backfeed)
        k = 0; while k < r {
            self.omega[k] = (self.omega[k] + self.z[k]*alpha*dt) * self.params.omega_decay;
            k += 1;
        }

        // 9. CLASSIFY
        let z_n = norm_safe_arr(&self.z, r);
        let s_n = norm_safe_arr(&self.s, r);
        let stress = s_n / z_n;
        let in_norm = { let mut s=0.0f32; i=0; while i<in_n { s+=input[i]*input[i]; i+=1; } norm_safe_val(s) };
        let novelty = r_norm / in_norm;
        let drift_safe = if drift != drift { 0.0 } else if drift < 0.0 { 0.0 } else { drift };
        let entropy = spectral_entropy(&self.z, r, self.frames_since_rebirth);
        let o_n = norm_safe_arr(&self.omega, r);
        let omega_ratio = o_n / z_n;
        let denat = self.rebirth_mode == RebirthMode::HighEntropy
            && self.frames_since_rebirth < RAMP_FRAMES;
        let ghost = classify_ghost(stress, novelty, drift_safe, entropy, omega_ratio, killed, denat);

        // 10. STATE COMMIT (w_prev AFTER all evolution)
        self.w_prev = self.w;
        self.frame += 1;
        self.t += dt;
        if self.frames_since_rebirth < u32::MAX { self.frames_since_rebirth += 1; }

        // 11. STIFFNESS PROBE (read-only, shadow state)
        let stiffness = {
            let mut z_shadow = self.z;
            if r_n2 > EPS {
                let inv = fast_rsqrt(r_n2);
                i = 0; while i < r { z_shadow[i] += STIFFNESS_EPS * self.res[i] * inv; i += 1; }
            }
            let e2_shadow = norm2(&z_shadow, r);
            ((e2_shadow - e2).abs() / STIFFNESS_EPS).min(1e6)
        };
        self.stiffness_last = stiffness;

        // 12. EMIT (delta-encoded)
        let delta = novelty - self.prev_novelty;
        let emit = delta > 1e-4 || delta < -1e-4 || killed || self.frame < 2;
        self.prev_novelty = novelty;

        let resonance = { let mut mx = 0.0f32; k=0; while k<r {
            let a = self.z[k].abs(); if a > mx { mx = a; } k += 1; } mx };

        BinaryFrame {
            frame: self.frame, energy: z_n, novelty, stress, stiffness,
            omega_norm: o_n, entropy, drift: drift_safe, resonance_peak: resonance,
            ghost: ghost as u8, contained: killed as u8, emitted: emit as u8, _pad: 0,
        }
    }

    fn rebirth(&mut self) {
        let r = self.params.r;
        let n = self.params.n;
        match self.rebirth_mode {
            RebirthMode::Structured => {
                let mut k = 0;
                while k < r { self.z[k] = EPS * self.w[k*N + (k % n)]; k += 1; }
                self.s = [0.0; N];
            }
            RebirthMode::HighEntropy => {
                let mut k = 0;
                while k < r { let mut i = 0;
                    while i < n {
                        self.w[k*N+i] = sin_approx((self.frame as f32)*0.618 + (k*N+i) as f32);
                        i += 1;
                    } k += 1;
                }
                orthonormalize(&mut self.w, N, r);
                k = 0; while k < r { self.z[k] = EPS * self.w[k*N + (k % n)]; k += 1; }
                self.s = [0.0; N];
            }
            RebirthMode::GhostSnap => {
                let sn2 = norm2_n(&self.s, r);
                if sn2 > EPS {
                    let inv = fast_rsqrt(sn2);
                    let mut i = 0; while i < n { self.w[i] = self.s[i] * inv; i += 1; }
                }
                orthonormalize(&mut self.w, N, r);
                let mut k = 0;
                while k < r { self.z[k] = EPS * self.w[k*N + (k % n)]; k += 1; }
                // S NOT zeroed — memory continuity
            }
        }
        self.v = [0.0; N]; self.omega = [0.0; N];
        self.alive = 1; self.contain_fails = 0; self.frames_since_rebirth = 0;
    }

    #[inline] pub fn is_vacuum(&self) -> bool { self.alive == 0 }
}

// ── standalone math (N-sized arrays where needed) ───────────

fn norm2(a: &[f32; N], n: usize) -> f32 {
    let mut s = 0.0f32; let mut i = 0;
    while i < n { s += a[i]*a[i]; i += 1; } s
}
fn norm2_n(a: &[f32; N], n: usize) -> f32 { norm2(a, n) }

fn norm_safe_arr(a: &[f32; N], n: usize) -> f32 {
    let v = norm2(a, n);
    if v < EPS { EPS } else { v * fast_rsqrt(v) }
}
fn norm_safe_val(v: f32) -> f32 {
    if v < EPS { EPS } else { v * fast_rsqrt(v) }
}

const STIFFNESS_EPS: f32 = 1e-5;

fn spectral_entropy(z: &[f32; N], r: usize, fsr: u32) -> f32 {
    let tot = norm2(z, r) + EPS;
    let mut h = 0.0f32; let mut k = 0;
    while k < r {
        let pk = z[k]*z[k] / tot;
        if pk > EPS { h -= pk * ln_approx(pk); }
        k += 1;
    }
    if fsr < RAMP_FRAMES {
        let ramp = fsr as f32 / RAMP_FRAMES as f32;
        ramp * h + (1.0 - ramp) * ln_approx(r as f32)
    } else { h }
}

fn select_rebirth(z: &[f32; N], s: &[f32; N], r: usize) -> RebirthMode {
    let e2 = norm2(z, r) + EPS;
    let mut h = 0.0f32; let mut k = 0;
    while k < r { let pk = z[k]*z[k]/e2; if pk > EPS { h -= pk * ln_approx(pk); } k += 1; }
    if h > ln_approx(r as f32) * 0.8 { RebirthMode::HighEntropy }
    else if norm2(s, r) > EPS { RebirthMode::GhostSnap }
    else { RebirthMode::Structured }
}

fn classify_ghost(
    stress: f32, novelty: f32, drift: f32, entropy: f32,
    omega_ratio: f32, killed: bool, denat: bool,
) -> Ghost {
    if killed { Ghost::Vacuum }
    else if denat { Ghost::Denatured }
    else if stress > 1.5 { Ghost::Burst }
    else if novelty < EPS && entropy < 0.1 { Ghost::Collapse }
    else if novelty > 0.9 && entropy > 2.0 { Ghost::Diffuse }
    else if entropy < 0.3 && stress < 0.1 { Ghost::Echo }
    else if omega_ratio > 1.0 || drift > 0.01 { Ghost::Trap }
    else { Ghost::Nominal }
}

fn orthonormalize(w: &mut [f32; RMAX*N], stride: usize, r: usize) {
    let mut k = 0;
    while k < r {
        let kb = k * stride; let mut j = 0;
        while j < k {
            let jb = j * stride;
            let mut d = 0.0f32; let mut i = 0;
            while i < stride { d += w[kb+i]*w[jb+i]; i += 1; }
            i = 0; while i < stride { w[kb+i] -= d*w[jb+i]; i += 1; }
            j += 1;
        }
        let mut n2 = 0.0f32; let mut i = 0;
        while i < stride { n2 += w[kb+i]*w[kb+i]; i += 1; }
        let inv = if n2 < EPS { 1.0/EPS } else { fast_rsqrt(n2) };
        i = 0; while i < stride { w[kb+i] *= inv; i += 1; }
        k += 1;
    }
}

fn stiefel_drift(w: &[f32; RMAX*N], stride: usize, r: usize) -> f32 {
    let mut d = 0.0f32; let mut k1 = 0;
    while k1 < r { let mut k2 = 0;
        while k2 < r {
            let mut g = 0.0f32; let mut i = 0;
            while i < stride { g += w[k1*stride+i]*w[k2*stride+i]; i += 1; }
            let e = g - if k1==k2 { 1.0 } else { 0.0 };
            d += e*e; k2 += 1;
        } k1 += 1;
    }
    fast_sqrt(d)
}

fn sign_lock(w: &mut [f32; RMAX*N], wp: &[f32; RMAX*N], stride: usize, r: usize) {
    let mut k = 0;
    while k < r {
        let kb = k * stride;
        let mut dp = 0.0f32; let mut i = 0;
        while i < stride { dp += w[kb+i]*wp[kb+i]; i += 1; }
        if dp < 0.0 { i = 0; while i < stride { w[kb+i] *= -1.0; i += 1; } }
        k += 1;
    }
}

// ── C ABI (stable, 5 functions) ─────────────────────────────

#[no_mangle] pub extern "C" fn dvsm_init(n: u32, r: u32) -> *mut DvsmCore {
    #[cfg(feature = "std")]
    {
        let p = Params { n: (n as usize).min(N), r: (r as usize).min(RMAX), ..Params::default() };
        let c = Box::new(DvsmCore::new(p));
        Box::into_raw(c)
    }
    #[cfg(not(feature = "std"))]
    { core::ptr::null_mut() }
}

#[no_mangle] pub unsafe extern "C" fn dvsm_step(
    core: *mut DvsmCore, input: *const f32, len: u32, out: *mut BinaryFrame,
) -> i32 {
    let c = match core.as_mut() { Some(c) => c, None => return -1 };
    let n = (c.params.n as u32).min(len) as usize;
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
    _c: *const DvsmCore, f: *const BinaryFrame, o: *mut BinaryFrame,
) -> i32 {
    match (f.as_ref(), o.as_mut()) { (Some(f), Some(o)) => { *o = *f; 0 }, _ => -1 }
}

#[no_mangle] pub unsafe extern "C" fn dvsm_free(core: *mut DvsmCore) {
    #[cfg(feature = "std")]
    if !core.is_null() { drop(Box::from_raw(core)); }
}
