//! ============================================================
//! DVSM-π+++ / DQSDv2 · math.rs
//! Spectral Math Kernel + Procedural κ Generator
//! author: Daniel J. Dillberg
//! Contact: Bigdillly95@gmail.com
//! ============================================================
//
// PURPOSE:
// Core deterministic arithmetic layer for:
//   - Lie-bracket evolution
//   - Stiefel manifold maintenance
//   - Orthogonality enforcement
//   - Procedural κ-field generation
//
// DESIGN:
//   - no_std-safe arithmetic
//   - allocation-free
//   - deterministic execution order
//   - SIMD-friendly contiguous memory layout
//
// ============================================================

#![allow(dead_code)]

pub const EPS: f32 = 1e-8;
pub const KAPPA_SCALE_A: f32 = 1.37;
pub const KAPPA_SCALE_B: f32 = 1.73;

// ============================================================
// DOT PRODUCT
// ============================================================

#[inline(always)]
pub fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
    let mut s = 0.0f32;

    for i in 0..n {
        s += a[i] * b[i];
    }

    s
}

// ============================================================
// L2 NORM²
// ============================================================

#[inline(always)]
pub fn norm2(a: &[f32], n: usize) -> f32 {
    dot(a, a, n)
}

// ============================================================
// SAFE NORM
// ============================================================

#[inline(always)]
pub fn norm_safe(a: &[f32], n: usize) -> f32 {
    norm2(a, n).sqrt().max(EPS)
}

// ============================================================
// NORMALIZE VECTOR
// ============================================================

#[inline(always)]
pub fn normalize(v: &mut [f32], n: usize) {
    let inv = 1.0 / norm_safe(v, n);

    for i in 0..n {
        v[i] *= inv;
    }
}

// ============================================================
// PROCEDURAL κ FIELD GENERATOR
// ============================================================
//
// κ(i,j) defines the non-normal coupling field used by
// the Lie-bracket evolution kernel.
//
// PROPERTIES:
//   - deterministic
//   - antisymmetric
//   - allocation-free
//   - no RNG dependency
//
// κ(i,j) = sin(iA - jB)
//
// ============================================================

#[inline(always)]
pub fn kappa(i: usize, j: usize) -> f32 {
    if i == j {
        return 0.0;
    }

    (
        i as f32 * KAPPA_SCALE_A
        - j as f32 * KAPPA_SCALE_B
    ).sin()
}

// ============================================================
// PRECOMPUTE κ MATRIX
// ============================================================
//
// Layout:
//   κ[i * r + j]
//
// Called once during init.
//
// ============================================================

pub fn generate_kappa<const R: usize>() -> [f32; R * R] {
    let mut out = [0.0f32; R * R];

    let mut i = 0;

    while i < R {
        let mut j = 0;

        while j < R {
            out[i * R + j] = kappa(i, j);
            j += 1;
        }

        i += 1;
    }

    out
}

// ============================================================
// LIE BRACKET EVOLUTION
// ============================================================
//
// dZ/dt = [Z,S]_κ - λZ
//
// ============================================================

#[inline(always)]
pub fn evolve_lie<const R: usize>(
    z: &mut [f32; R],
    s: &[f32; R],
    kappa_table: &[f32; R * R],
    dt: f32,
    lambda: f32,
    r: usize,
) {
    let prev = *z;

    for k in 0..r {

        let mut acc = 0.0f32;

        for j in 0..r {

            if j == k {
                continue;
            }

            let κ = kappa_table[k * R + j];

            acc += (
                prev[k] * s[j]
                - prev[j] * s[k]
            ) * κ;
        }

        z[k] += dt * (acc - lambda * z[k]);
    }
}

// ============================================================
// EMA UPDATE
// ============================================================

#[inline(always)]
pub fn ema_update(
    s: &mut [f32],
    z: &[f32],
    alpha: f32,
    r: usize,
) {
    for i in 0..r {
        s[i] =
            alpha * s[i]
            + (1.0 - alpha) * z[i];
    }
}

// ============================================================
// FROBENIUS ORTHOGONALITY ERROR
// ============================================================
//
// ||WᵀW - I||_F
//
// ============================================================

pub fn frobenius_ortho_error<const R: usize>(
    w: &[f32; R * R],
    r: usize,
) -> f32 {

    let mut err = 0.0f32;

    for i in 0..r {

        for j in 0..r {

            let mut d = 0.0f32;

            for k in 0..r {
                d += w[i * R + k] * w[j * R + k];
            }

            let target =
                if i == j { 1.0f32 }
                else { 0.0f32 };

            let delta = d - target;

            err += delta * delta;
        }
    }

    err.sqrt()
}
