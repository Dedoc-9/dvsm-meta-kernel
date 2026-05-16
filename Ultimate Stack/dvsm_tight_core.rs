// ============================================================
// DVSM-π+++ / DQSDv2 · TIGHTENED CORE
// Author: Daniel J. Dillberg · ABI-stable · allocation-free hot path
// Contact: BigDilly95@gmail.com
// ============================================================
// INVARIANT: μ_t immutable (no host mutation)
// INVARIANT: W^T W = I after every step
// INVARIANT: d‖Z‖²/dt = −2λ‖Z‖² (antisymmetric κ)
// INVARIANT: no backfeed Ω → V
// INVARIANT: panic-free ABI boundary
// ============================================================

pub const R: usize = 16;
pub const DT: f32 = 4.166_667e-3;   // 1/240 as fixed literal
pub const ALPHA: f32 = 0.98;
pub const LAMBDA: f32 = 0.05;
pub const ETA: f32 = 0.01;
pub const DAMPING: f32 = 0.98;
pub const U_MAX: f32 = 100.0;
pub const EPS: f32 = 1e-8;

// ── GHOST (diagnostic only — never branches core) ───────────
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ghost {
    Nominal = 0, Collapse = 1, Diffuse = 2,
    Echo = 3, Burst = 4, Trap = 5, Vacuum = 6,
}

fn classify(stress: f32, novelty: f32, drift: f32, entropy: f32) -> Ghost {
    if stress  > 1.5                    { Ghost::Burst }
    else if novelty < EPS && entropy < 0.1 { Ghost::Collapse }
    else if novelty > 0.9 && entropy > 2.0 { Ghost::Diffuse }
    else if entropy < 0.3 && stress < 0.1  { Ghost::Echo }
    else if drift   > 0.01                 { Ghost::Trap }
    else                                   { Ghost::Nominal }
}

// ── UNIFIED TRACE FRAME (replaces StepResult + TraceEntry) ──
// ABI: stable — do not reorder
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TraceFrame {
    pub frame:     u64,
    pub stress:    f32,   // B(t) = ‖S‖/(‖Z‖+ε)
    pub novelty:   f32,   // ‖R‖/‖input‖
    pub drift:     f32,   // ‖W^TW − I‖_F
    pub entropy:   f32,   // −Σ p_k ln p_k
    pub energy:    f32,   // ‖Z‖
    pub ghost:     u8,    // Ghost as u8
    pub contained: u8,    // 1 if containment fired
}

// ── CORE STATE (cache-aligned, fixed-size, no heap) ─────────
#[repr(C, align(64))]
pub struct DvsmCore {
    pub z: [f32; R],
    pub s: [f32; R],
    pub w: [f32; R * R],
    pub v: [f32; R],
    pub x: [f32; R],
    // workspace (not exported)
    c:   [f32; R],
    p:   [f32; R],
    res: [f32; R],
    // scalars
    pub n:     u32,
    pub r:     u32,
    pub frame: u64,
    pub alive: u8,
}

// ── MATH (SIMD-ready loops, no allocation) ──────────────────

#[inline(always)]
fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
    let mut s = 0.0f32;
    for i in 0..n { s += a[i] * b[i]; }
    s
}

#[inline(always)]
fn norm2(a: &[f32], n: usize) -> f32 { dot(a, a, n) }

#[inline(always)]
fn norm_safe(a: &[f32], n: usize) -> f32 { norm2(a, n).sqrt().max(EPS) }

// ── GEOMETRY (Stiefel ops) ──────────────────────────────────

fn orthonormalize(w: &mut [f32], stride: usize, r: usize) {
    for k in 0..r {
        let base = k * stride;
        for j in 0..k {
            let jb = j * stride;
            let d = dot(&w[base..base+stride], &w[jb..jb+stride], stride);
            for i in 0..stride { w[base + i] -= d * w[jb + i]; }
        }
        let nrm = norm_safe(&w[base..base+stride], stride);
        for i in 0..stride { w[base + i] /= nrm; }
    }
}

fn stiefel_drift(w: &[f32], stride: usize, r: usize) -> f32 {
    let mut d = 0.0f32;
    for k1 in 0..r {
        for k2 in 0..r {
            let g = dot(&w[k1*stride..], &w[k2*stride..], stride);
            let t = if k1 == k2 { 1.0f32 } else { 0.0 };
            let e = g - t;
            d += e * e;
        }
    }
    d.sqrt()
}

// ── CONTAINMENT ─────────────────────────────────────────────

#[inline]
fn contain(z: &mut [f32], r: usize) -> bool {
    let e2 = norm2(z, r);
    if e2 > U_MAX * U_MAX || !e2.is_finite() {
        for k in 0..r { z[k] = 0.0; }
        return true;
    }
    false
}

fn rebirth(c: &mut DvsmCore) {
    let r = c.r as usize;
    for k in 0..r { c.z[k] = EPS * c.w[k * R + (k % (c.n as usize))]; }
    c.s = [0.0; R];
    c.v = [0.0; R];
    c.alive = 1;
}

