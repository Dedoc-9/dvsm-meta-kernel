// src/math.rs

pub const EPS: f32 = 1e-6;

#[inline(always)]
pub fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
let mut s = 0.0;
for i in 0..n {
s += a[i] * b[i];
}
s
}

#[inline(always)]
pub fn norm2(a: &[f32], n: usize) -> f32 {
dot(a, a, n)
}

#[inline(always)]
pub fn norm(a: &[f32], n: usize) -> f32 {
norm2(a, n).sqrt().max(EPS)
}

#[inline(always)]
pub fn clampf(v: f32, lo: f32, hi: f32) -> f32 {
if v < lo {
lo
} else if v > hi {
hi
} else {
v
}
}
//! add mod math; to your root file and call them as math::dot(...)
