// src/math.rs — no_std, no libm, SIMD-auto-vectorizable
// All numerical primitives for the 11-stage pipeline.
// Every loop is stride-1 while-indexed for autovectorization.
use crate::constants::*;

// ── dot / norm ──────────────────────────────────────────────

#[inline(always)]
pub fn dot(a: &[f32; R], b: &[f32; R], n: usize) -> f32 {
    let mut s = 0.0f32; let mut i = 0;
    while i < n { s += a[i] * b[i]; i += 1; }
    s
}

#[inline(always)]
pub fn norm2(a: &[f32; R], n: usize) -> f32 { dot(a, a, n) }

#[inline(always)]
pub fn fast_rsqrt(v: f32) -> f32 {
    let x = f32::from_bits(0x5f37_5a86 - (v.to_bits() >> 1));
    x * (1.5 - 0.5 * v * x * x)
}

#[inline(always)]
pub fn fast_sqrt(v: f32) -> f32 {
    if v < EPS { return 0.0; }
    v * fast_rsqrt(v)
}

#[inline(always)]
pub fn norm_safe(a: &[f32; R], n: usize) -> f32 {
    let v = norm2(a, n);
    if v < EPS { EPS } else { v * fast_rsqrt(v) }
}

#[inline(always)]
pub fn norm_safe_val(v: f32) -> f32 {
    if v < EPS { EPS } else { v * fast_rsqrt(v) }
}

// ── transcendentals (no libm) ───────────────────────────────

/// Bhaskara I sin. Max error 0.18%.
#[inline(always)]
pub fn sin_approx(x: f32) -> f32 {
    let pi = core::f32::consts::PI;
    let x = x - (x / (2.0 * pi)).floor() * 2.0 * pi;
    let x = if x > pi { x - 2.0 * pi } else { x };
    let num = 16.0 * x * (pi - x.abs());
    let den = 5.0 * pi * pi - 4.0 * x.abs() * (pi - x.abs());
    num / den
}

/// Bit-cast ln. No libm.
#[inline(always)]
pub fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 { return -20.0; }
    let bits = x.to_bits() as i32;
    let exp = ((bits >> 23) & 0xff) - 127;
    let frac = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);
    let log2 = exp as f32 + (frac - 1.0) * (2.0 - 0.333 * (frac - 1.0));
    log2 * 0.693_147_2
}

// ── step 2: projection  c=WᵀZ; p=Wc; R=Z−p ────────────────

/// Two-stage projection. Returns ‖res‖.
#[inline]
pub fn project(
    w: &[f32; R2], input: &[f32], in_n: usize, r: usize,
    c: &mut [f32; R], p: &mut [f32; R], res: &mut [f32; R],
) -> f32 {
    let mut k = 0;
    while k < r {
        let mut s = 0.0f32; let mut i = 0;
        while i < in_n { s += w[k*R+i] * input[i]; i += 1; }
        c[k] = s; k += 1;
    }
    let mut i = 0; while i < in_n { p[i] = 0.0; i += 1; }
    k = 0; while k < r {
        i = 0; while i < in_n { p[i] += w[k*R+i] * c[k]; i += 1; }
        k += 1;
    }
    let mut r_n2 = 0.0f32;
    i = 0; while i < in_n {
        res[i] = input[i] - p[i];
        r_n2 += res[i] * res[i]; i += 1;
    }
    fast_sqrt(r_n2)
}

// ── step 3: Lie-bracket  Z += dt·([Z,S]_κ − λZ) ────────────
// INVARIANT: κ antisymmetric → d‖Z‖²/dt = −2λ‖Z‖²

#[inline]
pub fn lie_step(z: &mut [f32; R], s: &[f32; R], kappa: &[f32; R2], r: usize) {
    let mut k = 0;
    while k < r {
        let mut acc = 0.0f32; let mut j = 0;
        while j < r {
            if j != k { acc += (z[k]*s[j] - z[j]*s[k]) * kappa[k*R+j]; }
            j += 1;
        }
        z[k] += DT * (acc - LAMBDA * z[k]);
        k += 1;
    }
}

// ── step 4: EMA  S = αS + (1−α)Z ───────────────────────────

#[inline]
pub fn ema_update(s: &mut [f32; R], z: &[f32; R], r: usize) {
    let mut k = 0;
    while k < r { s[k] = ALPHA*s[k] + (1.0-ALPHA)*z[k]; k += 1; }
}

// ── step 5: basis adapt  W += η·res⊗(c/‖c‖) ───────────────

#[inline]
pub fn basis_adapt(
    w: &mut [f32; R2], res: &[f32; R], c: &[f32; R],
    in_n: usize, r: usize, r_norm: f32,
) {
    if r_norm <= EPS { return; }
    let cn = norm_safe(c, r);
    let mut k = 0;
    while k < r {
        let sc = c[k] / cn; let mut i = 0;
        while i < in_n { w[k*R+i] += ETA * res[i] * sc; i += 1; }
        k += 1;
    }
}

// ── step 7: velocity  V=clamp(V·γ+(R+S)·η); X+=V·dt ───────

#[inline]
pub fn velocity_update(
    v: &mut [f32; R], x: &mut [f32; R],
    res: &[f32; R], s: &[f32; R], in_n: usize,
) {
    let mut i = 0;
    while i < in_n {
        let nv = v[i]*DAMPING + (res[i]+s[i])*ETA;
        v[i] = if nv > U_MAX { U_MAX } else if nv < -U_MAX { -U_MAX } else { nv };
        x[i] += v[i] * DT;
        i += 1;
    }
}

// ── step 8: omega  Ω = (Ω + Z·α·dt)·decay ─────────────────
// INVARIANT: no Ω→V backfeed

#[inline]
pub fn omega_update(omega: &mut [f32; R], z: &[f32; R], r: usize) {
    let mut k = 0;
    while k < r { omega[k] = (omega[k] + z[k]*ALPHA*DT) * OMEGA_DECAY; k += 1; }
}

// ── step 9: spectral entropy  H = −Σ p_k ln p_k ────────────
// Includes H6 rebirth ramp: blend toward ln(R) during first RAMP_FRAMES

#[inline]
pub fn spectral_entropy(z: &[f32; R], r: usize, frames_since_rebirth: u32) -> f32 {
    let tot = norm2(z, r) + EPS;
    let mut h = 0.0f32; let mut k = 0;
    while k < r {
        let pk = z[k]*z[k] / tot;
        if pk > EPS { h -= pk * ln_approx(pk); }
        k += 1;
    }
    if frames_since_rebirth < RAMP_FRAMES {
        let ramp = frames_since_rebirth as f32 / RAMP_FRAMES as f32;
        ramp * h + (1.0 - ramp) * ln_approx(r as f32)
    } else { h }
}

// ── step 10: state commit  W_prev ← W; frame += 1 ──────────
// MUST execute AFTER steps 5-9 (all evolution complete).
// MUST execute BEFORE step 11 (trace reads committed frame).
// sign_lock in step 6 of the NEXT cycle reads w_prev set here.

#[inline]
pub fn state_commit(
    w_prev: &mut [f32; R2], w: &[f32; R2],
    frame: &mut u64, frames_since_rebirth: &mut u32,
) {
    *w_prev = *w;
    *frame += 1;
    if *frames_since_rebirth < u32::MAX { *frames_since_rebirth += 1; }
}

// ── hash (portable, deterministic) ──────────────────────────

pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut i = 0;
    while i < data.len() { h ^= data[i] as u64; h = h.wrapping_mul(0x100000001b3); i += 1; }
    h
}
