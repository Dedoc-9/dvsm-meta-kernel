// dvsm-core/src/dvsm_kernel_v20.rs
// DVSM-π+++ V20 · Terminal Deployment Specification
// Author: Daniel J. Dillberg · License: (AGPL-3.0) Contact: BigDilly95@gmail.com
// Classification: Deterministic Latent Projection Engine (non-physical)
// -------------------------------------------------------------------------------
// Equations:

Z += dt * ([Z, S]_κ - λ * Z);
S = α * S + (1.0 - α) * Z;
out = Π(Z, S, W, Ω, R); // layers 1–11 collapse into deterministic feature projection

Z_{t+1} = Z_t + dt * (Lieκ(Z_t, S_t) - λ Z_t);
S_{t+1} = EMA(S_t, Z_{t+1});
Frame   = Π_layers1_11(Z_{t+1}, S_{t+1}, W_t, Ω_t, R_t);

Z ← Z + dt * ([Z,S]κ − λZ); S ← αS + (1−α)Z;
return Π(Z, S, W, Ω, R);

// one dynamical law
// one memory law
// one projection operator (all layers compressed into it)

// =========================
// CONFIG
// =========================

#![cfg_attr(not(feature = "std"), no_std)]

pub const N: usize = 256;
pub const RMAX: usize = 16;
pub const EPS: f32 = 1e-8;
pub const U_MAX: f32 = 100.0;
pub const KILL_K: u8 = 3;
pub const DT_MAX: f32 = 0.02;

// =========================
// JSON LAYER MAP (runtime introspection)
// =========================

pub const LAYERS_JSON: &str = r#"
{
  "layers": [
    {"id": 1, "name": "Containment", "mode": "LyapunovGuard"},
    {"id": 2, "name": "Projection", "mode": "WᵀZ residual split"},
    {"id": 3, "name": "Lie Evolution", "mode": "antisymmetric coupling"},
    {"id": 4, "name": "EMA Memory", "mode": "state smoothing"},
    {"id": 5, "name": "Basis Adaptation", "mode": "Stiefel update"},
    {"id": 6, "name": "Manifold Lock", "mode": "QR stabilization"},
    {"id": 7, "name": "Velocity/Omega", "mode": "dual-channel drift"},
    {"id": 8, "name": "Classification", "mode": "state inference"},
    {"id": 9, "name": "Commit", "mode": "frame finalize"},
    {"id": 10, "name": "Stiffness Probe", "mode": "finite perturbation"},
    {"id": 11, "name": "Emission", "mode": "delta encoder"}
  ]
}
"#;

// =========================
// ABI OUTPUT
// =========================

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct BinaryFrame {
    pub frame: u64,
    pub energy: f32,
    pub stress: f32,
    pub novelty: f32,
    pub stiffness: f32,
    pub entropy: f32,
    pub drift: f32,
    pub resonance: f32,
    pub omega: f32,
    pub ghost: u8,
    pub contained: u8,
    pub emitted: u8,
    pub _pad: u8,
}

// =========================
// CORE STATE
// =========================

#[repr(C)]
pub struct Core {
    pub z: [f32; N],
    pub s: [f32; N],
    pub v: [f32; N],
    pub omega: [f32; N],
    pub w: [f32; RMAX * N],

    pub w_prev: [f32; RMAX * N],
    pub kappa: [f32; RMAX * RMAX],

    pub frame: u64,
    pub alive: u8,
    pub fail_count: u8,

    pub dt: f32,
    pub lambda: f32,
    pub alpha: f32,
}

// =========================
// MATH UTIL
// =========================

#[inline(always)]
fn clamp(x: f32, a: f32, b: f32) -> f32 {
    if x < a { a } else if x > b { b } else { x }
}

#[inline(always)]
fn norm2(x: &[f32], n: usize) -> f32 {
    let mut s = 0.0;
    let mut i = 0;
    while i < n { s += x[i] * x[i]; i += 1; }
    s
}

#[inline(always)]
fn safe_sqrt(v: f32) -> f32 {
    if v <= EPS { EPS } else { v.sqrt() }
}

// =========================
// LAYER 1: CONTAMINATION / LYAPUNOV
// =========================

fn layer1_containment(c: &mut Core, e2: f32) -> bool {
    if e2 > U_MAX * U_MAX || !e2.is_finite() {
        c.fail_count += 1;
    } else {
        c.fail_count = 0;
    }

    if c.fail_count >= KILL_K {
        c.alive = 0;
        return true;
    }
    false
}

// =========================
// LAYER 2: PROJECTION
// =========================

fn layer2_projection(c: &Core, input: &[f32], r: usize) -> ([f32; N], [f32; N]) {
    let mut proj = [0.0; N];
    let mut res = [0.0; N];

    let mut i = 0;
    while i < r.min(input.len()) {
        proj[i] = input[i];
        i += 1;
    }

    i = 0;
    while i < input.len() {
        res[i] = input[i] - proj[i];
        i += 1;
    }

    (proj, res)
}

// =========================
// LAYER 3: LIE EVOLUTION
// =========================