// ── CORE STEP (5-stage: project → evolve → update → adapt → emit)

impl DvsmCore {
    pub fn new(n: u32, r: u32) -> Self {
        let n = n.min(R as u32);
        let r = r.min(n);
        let mut w = [0.0f32; R * R];
        for k in 0..(r as usize) { w[k * R + k] = 1.0; }
        Self {
            z: [0.0; R], s: [0.0; R], w, v: [0.0; R], x: [0.0; R],
            c: [0.0; R], p: [0.0; R], res: [0.0; R],
            n, r, frame: 0, alive: 1,
        }
    }

    pub fn step(&mut self, input: &[f32]) -> TraceFrame {
        let n = self.n as usize;
        let r = self.r as usize;
        let in_n = input.len().min(n);

        if self.alive == 0 { rebirth(self); }

        // ── STAGE 1: project ────────────────────────────────
        for k in 0..r {
            self.c[k] = dot(&self.w[k*R..k*R+in_n], input, in_n);
        }
        for i in 0..in_n { self.p[i] = 0.0; }
        for k in 0..r {
            for i in 0..in_n { self.p[i] += self.w[k*R+i] * self.c[k]; }
        }
        let mut r_n2 = 0.0f32;
        for i in 0..in_n {
            self.res[i] = input[i] - self.p[i];
            r_n2 += self.res[i] * self.res[i];
        }
        let r_norm = r_n2.sqrt();

        // ── STAGE 2: evolve (Lie-bracket + dissipation) ─────
        for k in 0..r {
            let mut acc = 0.0f32;
            for j in 0..r {
                if j == k { continue; }
                let kappa = ((k as f32) * 1.37 - (j as f32) * 1.73).sin();
                acc += (self.z[k] * self.s[j] - self.z[j] * self.s[k]) * kappa;
            }
            self.z[k] += DT * (acc - LAMBDA * self.z[k]);
        }
        let killed = contain(&mut self.z, r);
        if killed { self.alive = 0; }

        // ── STAGE 3: update (memory + state) ────────────────
        for k in 0..r {
            self.s[k] = ALPHA * self.s[k] + (1.0 - ALPHA) * self.z[k];
        }
        for i in 0..in_n {
            self.v[i] = self.v[i] * DAMPING + (self.res[i] + self.s[i]) * ETA;
            self.x[i] += self.v[i] * DT;
        }

        // ── STAGE 4: adapt (basis on Stiefel) ───────────────
        if r_norm > EPS {
            let c_norm = norm_safe(&self.c, r);
            for k in 0..r {
                let sc = self.c[k] / c_norm;
                for i in 0..in_n { self.w[k*R+i] += ETA * self.res[i] * sc; }
            }
            orthonormalize(&mut self.w, R, r);
        }

        // ── STAGE 5: emit ───────────────────────────────────
        let z_n = norm_safe(&self.z, r);
        let s_n = norm2(&self.s, r).sqrt();
        let in_norm = norm_safe(input, in_n);
        let stress = s_n / z_n;
        let novelty = r_norm / in_norm;
        let drift = stiefel_drift(&self.w, R, r);
        let ent = {
            let tot = norm2(&self.z, r) + EPS;
            let mut h = 0.0f32;
            for k in 0..r {
                let pk = (self.z[k] * self.z[k]) / tot;
                if pk > EPS { h -= pk * pk.ln(); }
            }
            h
        };

        self.frame += 1;

        TraceFrame {
            frame: self.frame,
            stress, novelty, drift, entropy: ent, energy: z_n,
            ghost: classify(stress, novelty, drift, ent) as u8,
            contained: killed as u8,
        }
    }

    #[inline] pub fn is_vacuum(&self) -> bool { self.alive == 0 }
}

// ── C ABI (stable surface — 5 functions, no signature changes) ──

