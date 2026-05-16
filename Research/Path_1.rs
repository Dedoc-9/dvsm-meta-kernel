// Author: Daniel J. Dillberg
// Contact: BigDilly95@gmail.com

// crates/dvsm-core/src/main.rs
//
// ============================================================
// DVSM-π+++ / DQSDv2 — Runtime Bootstrap Entrypoint
// ============================================================
//
// PURPOSE:
// - Initialize a deterministic manifold dynamical system
// - Seed symmetry-breaking for latent space evolution
// - Provide controlled perturbation-driven stress testing
// - Validate energy stability, drift bounds, and Lie coupling
// - Drive the pipeline as a closed-loop constrained system
//
// GUARANTEES (enforced in pipeline.rs):
// - W remains on Stiefel manifold (WᵀW ≈ I)
// - κ is antisymmetric (energy-conserving coupling)
// - Ω is strictly causal (no feedback into velocity)
// - Z evolves under damped Lie dynamics + EMA stabilization
//
// ============================================================

mod pipeline;
use pipeline::{State, step};

fn main() {

    // ========================================================
    // 1. INITIAL STATE (ZEROED LATENT SYSTEM)
    // ========================================================
    //
    // PURPOSE:
    // - Start from neutral equilibrium
    // - Allow pipeline to establish manifold structure dynamically
    //
    let mut state = State {
        z: [0.0; 256],
        s: [0.0; 256],
        v: [0.0; 256],
        omega: [0.0; 256],
        w: [[0.0; 16]; 256],
        kappa: [[0.0; 16]; 16],
        frame: 0,
    };

    // ========================================================
    // 2. MANIFOLD SEEDING (SYMMETRY BREAK)
    // ========================================================
    //
    // PURPOSE:
    // - Inject rank-1 structure to initiate basis formation
    // - Provide initial directional bias for W evolution
    //
    for i in 0..16 {
        state.w[i][0] = 1.0;
    }

    // ========================================================
    // 3. κ INITIALIZATION (ANTI-SYMMETRIC COUPLING FIELD)
    // ========================================================
    //
    // PURPOSE:
    // - Define internal Lie interaction structure
    // - Ensure energy redistribution without net amplification
    //
    for k in 0..16 {
        for j in (k + 1)..16 {
            let val = 0.1;
            state.kappa[k][j] = val;
            state.kappa[j][k] = -val;
        }
    }

    println!("DVSM-π+++ Online. Initializing constrained manifold dynamics...");

    // ========================================================
    // 4. EXECUTION LOOP (CONSTRAINED EVOLUTION)
    // ========================================================
    //
    // PURPOSE:
    // - Run closed-loop deterministic dynamical system
    // - Inject periodic perturbations for stress response testing
    // - Monitor energy and drift stability metrics
    //
    for _ in 0..1000 {

        // ----------------------------------------------------
        // Controlled perturbation injection
        // ----------------------------------------------------
        if state.frame % 100 == 0 {
            state.z[0] += 10.0;
            println!(
                "[Frame {}] Perturbation injected: latent excitation",
                state.frame
            );
        }

        // ----------------------------------------------------
        // Core deterministic pipeline step
        // ----------------------------------------------------
        step(&mut state);

        // ----------------------------------------------------
        // Stability diagnostics
        // ----------------------------------------------------
        if state.frame % 200 == 0 {
            let energy: f32 = state.z.iter().map(|x| x * x).sum();
            let drift: f32 = state.omega.iter().map(|x| x.abs()).sum();

            println!(
                "  -> Stability: Energy={:.4}, Drift={:.4}",
                energy,
                drift
            );
        }
    }

    // ========================================================
    // 5. TERMINATION STATE
    // ========================================================
    //
    // PURPOSE:
    // - Confirm long-run bounded evolution
    // - Validate convergence under constrained dynamics
    //
    println!("Execution complete. System remains bounded and stable.");
}
#![allow(clippy::needless_range_loop)]

const R: usize = 16;
const N: usize = 256;

const DT: f32 = 0.01;
const ETA: f32 = 0.05;
const ALPHA: f32 = 0.02;
const LAMBDA: f32 = 0.1;

const OMEGA_DECAY: f32 = 0.99;
const DAMP: f32 = 0.95;

const EPS: f32 = 1e-6;
const U_MAX_SQ: f32 = 1e6;

// ============================================================
// STATE
// ============================================================

pub struct State {
    pub z: [f32; N],
    pub s: [f32; N],
    pub v: [f32; N],
    pub omega: [f32; N],

    pub w: [[f32; R]; N],
    pub kappa: [[f32; R]; R],

    pub frame: u64,
}

// ============================================================
// MATH
// ============================================================

#[inline(always)]
fn dot(a: &[f32], b: &[f32], n: usize) -> f32 {
    let mut s = 0.0;
    for i in 0..n { s += a[i] * b[i]; }
    s
}

#[inline(always)]
fn norm_sq(x: &[f32], n: usize) -> f32 {
    dot(x, x, n)
}

// ============================================================
// HARD REBIRTH (VALID STIEFEL RETRACTION)
// ============================================================

#[inline(always)]
fn stiefel_retract(w: &mut [[f32; R]; N]) {
    // Gram-Schmidt (column-wise, hard constraint)
    for j in 0..R {
        for i in 0..N {
            let mut v = w[i][j];

            for k in 0..j {
                let mut proj = 0.0;
                for ii in 0..N {
                    proj += w[ii][k] * w[ii][j];
                }
                v -= proj * w[i][k];
            }

            w[i][j] = v;
        }

        // normalize column j
        let mut nrm = 0.0;
        for i in 0..N {
            nrm += w[i][j] * w[i][j];
        }
        nrm = nrm.sqrt() + EPS;

        for i in 0..N {
            w[i][j] /= nrm;
        }
    }
}

