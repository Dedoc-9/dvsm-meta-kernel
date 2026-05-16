// src/containment.rs — kill/rebirth/denaturation
use crate::constants::*;
use crate::math::*;
use crate::manifold::orthonormalize;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RebirthMode { Structured=0, HighEntropy=1, GhostSnap=2 }

pub fn check_containment(z: &[f32; R], r: usize) -> bool {
    let e2 = norm2(z, r);
    e2 > U_MAX2 || e2 != e2 // NaN: e2 != e2
}

pub fn select_rebirth(z: &[f32; R], s: &[f32; R], r: usize) -> RebirthMode {
    let e2 = norm2(z, r) + EPS;
    let mut h = 0.0f32; let mut k = 0;
    while k < r {
        let pk = z[k]*z[k]/e2;
        if pk > EPS { h -= pk * ln_approx(pk); }
        k += 1;
    }
    if h > ln_approx(r as f32) * 0.8 { RebirthMode::HighEntropy }
    else if norm2(s, r) > EPS        { RebirthMode::GhostSnap }
    else                              { RebirthMode::Structured }
}

pub fn rebirth(
    z: &mut [f32; R], s: &mut [f32; R], v: &mut [f32; R], omega: &mut [f32; R],
    w: &mut [f32; R2], mode: RebirthMode, frame: u64, r: usize,
) {
    match mode {
        RebirthMode::Structured => {
            let mut k = 0;
            while k < r { z[k] = EPS * w[k*R + (k % R)]; k += 1; }
        }
        RebirthMode::HighEntropy => {
            let mut k = 0;
            while k < r { let mut i = 0;
                while i < R {
                    w[k*R+i] = sin_approx((frame as f32)*0.618 + (k*R+i) as f32);
                    i += 1;
                } k += 1;
            }
            orthonormalize(w, r);
            let mut k = 0;
            while k < r { z[k] = EPS * w[k*R + (k % R)]; k += 1; }
        }
        RebirthMode::GhostSnap => {
            let s_n2 = norm2(s, r);
            if s_n2 > EPS {
                let inv = fast_rsqrt(s_n2);
                let mut i = 0;
                while i < R { w[i] = s[i] * inv; i += 1; }
            }
            orthonormalize(w, r);
            let mut k = 0;
            while k < r { z[k] = EPS * w[k*R + (k % R)]; k += 1; }
            // S preserved in GhostSnap — do NOT zero
            *v = [0.0; R]; *omega = [0.0; R];
            return;
        }
    }
    *s = [0.0; R]; *v = [0.0; R]; *omega = [0.0; R];
}
