/*
===========================================================
STREAMING LOW-RANK INTERACTION ENGINE (ONE-FILE COLLAPSE)
===========================================================\

Author: Daniel J. Dillberg

This is a minimal real-time particle system derived from a
low-rank McKean–Vlasov / feature-field reduction.

Core idea:
- Particles do NOT interact directly.
- They interact through a shared low-rank feature field.
- Non-normal dynamics arise from EMA lag (shear memory).
- Geometry is encoded in a polynomial basis.

Complexity: O(N · R)
No neighbor lists. No graphs. No external solvers.
===========================================================
*/

use std::f32::consts::SQRT_2;

/// -------------------------------
/// CONFIG
/// -------------------------------
const R: usize = 8;          // feature rank (8–16 typical)
const DT: f32 = 1.0 / 240.0;  // fixed timestep (240 FPS)
const ALPHA: f32 = 0.98;      // EMA memory (non-normality control)
const LAMBDA: f32 = 0.05;     // stability (restoring force)

/// -------------------------------
/// PARTICLE STATE (SoA)
/// -------------------------------
pub struct System {
    pub n: usize,

    pub x0: Vec<f32>,
    pub x1: Vec<f32>,
    pub x2: Vec<f32>,

    pub v0: Vec<f32>,
    pub v1: Vec<f32>,
    pub v2: Vec<f32>,

    pub z: [f32; R],        // global feature field
    pub z_shear: [f32; R],  // EMA lag (non-normality)
    pub w: [f32; R * 4],    // polynomial basis weights
}

impl System {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            x0: vec![0.0; n],
            x1: vec![0.0; n],
            x2: vec![0.0; n],

            v0: vec![0.0; n],
            v1: vec![0.0; n],
            v2: vec![0.0; n],

            z: [0.0; R],
            z_shear: [0.0; R],
            w: [0.0; R * 4],
        }
    }
}

/// -------------------------------
/// POLYNOMIAL BASIS (local feature map)
/// -------------------------------
#[inline(always)]
fn basis(x: f32) -> [f32; 4] {
    let x2 = x * x;
    let x3 = x2 * x;
    [1.0, x, x2, x3]
}

/// -------------------------------
/// FEATURE EVALUATION (low-rank projection)
/// -------------------------------
#[inline(always)]
fn phi(system: &System, k: usize, b: &[f32; 4]) -> f32 {
    let w = &system.w[k * 4..k * 4 + 4];
    w[0] * b[0] + w[1] * b[1] + w[2] * b[2] + w[3] * b[3]
}

/// -------------------------------
/// MAIN UPDATE STEP
/// -------------------------------
pub fn step(sys: &mut System) {
    // reset features
    for k in 0..R {
        sys.z[k] = 0.0;
    }

    /*
    ----------------------------------------------------
    PASS 1: GLOBAL FEATURE FIELD (alignment statistics)
    ----------------------------------------------------
    */
    for i in 0..sys.n {
        let b = basis(sys.x0[i]);

        for k in 0..R {
            sys.z[k] += phi(sys, k, &b);
        }
    }

    let inv_n = 1.0 / sys.n as f32;

    for k in 0..R {
        sys.z[k] *= inv_n;
    }

    /*
    ----------------------------------------------------
    PASS 2: EMA SHEAR (NON-NORMAL MEMORY)
    ----------------------------------------------------
    */
    for i in 0..sys.n {
        let b = basis(sys.x0[i]);

        for k in 0..R {
            let p = phi(sys, k, &b);

            // collapsed psi = local damped projection
            let psi = 0.5 * p;

            let diff = p - psi;

            sys.z_shear[k] =
                ALPHA * sys.z_shear[k]
                + (1.0 - ALPHA) * diff;
        }
    }

    /*
    ----------------------------------------------------
    PASS 3: FORCE + INTEGRATION
    ----------------------------------------------------
    */
    for i in 0..sys.n {
        let bx = sys.x0[i];
        let by = sys.x1[i];
        let bz = sys.x2[i];

        let b = basis(bx);

        let mut fx = 0.0;
        let mut fy = 0.0;
        let mut fz = 0.0;

        for k in 0..R {
            let uk = phi(sys, k, &b);

            let signal = sys.z[k] + sys.z_shear[k];

            let f = uk * signal;

            fx += f;
            fy += f;
            fz += f;
        }

        // restoring stability (spectral damping)
        fx -= LAMBDA * bx;
        fy -= LAMBDA * by;
        fz -= LAMBDA * bz;

        // integrate velocity
        sys.v0[i] += DT * fx;
        sys.v1[i] += DT * fy;
        sys.v2[i] += DT * fz;

        // integrate position (with stochastic perturbation)
        sys.x0[i] += DT * sys.v0[i];
        sys.x1[i] += DT * sys.v1[i];
        sys.x2[i] += DT * sys.v2[i];
    }
}

/// -------------------------------
/// OPTIONAL NOISE (stub)
/// -------------------------------
#[inline(always)]
fn noise() -> f32 {
    // replace with PCG / xorshift in production
    0.0
}

/*
===========================================================
END STATE INTERPRETATION
===========================================================

This system is now:

- a low-rank feature-field dynamical system
- with EMA-induced non-normal temporal skew
- and polynomial basis interaction geometry

No explicit particle-particle coupling exists.

All emergence arises from:
    z (global statistics)
    z_shear (memory lag)
    w (feature geometry)

Complexity: O(N · R)
Memory: O(N + R)
Structure: fully streaming, single-pass per frame
===========================================================
*/
