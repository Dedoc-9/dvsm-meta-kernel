// dvsm-core/src/v17r_render.rs
// V17-R · Deterministic Manifold Projection → Render Layer
// Author: Daniel J. Dillberg · License: (AGPL-3.0) contact: Bigdilly95@gmail.com
// Use with NextGenStack.rs
//
// CLASSIFICATION: Computational Visualization System
//   NOT physics, NOT optics, NOT holography, NOT wave synthesis.
//   IS: latent state → RGB/depth/curvature projection operator.
//
// ARITHMETIC:
//   Π_render : (Z, S, W, Ω, R) → RenderFrame
//   Pure function. No state mutation. No feedback to core.
//
//   Core equation (external, drives Z):
//     dZ/dt = [Z, S]_κ − λZ
//     d‖Z‖²/dt = −2λ‖Z‖²
//
//   Render projections (this file):
//     stress      = ‖S‖ / ‖Z‖             [memory-to-field ratio]
//     novelty     = ‖R‖ / ‖input‖         [unexplained energy fraction]
//     entropy     = −Σ p_k ln p_k          [Shannon, NOT thermodynamic]
//     stiffness   = |Δ‖Z‖²/Δε|            [finite-difference sensitivity]
//     curvature   = (max_k ‖w_k‖² − min_k ‖w_k‖²) / max  [basis anisotropy]
//     resonance   = max_k |Z_k|            [peak mode amplitude]
//     depth       = ‖Z‖ / U_MAX           [normalized field energy]
//     RGB         = f(mode, diagnostics)   [semantic encoding, NOT light]
//
// HOOKS TO CORE:
//   Step 2 → R (residual) feeds novelty + stiffness probe direction
//   Step 3 → Z (post Lie evolution) feeds all energy-based metrics
//   Step 4 → S (EMA memory) feeds stress
//   Step 7 → Ω (drift accumulator) feeds omega render mode
//   Step 9 → W_prev committed before next render reads W
//
// HASH: RenderFrame includes frame_id for trace correlation.
//   Use fnv1a(rgb bytes) for cross-platform render parity checks.
//
// PORTING:
//   C:     call v17r_render(state_ptr, mode, &out)
//   Unity: P/Invoke v17r_render, read out.rgb[3] for material tint
//   UE5:   bind to RDG pass output, feed rgb to post-process material
//   WASM:  compile with --target wasm32, call from JS typed array view

#![cfg_attr(not(feature = "std"), no_std)]

pub const RMAX: usize = 16;
pub const N: usize = 256;
pub const EPS: f32 = 1e-8;
pub const STIFFNESS_EPS: f32 = 1e-5;
pub const RENDER_VERSION: u32 = 1; // bump on RenderFrame schema change

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum RenderMode {
    Spectral=0, Stress=1, Stiffness=2,
    Entropy=3, Novelty=4, Omega=5,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RenderFrame {
    pub frame_id: u64,
    pub rgb: [f32; 3],
    pub depth: f32,
    pub curvature_proxy: f32,
    pub stiffness: f32,
    pub entropy: f32,
    pub resonance_peak: f32,
    pub novelty: f32,
    pub stress: f32,
    pub render_mode: u8,
    pub version: u8,       // RENDER_VERSION for backward compat
    _pad: [u8; 2],
}

impl Default for RenderFrame {
    fn default() -> Self {
        Self {
            frame_id:0, rgb:[0.0;3], depth:0.0, curvature_proxy:0.0,
            stiffness:0.0, entropy:0.0, resonance_peak:0.0, novelty:0.0,
            stress:0.0, render_mode:0, version:RENDER_VERSION as u8, _pad:[0;2],
        }
    }
}

// ── math (no libm, SIMD-friendly) ───────────────────────────

#[inline(always)] fn fast_rsqrt(v: f32) -> f32 {
    let x = f32::from_bits(0x5f37_5a86 - (v.to_bits() >> 1));
    x * (1.5 - 0.5 * v * x * x)
}
#[inline(always)] fn fast_sqrt(v: f32) -> f32 {
    if v < EPS { 0.0 } else { v * fast_rsqrt(v) }
}
#[inline(always)] fn norm2_slice(a: &[f32], n: usize) -> f32 {
    let mut s = 0.0f32; let mut i = 0;
    let len = n.min(a.len()); // bounds safety
    while i < len { s += a[i]*a[i]; i += 1; }
    s
}
#[inline(always)] fn norm_safe_slice(a: &[f32], n: usize) -> f32 {
    let v = norm2_slice(a, n);
    if v < EPS { EPS } else { v * fast_rsqrt(v) }
}
#[inline(always)] fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 { return -20.0; }
    let b = x.to_bits() as i32;
    let e = ((b >> 23) & 0xff) - 127;
    let f = f32::from_bits((b & 0x007f_ffff) | 0x3f80_0000);
    (e as f32 + (f-1.0)*(2.0-0.333*(f-1.0))) * 0.693_147_2
}

