// src/manifold.rs — Stiefel geometry ops
use crate::constants::*;
use crate::math::*;

pub fn orthonormalize(w: &mut [f32; R2], r: usize) {
    let mut k = 0;
    while k < r {
        let kb = k * R;
        let mut j = 0;
        while j < k {
            let jb = j * R;
            let mut d = 0.0f32; let mut i = 0;
            while i < R { d += w[kb+i] * w[jb+i]; i += 1; }
            i = 0;
            while i < R { w[kb+i] -= d * w[jb+i]; i += 1; }
            j += 1;
        }
        let mut n2 = 0.0f32; let mut i = 0;
        while i < R { n2 += w[kb+i] * w[kb+i]; i += 1; }
        let inv = if n2 < EPS { 1.0 / EPS } else { fast_rsqrt(n2) };
        i = 0;
        while i < R { w[kb+i] *= inv; i += 1; }
        k += 1;
    }
}

pub fn stiefel_drift(w: &[f32; R2], r: usize) -> f32 {
    let mut d = 0.0f32;
    let mut k1 = 0;
    while k1 < r { let mut k2 = 0;
        while k2 < r {
            let mut g = 0.0f32; let mut i = 0;
            while i < R { g += w[k1*R+i] * w[k2*R+i]; i += 1; }
            let e = g - if k1==k2 { 1.0 } else { 0.0 };
            d += e * e; k2 += 1;
        } k1 += 1;
    }
    fast_sqrt(d)
}

pub fn sign_lock(w: &mut [f32; R2], w_prev: &[f32; R2], n: usize, r: usize) {
    let mut k = 0;
    while k < r {
        let kb = k * R;
        let mut dp = 0.0f32; let mut i = 0;
        while i < n { dp += w[kb+i] * w_prev[kb+i]; i += 1; }
        if dp < 0.0 { i = 0; while i < n { w[kb+i] *= -1.0; i += 1; } }
        k += 1;
    }
}
