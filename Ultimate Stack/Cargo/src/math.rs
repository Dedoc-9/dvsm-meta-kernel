//! ============================================================
//! DVSM-π+++ / DQSDv2 · math.rs
//! Spectral Math Kernel + Procedural κ Generator
//! author: Daniel J. Dillberg
//! Contact: Bigdillly95@gmail.com
//! ============================================================
//! Spectral Math Kernel + κ Field System
//! ============================================================

#![allow(dead_code)]

pub const EPS: f32 = 1e-8;
pub const KAPPA_SCALE_A: f32 = 1.37;
pub const KAPPA_SCALE_B: f32 = 1.73;

// ============================================================
// DOT / NORM CORE
// ============================================================

#[inline(always)]
pub fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
    let mut s = 0.0;
    let mut i = 0;

    while i < n {
        s += a[i] * b[i];
        i += 1;
    }

    s
}

#[inline(always)]
pub fn norm2(a: &[f32], n: usize) -> f32 {
    dot(a, a, n)
}

#[inline(always)]
pub fn norm_safe(a: &[f32], n: usize) -> f32 {
    norm2(a, n).sqrt().max(EPS)
}

#[inline(always)]
pub fn normalize(v: &mut [f32], n: usize) {
    let inv = 1.0 / norm_safe(v, n);

    let mut i = 0;
    while i < n {
        v[i] *= inv;
        i += 1;
    }
}

// ============================================================
// κ FIELD (ANALYTIC)
// ============================================================
//
// Antisymmetric deterministic coupling:
// κ(i,j) = sin(iA - jB)
// ============================================================

#[inline(always)]
pub fn kappa(i: usize, j: usize) -> f32 {
    if i == j {
        return 0.0;
    }

    let x = (i as f32 * KAPPA_SCALE_A) - (j as f32 * KAPPA_SCALE_B);
    x.sin()
}

// ============================================================
// PRECOMPUTED κ MATRIX (OPTIONAL CACHE)
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
// Notes:
// - uses snapshot of Z for stability
// - avoids in-place contamination
// ============================================================

#[inline(always)]
pub fn evolve_lie<const R: usize>(
    z: &mut [f32; R],
    s: &[f32; R],
    kappa_table: Option<&[f32; R * R]>,
    dt: f32,
    lambda: f32,
    r: usize,
) {
    let z_prev = *z;

    let use_table = kappa_table.is_some();

    let table = kappa_table.unwrap_or(&[[0.0; R * R]; 1][0]);

    let mut k = 0;
    while k < r {
        let mut acc = 0.0;

        let mut j = 0;
        while j < r {
            if j != k {
                let κ = if use_table {
                    table[k * R + j]
                } else {
                    kappa(k, j)
                };

                acc += (z_prev[k] * s[j] - z_prev[j] * s[k]) * κ;
            }
            j += 1;
        }

        z[k] = z_prev[k] + dt * (acc - lambda * z_prev[k]);
        k += 1;
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
    let mut i = 0;

    while i < r {
        s[i] = alpha * s[i] + (1.0 - alpha) * z[i];
        i += 1;
    }
}

// ============================================================
// FROBENIUS ORTHOGONALITY ERROR
// ============================================================
//
// ||WᵀW - I||_F
// ============================================================

pub fn frobenius_ortho_error<const R: usize>(
    w: &[f32; R * R],
    r: usize,
) -> f32 {
    let mut err = 0.0;

    let mut i = 0;
    while i < r {

        let mut j = 0;
        while j < r {

            let mut d = 0.0;

            let mut k = 0;
            while k < r {
                d += w[i * R + k] * w[j * R + k];
                k += 1;
            }

            let target = if i == j { 1.0 } else { 0.0 };
            let delta = d - target;

            err += delta * delta;

            j += 1;
        }

        i += 1;
    }

    err.sqrt()
}