// FNV-1a for render parity hash
pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let mut i = 0;
    while i < data.len() { h ^= data[i] as u64; h = h.wrapping_mul(0x100000001b3); i += 1; }
    h
}

pub fn hash_render_frame(f: &RenderFrame) -> u64 {
    let bytes = unsafe {
        core::slice::from_raw_parts(
            f as *const RenderFrame as *const u8,
            core::mem::size_of::<RenderFrame>()
        )
    };
    fnv1a(bytes)
}

// ── core snapshot (read-only, zero-copy from core) ──────────

#[repr(C)]
pub struct CoreSnapshot<'a> {
    pub z: &'a [f32],
    pub s: &'a [f32],
    pub w: &'a [f32],
    pub omega: &'a [f32],
    pub res: &'a [f32],
    pub r: usize,
    pub n: usize,
    pub frame: u64,
    pub input_norm: f32,
}

// ── render operator (pure projection, no mutation) ──────────

pub fn render(snap: &CoreSnapshot, mode: RenderMode) -> RenderFrame {
    let r = snap.r;
    let n = snap.n;

    // shared diagnostics
    let z_n = norm_safe_slice(snap.z, r);
    let s_n = norm_safe_slice(snap.s, r);
    let r_n = norm_safe_slice(snap.res, n);
    let o_n = norm_safe_slice(snap.omega, r);
    let stress = s_n / z_n;
    let novelty = r_n / snap.input_norm.max(EPS);

    // resonance peak
    let resonance = {
        let mut mx = 0.0f32; let mut k = 0;
        while k < r { let a = if k < snap.z.len() { snap.z[k].abs() } else { 0.0 };
            if a > mx { mx = a; } k += 1; }
        mx
    };

    // entropy (Shannon, not thermodynamic)
    let entropy = {
        let tot = norm2_slice(snap.z, r) + EPS;
        let mut h = 0.0f32; let mut k = 0;
        while k < r {
            let pk = if k < snap.z.len() { snap.z[k]*snap.z[k] / tot } else { 0.0 };
            if pk > EPS { h -= pk * ln_approx(pk); }
            k += 1;
        }
        h
    };

    // curvature proxy (basis energy anisotropy)
    let curvature_proxy = {
        let mut emin = f32::MAX;
        let mut emax = 0.0f32;
        let mut k = 0;
        while k < r {
            let start = k * n;
            let end = start + n;
            // bounds check: slice may be shorter if w is undersized
            if end <= snap.w.len() {
                let ek = norm2_slice(&snap.w[start..end], n);
                if ek < emin { emin = ek; }
                if ek > emax { emax = ek; }
            }
            k += 1;
        }
        if emax < EPS { 0.0 } else { (emax - emin) / (emax + EPS) }
    };

    // stiffness (shadow probe along residual direction)
    let stiffness = {
        let e2 = norm2_slice(snap.z, r);
        let res_r = r.min(snap.res.len()); // res may be n-sized; only use r components
        let r2 = norm2_slice(snap.res, res_r);
        if r2 > EPS {
            let inv = fast_rsqrt(r2);
            let mut e2s = 0.0f32; let mut k = 0;
            while k < r {
                let rk = if k < res_r { snap.res[k] } else { 0.0 };
                let zs = snap.z[k] + STIFFNESS_EPS * rk * inv;
                e2s += zs * zs;
                k += 1;
            }
            ((e2s - e2).abs() / STIFFNESS_EPS).min(1e6)
        } else { 0.0 }
    };

    let depth = (z_n / 100.0).min(1.0);

    // RGB (semantic encoding — NOT physical light)
    let rgb = match mode {
        RenderMode::Spectral => {
            // FIX: safe thirds for any r (including r=1, r=2)
            let t = if r < 3 { 1 } else { r / 3 };
            let m1 = t.min(snap.z.len());
            let m2 = (2*t).min(snap.z.len());
            let m3 = r.min(snap.z.len());
            let lo = norm_safe_slice(&snap.z[..m1], m1);
            let mi = if m2 > m1 { norm_safe_slice(&snap.z[m1..m2], m2-m1) } else { EPS };
            let hi = if m3 > m2 { norm_safe_slice(&snap.z[m2..m3], m3-m2) } else { EPS };
            let mx = lo.max(mi).max(hi).max(EPS);
            [lo/mx, mi/mx, hi/mx]
        }
        RenderMode::Stress => {
            let t = (stress / 2.0).min(1.0);
            [t, 1.0-t, 0.1]
        }
        RenderMode::Stiffness => {
            let t = (stiffness / 100.0).min(1.0);
            [t, t*0.8, 1.0-t]
        }
        RenderMode::Entropy => {
            let max_h = ln_approx(r.max(2) as f32).max(EPS);
            let t = (entropy / max_h).min(1.0);
            [0.5+0.5*t, t, 0.5+0.5*t]
        }
        RenderMode::Novelty => {
            let t = novelty.min(1.0);
            [0.1, t, t*0.8]
        }
        RenderMode::Omega => {
            let t = (o_n / z_n).min(1.0);
            [0.8*t, 0.3*(1.0-t), 1.0-t]
        }
    };

    RenderFrame {
        frame_id: snap.frame, rgb, depth, curvature_proxy,
        stiffness, entropy, resonance_peak: resonance, novelty, stress,
        render_mode: mode as u8, version: RENDER_VERSION as u8, _pad: [0;2],
    }
}

