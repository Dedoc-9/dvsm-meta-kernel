// src/math.rs — no_std, no libm, SIMD-auto-vectorizable
use crate::constants::*;

#[inline(always)]
pub fn dot(a: &[f32; R], b: &[f32; R], n: usize) -> f32 {
    let mut s = 0.0f32;
    let mut i = 0;
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

// Bhaskara I sin: max error 0.18%, no libm
#[inline(always)]
pub fn sin_approx(x: f32) -> f32 {
    let pi = core::f32::consts::PI;
    let x = x - (x / (2.0 * pi)).floor() * 2.0 * pi;
    let x = if x > pi { x - 2.0 * pi } else { x };
    let num = 16.0 * x * (pi - x.abs());
    let den = 5.0 * pi * pi - 4.0 * x.abs() * (pi - x.abs());
    num / den
}

// Bit-cast ln: no libm
#[inline(always)]
pub fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 { return -20.0; }
    let bits = x.to_bits() as i32;
    let exp = ((bits >> 23) & 0xff) - 127;
    let frac = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);
    let log2 = exp as f32 + (frac - 1.0) * (2.0 - 0.333 * (frac - 1.0));
    log2 * 0.693_147_2
}

// FNV-1a 64: deterministic portable hash
pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut i = 0;
    while i < data.len() { h ^= data[i] as u64; h = h.wrapping_mul(0x100000001b3); i += 1; }
    h
}