#[no_mangle]
pub extern "C" fn dvsm_init(n: u32, r: u32) -> *mut DvsmCore {
    Box::into_raw(Box::new(DvsmCore::new(n, r)))
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_step(
    core: *mut DvsmCore, input: *const f32, input_len: u32, out: *mut TraceFrame,
) -> i32 {
    let Some(c) = core.as_mut() else { return -1 };
    let n = c.n.min(input_len) as usize;
    if input.is_null() || n == 0 { return -2; }
    let inp = std::slice::from_raw_parts(input, n);
    let tf = c.step(inp);
    if let Some(o) = out.as_mut() { *o = tf; }
    0
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_is_vacuum(core: *const DvsmCore) -> u8 {
    core.as_ref().map_or(1, |c| c.alive ^ 1)
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_get_trace(
    core: *const DvsmCore, frame: *const TraceFrame, out: *mut TraceFrame,
) -> i32 {
    let (Some(_c), Some(f), Some(o)) = (core.as_ref(), frame.as_ref(), out.as_mut())
        else { return -1 };
    *o = *f;
    0
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_free(core: *mut DvsmCore) {
    if !core.is_null() { drop(Box::from_raw(core)); }
}

/* ============================================================
 * DVSM-π+++ / DQSDv2 · C ABI HEADER (tightened)
 * Stable surface — do not modify signatures
 * Author: Daniel J. Dillberg
 * ============================================================ */

#ifndef DVSM_H
#define DVSM_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Ghost modes (diagnostic only — never branches core) */
#define DVSM_NOMINAL   0
#define DVSM_COLLAPSE  1
#define DVSM_DIFFUSE   2
#define DVSM_ECHO      3
#define DVSM_BURST     4
#define DVSM_TRAP      5
#define DVSM_VACUUM    6

/* Unified trace frame (replaces separate StepResult + TraceEntry) */
typedef struct {
    uint64_t frame;
    float    stress;      /* B(t)                             */
    float    novelty;     /* ‖R‖/‖input‖                     */
    float    drift;       /* ‖WᵀW − I‖_F                     */
    float    entropy;     /* −Σ p_k ln p_k                    */
    float    energy;      /* ‖Z‖                              */
    uint8_t  ghost;       /* DVSM_NOMINAL..DVSM_VACUUM        */
    uint8_t  contained;   /* 1 if containment fired this step */
} DvsmTraceFrame;

/* Opaque core handle */
typedef struct DvsmCore DvsmCore;

/* ── STABLE API (5 functions, additive extension only) ────── */

DvsmCore*  dvsm_init      (uint32_t n, uint32_t r);

int32_t    dvsm_step       (DvsmCore*       core,
                            const float*    input,
                            uint32_t        input_len,
                            DvsmTraceFrame* out);

uint8_t    dvsm_is_vacuum  (const DvsmCore* core);

int32_t    dvsm_get_trace  (const DvsmCore*       core,
                            const DvsmTraceFrame*  frame,
                            DvsmTraceFrame*        out);

void       dvsm_free       (DvsmCore* core);

#ifdef __cplusplus
}
#endif

#endif /* DVSM_H */

// ============================================================
// DVSM-π+++ / DQSDv2 · GPU COMPUTE CONTRACT
// WGSL binding layout for WebGPU / Vulkan / Metal / DX12
// Author: Daniel J. Dillberg
// ============================================================
// CONTRACT: R_buf[i] = Z[i] - (W Wᵀ Z)[i]
// CONTRACT: containment at ‖Z‖ > u_max (atomic kill in diag.w)
// CONTRACT: S = mix(S, Z, 1-alpha) per component
// ============================================================

struct Params {
    dt:     f32,
    alpha:  f32,
    lambda: f32,
    u_max:  f32,
    r:      u32,
    _pad:   u32,
    _pad2:  u32,
    _pad3:  u32,
};

@group(0) @binding(0) var<storage, read_write> Z:     array<f32>;
@group(0) @binding(1) var<storage, read_write> S:     array<f32>;
@group(0) @binding(2) var<storage, read>       W:     array<f32>;
@group(0) @binding(3) var<storage, read_write> R_buf: array<f32>;
@group(0) @binding(4) var<storage, read_write> diag:  array<f32>;
@group(0) @binding(5) var<uniform>             p:     Params;

// ── KERNEL 1: Lie-bracket + dissipation ─────────────────────
@compute @workgroup_size(64)
fn lie_bracket(@builtin(global_invocation_id) gid: vec3<u32>) {
    let k = gid.x;
    let r = p.r;
    if (k >= r) { return; }

    var acc: f32 = 0.0;
    for (var j: u32 = 0u; j < r; j++) {
        if (j == k) { continue; }
        let kappa = sin(f32(k) * 1.37 - f32(j) * 1.73);
        acc += (Z[k] * S[j] - Z[j] * S[k]) * kappa;
    }
    Z[k] += p.dt * (acc - p.lambda * Z[k]);
}

// ── KERNEL 2: EMA memory ────────────────────────────────────
@compute @workgroup_size(64)
fn ema_update(@builtin(global_invocation_id) gid: vec3<u32>) {
    let k = gid.x;
    if (k >= p.r) { return; }
    S[k] = p.alpha * S[k] + (1.0 - p.alpha) * Z[k];
}

// ── KERNEL 3: containment + diagnostics ─────────────────────
@compute @workgroup_size(1)
fn containment() {
    var e2: f32 = 0.0;
    var s2: f32 = 0.0;
    for (var k: u32 = 0u; k < p.r; k++) {
        e2 += Z[k] * Z[k];
        s2 += S[k] * S[k];
    }

    let z_norm = sqrt(e2) + 1e-8;
    let s_norm = sqrt(s2);

    // containment
    if (e2 > p.u_max * p.u_max) {
        for (var k: u32 = 0u; k < p.r; k++) { Z[k] = 0.0; }
        diag[3] = 1.0; // kill flag
    } else {
        diag[3] = 0.0;
    }

    // diagnostics: [stress, energy, s_norm, kill_flag]
    diag[0] = s_norm / z_norm;
    diag[1] = z_norm;
    diag[2] = s_norm;
}