// ── C ABI (binary-safe, pointer-based) ──────────────────────

#[repr(C)]
pub struct V17RState {
    pub z: [f32; RMAX],
    pub s: [f32; RMAX],
    pub w: [f32; RMAX * N],
    pub omega: [f32; RMAX],
    pub res: [f32; N],
    pub r: u32,
    pub n: u32,
    pub frame: u64,
    pub input_norm: f32,
}

/// Render one frame. Returns 0 on success, <0 on error.
#[no_mangle]
pub unsafe extern "C" fn v17r_render(
    state: *const V17RState, mode: u8, out: *mut RenderFrame,
) -> i32 {
    let s = match state.as_ref() { Some(s) => s, None => return -1 };
    let o = match out.as_mut() { Some(o) => o, None => return -2 };
    let r = (s.r as usize).min(RMAX);
    let n = (s.n as usize).min(N);
    let snap = CoreSnapshot {
        z: &s.z[..r], s: &s.s[..r], w: &s.w[..r*n],
        omega: &s.omega[..r], res: &s.res[..n],
        r, n, frame: s.frame, input_norm: s.input_norm,
    };
    let m = match mode {
        0=>RenderMode::Spectral, 1=>RenderMode::Stress,
        2=>RenderMode::Stiffness, 3=>RenderMode::Entropy,
        4=>RenderMode::Novelty, 5=>RenderMode::Omega,
        _=>RenderMode::Spectral,
    };
    *o = render(&snap, m);
    0
}

/// Hash a render frame for cross-platform parity check.
#[no_mangle]
pub unsafe extern "C" fn v17r_hash(frame: *const RenderFrame) -> u64 {
    match frame.as_ref() { Some(f) => hash_render_frame(f), None => 0 }
}

/// Query render schema version.
#[no_mangle]
pub extern "C" fn v17r_version() -> u32 { RENDER_VERSION }
// --------------------------------------------------------------------------------
// Clean V20 render kernel (corrected core) (Diamond Dini)

pub fn render_v20(
    z: &[q64; 16],
    geo: &GeoAddonV20,
    frame_id: u64,
    kill: bool
) -> RenderFrameV20 {

    let energy = reduce_q64_energy(z);
    let ghost  = reduce_q64_energy(&geo.ghost);

    let suchness = sqrt_q64_to_f32(energy);

    // OP5: Stiefel curvature (proper form)
    let kappa_stiefel =
        (geo.w_max - geo.w_min) / (geo.w_max + EPS);

    let curvature_sign =
        if kappa_stiefel > 0.0 { 1.0 } else { -1.0 };

    let klein = geo.klein[0] as f32;
    let rose  = geo.rose[0] as f32;
    let dini  = geo.dini[0] as f32;

    RenderFrameV20 {
        frame_id,
        rgb: [
            klein.tanh(),
            rose.tanh(),
            suchness.min(1.0),
        ],
        suchness_energy: suchness,
        ghost_anomaly: sqrt_q64_to_f32(ghost),
        stiefel_curvature: curvature_sign,
        rf_phase_warp: rose.sin(),
        stability_flag: kill as u8,
        version: 20,
    }
}