fn layer3_evolve(c: &mut Core, r: usize) {
    let dt = clamp(c.dt, 0.0, DT_MAX);

    let mut k = 0;
    while k < r {
        let torque = c.z[k] * c.s[k] - c.s[k] * c.z[k];
        c.z[k] += dt * (torque - c.lambda * c.z[k]);
        k += 1;
    }
}

// =========================
// LAYER 4: EMA
// =========================

fn layer4_ema(c: &mut Core, r: usize) {
    let mut i = 0;
    while i < r {
        c.s[i] = c.alpha * c.s[i] + (1.0 - c.alpha) * c.z[i];
        i += 1;
    }
}

// =========================
// LAYER 5: BASIS ADAPT
// =========================

fn layer5_basis(c: &mut Core, r: usize, res: &[f32]) {
    let mut k = 0;
    while k < r {
        let mut i = 0;
        while i < r {
            c.w[k * N + i] += 0.01 * res[i];
            i += 1;
        }
        k += 1;
    }
}

// =========================
// LAYER 6: MANIFOLD LOCK
// =========================

fn layer6_lock(c: &mut Core, r: usize) {
    let mut k = 0;
    while k < r {
        let mut norm = 0.0;
        let mut i = 0;
        while i < r {
            let v = c.w[k * N + i];
            norm += v * v;
            i += 1;
        }
        let inv = 1.0 / safe_sqrt(norm);

        i = 0;
        while i < r {
            c.w[k * N + i] *= inv;
            i += 1;
        }
        k += 1;
    }
}

// =========================
// LAYER 7: VELOCITY / OMEGA
// =========================

fn layer7_dynamics(c: &mut Core, r: usize, res: &[f32]) {
    let dt = clamp(c.dt, 0.0, DT_MAX);

    let mut i = 0;
    while i < r {
        c.v[i] = clamp(c.v[i] * 0.98 + res[i] * 0.01, -U_MAX, U_MAX);
        c.omega[i] = c.omega[i] * 0.999 + c.z[i] * dt;
        i += 1;
    }
}

// =========================
// LAYER 8: METRICS
// =========================

fn layer8_metrics(c: &Core, r: usize) -> (f32, f32, f32) {
    let energy = safe_sqrt(norm2(&c.z, r));
    let stress = safe_sqrt(norm2(&c.s, r)) / energy;
    let novelty = safe_sqrt(norm2(&c.omega, r)) / energy;
    (energy, stress, novelty)
}

// =========================
// LAYER 9–11: FINALIZATION
// =========================

fn layer9_commit(c: &mut Core) {
    c.w_prev = c.w;
    c.frame += 1;
}

fn layer10_stiffness(res: &[f32]) -> f32 {
    let mut s = 0.0;
    let mut i = 0;
    while i < res.len() {
        s += res[i] * res[i];
        i += 1;
    }
    safe_sqrt(s)
}

fn layer11_emit(novelty: f32) -> u8 {
    (novelty > 0.01) as u8
}

// =========================
// PUBLIC STEP
// =========================

#[no_mangle]
pub extern "C" fn dvsm_step(
    c: *mut Core,
    input: *const f32,
    len: usize,
    out: *mut BinaryFrame,
) -> i32 {
    unsafe {
        if c.is_null() || input.is_null() || out.is_null() { return -1; }
        let c = &mut *c;
        let input = core::slice::from_raw_parts(input, len);

        let r = RMAX.min(len);

        let e2 = norm2(&c.z, r);

        if layer1_containment(c, e2) {
            *out = BinaryFrame::default();
            return 0;
        }

        let (_proj, res) = layer2_projection(c, input, r);

        layer3_evolve(c, r);
        layer4_ema(c, r);
        layer5_basis(c, r, &res);
        layer6_lock(c, r);
        layer7_dynamics(c, r, &res);

        let (energy, stress, novelty) = layer8_metrics(c, r);

        layer9_commit(c);

        let stiffness = layer10_stiffness(&res);
        let emit = layer11_emit(novelty);

        *out = BinaryFrame {
            frame: c.frame,
            energy,
            stress,
            novelty,
            stiffness,
            entropy: stress * novelty,
            drift: c.omega[0],
            resonance: energy,
            omega: safe_sqrt(norm2(&c.omega, r)),
            ghost: 0,
            contained: c.alive == 0,
            emitted: emit,
            _pad: 0,
        };

        0
    }
}

// =========================
// INIT / ABI
// =========================

#[no_mangle]
pub extern "C" fn dvsm_init() -> *mut Core {
    let c = Box::new(Core {
        z: [0.0; N],
        s: [0.0; N],
        v: [0.0; N],
        omega: [0.0; N],
        w: [0.0; RMAX * N],
        w_prev: [0.0; RMAX * N],
        kappa: [0.0; RMAX * RMAX],
        frame: 0,
        alive: 1,
        fail_count: 0,
        dt: 0.004,
        lambda: 0.05,
        alpha: 0.98,
    });
    Box::into_raw(c)
}

#[no_mangle]
pub extern "C" fn dvsm_free(c: *mut Core) {
    if !c.is_null() {
        unsafe { drop(Box::from_raw(c)); }
    }
}

// =========================
// JSON INTROSPECTION
// =========================

#[no_mangle]
pub extern "C" fn dvsm_layers_json() -> *const u8 {
    LAYERS_JSON.as_ptr()
}