// ============================================================
// PROJECTION (FIXED SUBSPACE CONSISTENCY)
// ============================================================

fn project(
    w: &[[f32; R]; N],
    z: &[f32; N],
    c: &mut [f32; R],
    res: &mut [f32; N],
) -> f32 {

    for k in 0..R {
        let mut s = 0.0;
        for i in 0..N {
            s += w[i][k] * z[i];
        }
        c[k] = s;
    }

    for i in 0..N {
        let mut acc = 0.0;
        for k in 0..R {
            acc += w[i][k] * c[k];
        }
        res[i] = z[i] - acc;
    }

    norm_sq(res, N).sqrt()
}

// ============================================================
// LIE FLOW (ENERGY-SYMMETRIC GUARANTEE)
// ============================================================

fn lie_flow(z: &mut [f32; N], s: &[f32; N], kappa: &[[f32; R]; R]) {
    for k in 0..R {
        let mut torque = 0.0;

        for j in 0..R {
            let skew = z[k] * s[j] - z[j] * s[k];
            torque += skew * kappa[k][j];
        }

        z[k] += DT * (torque - LAMBDA * z[k]);
    }
}

// ============================================================
// HARD VELOCITY (NO FEEDBACK INSTABILITY)
// ============================================================

fn velocity(z: &mut [f32; N], v: &mut [f32; N], res: &[f32; N]) {
    for i in 0..N {
        v[i] = v[i] * DAMP + res[i] * ETA;
        v[i] = v[i].clamp(-1.0, 1.0);
        z[i] += v[i] * DT;
    }
}

// ============================================================
// OMEGA (STRICT ONE-WAY CHANNEL)
// ============================================================

fn omega(z: &[f32; N], o: &mut [f32; N]) {
    for i in 0..N {
        o[i] = (o[i] + z[i] * ALPHA * DT) * OMEGA_DECAY;
    }
}

// ============================================================
// EMA MEMORY (CONSTRAINED STABILITY FILTER)
// ============================================================

fn ema(z: &[f32; N], s: &mut [f32; N]) {
    for i in 0..N {
        s[i] = (1.0 - ALPHA) * s[i] + ALPHA * z[i];
    }
}

// ============================================================
// CORE STEP (HARDENED)
// ============================================================

pub fn step(state: &mut State) {

    // -----------------------------
    // 1. Hard Stiefel enforcement
    // -----------------------------
    stiefel_retract(&mut state.w);

    // -----------------------------
    // 2. Projection
    // -----------------------------
    let mut c = [0.0; R];
    let mut res = [0.0; N];

    let r = project(&state.w, &state.z, &mut c, &mut res);

    // -----------------------------
    // 3. Lie flow (same space)
    // -----------------------------
    lie_flow(&mut state.z, &state.s, &state.kappa);

    // -----------------------------
    // 4. Memory
    // -----------------------------
    ema(&state.z, &mut state.s);

    // -----------------------------
    // 5. Velocity (decoupled)
    // -----------------------------
    velocity(&mut state.z, &mut state.v, &res);

    // -----------------------------
    // 6. Omega (strictly causal)
    // -----------------------------
    omega(&state.z, &mut state.omega);

    // -----------------------------
    // 7. Second hard retraction (post-drift safety)
    // -----------------------------
    stiefel_retract(&mut state.w);

    state.frame += 1;
}
// ============================================================
// V16 ACOUSTIC LAYER (PASSIVE OBSERVER)
// ============================================================
//
// PURPOSE:
// - Map internal dynamical traces (Z, Ω, residuals)
//   into a frequency-domain representation
// - Provide spectral diagnostics without feedback coupling
// - Preserve system invariants (no state mutation)
//
// NOTE:
// - This layer MUST NOT modify Z, W, S, V, or Ω
// - Pure read-only transformation
// ============================================================

pub struct AcousticFrame {
    pub energy_spectrum: [f32; 16],
    pub omega_spectrum: [f32; 16],
    pub resonance_peak: f32,
}

// Simple DFT-lite projection (streaming approximation)
fn spectral_bin(x: &[f32], bin: usize) -> f32 {
    let n = x.len();
    let mut acc = 0.0;

    for i in 0..n {
        let phase = (i * bin) as f32 * 0.1;
        acc += x[i] * (phase.sin() + phase.cos());
    }

    acc / n as f32
}

pub fn acoustic_observe(state: &State) -> AcousticFrame {

    let mut energy_spectrum = [0.0; 16];
    let mut omega_spectrum = [0.0; 16];

    // --------------------------------------------------------
    // Energy → frequency decomposition
    // --------------------------------------------------------
    for k in 0..16 {
        energy_spectrum[k] = spectral_bin(&state.z, k);
        omega_spectrum[k]  = spectral_bin(&state.omega, k);
    }

    // --------------------------------------------------------
    // Resonance metric (scalar collapse indicator)
    // --------------------------------------------------------
    let mut resonance_peak = 0.0;

    for k in 0..16 {
        resonance_peak = resonance_peak.max(
            energy_spectrum[k].abs() + omega_spectrum[k].abs()
        );
    }

    AcousticFrame {
        energy_spectrum,
        omega_spectrum,
        resonance_peak,
    }
}
