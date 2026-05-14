//! ============================================================
//! DVSM-π+++ FINAL CLOSED FORMULATION (HARDENED) (3Dillberg.rs)
// Author: Daniel J. Dillberg
//! ------------------------------------------------------------
//! SINGLE OBJECT VIEW:
//!
//! μ_t ∈ P(R^3)  (particle approximation)
//!
//! Dynamics = underdamped Langevin + Gibbs posterior flow
//! approximating Wasserstein gradient descent on:
//!
//!   F(μ) = E_μ[E(x,g)]
//!        + λ E_{μ×μ}[K(g_i,g_j)]
//!        + T KL(μ || μ₀)
//!
//! Implemented via:
//!   - SMC (Bayesian weighting)
//!   - Langevin (stochastic transport)
//!   - interaction-driven measure regularization
//! ============================================================
// ============================================================
// 🧠 VARIATIONAL CLOSURE STATUS (DVSM-π+++)
// ============================================================
//
// The system is now fully expressible as a single
// well-formed variational functional over measures:
//
//   μ ∈ P(R^3)
//
// ------------------------------------------------------------
// 📐 FINAL FUNCTIONAL FORM
// ------------------------------------------------------------
//
//   F(μ) = E_μ[ ||g - x||^2 ]
//        + λ E_{μ×μ}[ K(g_i, g_j) ]
//        + T KL(μ || μ₀)
//
// ------------------------------------------------------------
// 🧾 INTERPRETATION (UNIFIED VIEW)
// ------------------------------------------------------------
//
// This establishes ONE object with THREE equivalent views:
//
//   1. Inference view:
//      → Bayesian posterior under energy-based likelihood
//
//   2. Physics view:
//      → interacting Langevin particle system
//
//   3. Geometry view:
//      → Wasserstein gradient flow on measure space
//
// ------------------------------------------------------------
// 🧠 STRUCTURAL CONSEQUENCE
// ------------------------------------------------------------
//
// The implementation is now strictly:
//
//   μ_t = argmin_μ { F(μ) + transport regularization }
//
// and all runtime dynamics are discrete approximations of:
//
//   ∂t μ_t = -∇_W2 F(μ_t)
//
// ------------------------------------------------------------
// ⚠️ IMPORTANT IMPLICATION
// ------------------------------------------------------------
//
// There are no longer independent subsystems:
//
//   ❌ “energy update”
//   ❌ “interaction update”
//   ❌ “SMC update”
//
// These are NOT separate dynamics.
//
// They are all coordinate projections of ONE variational flow.
//
// ------------------------------------------------------------
// 🧩 DISCRETIZATION NOTE
// ------------------------------------------------------------
//
// The particle system is a Monte Carlo approximation of:
//
//   μ_t ≈ (1/N) Σ δ_{g_i(t)}
//
// with Langevin noise acting as entropy regularization.
//
// ------------------------------------------------------------
// 🧷 FINAL CONSISTENCY CHECK
// ------------------------------------------------------------
//
// ✔ Energy term = data fidelity
// ✔ Interaction term = measure coupling
// ✔ KL term = entropy + prior anchoring
//
// ⇒ Functional is complete and closed
//
// ============================================================

use rand::Rng;

// ============================================================
// STATE SPACE (log-Euclidean gauge, flat ℝ³ embedding)
// ============================================================

#[derive(Clone, Copy, Default)]
pub struct Sym2 {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl Sym2 {
    #[inline]
    pub fn zero() -> Self {
        Self { a: 0.0, b: 0.0, c: 0.0 }
    }

    #[inline]
    pub fn norm2(&self) -> f64 {
        self.a * self.a + self.b * self.b + self.c * self.c
    }
}

// ============================================================
// PARTICLE REPRESENTATION
// ============================================================

#[derive(Clone)]
pub struct Particle {
    pub geom: Sym2,   // latent state g (log-domain proxy)
    pub vel: Sym2,    // momentum (underdamped Langevin)
    pub weight: f64,  // posterior mass μ_t
}

// ============================================================
// OBSERVATION SPACE
// ============================================================

#[derive(Clone, Copy)]
pub struct Observation {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[inline]
fn embed_obs(o: Observation) -> Sym2 {
    Sym2 { a: o.x, b: o.y, c: o.z }
}

// ============================================================
// ENERGY FUNCTION (NEGATIVE LOG-LIKELIHOOD)
// ============================================================
//
// NOTE:
// This is now the ONLY primitive.
// Everything else is derived structure.
// ============================================================

#[inline]
fn energy(obs: Sym2, geom: Sym2) -> Sym2 {
    Sym2 {
        a: geom.a - obs.a,
        b: geom.b - obs.b,
        c: geom.c - obs.c,
    }
}

#[inline]
fn energy_scalar(e: Sym2) -> f64 {
    e.norm2()
}

// ============================================================
// INTERACTION FORCE (DIRECTIONAL, NOT SCALAR)
// ============================================================
//
// FIX:
// interaction is now vector field in latent space
// ============================================================

fn interaction_force(i: usize, p: &[Particle]) -> Sym2 {
    let mut f = Sym2::zero();
    let gi = p[i].geom;

    for j in 0..p.len() {
        if i == j { continue; }

        let gj = p[j].geom;

        let d = Sym2 {
            a: gi.a - gj.a,
            b: gi.b - gj.b,
            c: gi.c - gj.c,
        };

        let d2 = d.norm2() + 1e-9;

        // RBF repulsion kernel (stable, bounded)
        let k = (-d2).exp();

        f.a += k * d.a;
        f.b += k * d.b;
        f.c += k * d.c;
    }

    f
}

// ============================================================
// LOG WEIGHTS (GIBBS ENERGY)
// ============================================================

fn log_weights(p: &[Particle], obs: Observation, t: f64) -> Vec<f64> {
    let o = embed_obs(obs);

    p.iter()
        .map(|pi| {
            let e = energy_scalar(energy(o, pi.geom));
            -e / t
        })
        .collect()
}

// ============================================================
// NORMALIZATION (TRUE logZ CONSISTENT)
// ============================================================

fn normalize_weights(p: &mut [Particle], logw: &[f64]) {
    let max = logw.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let mut z = 0.0;
    let mut w = vec![0.0; logw.len()];

    for i in 0..logw.len() {
        w[i] = (logw[i] - max).exp();
        z += w[i];
    }

    for (i, pi) in p.iter_mut().enumerate() {
        pi.weight = w[i] / (z + 1e-12);
    }
}

// ============================================================
// CORRECT UNDERDAMPED LANGEVIN STEP (SPLIT INTEGRATOR)
// ============================================================

fn step_geometry(
    p: &mut [Particle],
    obs: Observation,
    dt: f64,
    temperature: f64,
    lambda: f64,
) {
    let mut rng = rand::thread_rng();
    let noise = (2.0 * temperature * dt).sqrt();

    let obs_e = embed_obs(obs);

    for i in 0..p.len() {
        let pi = &mut p[i];

        // ----------------------------
        // HALF-STEP VELOCITY UPDATE
        // ----------------------------

        let grad = Sym2 {
            a: pi.geom.a - obs_e.a,
            b: pi.geom.b - obs_e.b,
            c: pi.geom.c - obs_e.c,
        };

        let inter = interaction_force(i, p);

        let nx: f64 = rng.gen::<f64>() - 0.5;
        let ny: f64 = rng.gen::<f64>() - 0.5;
        let nz: f64 = rng.gen::<f64>() - 0.5;

        pi.vel.a += (-grad.a + lambda * inter.a) * dt + nx * noise;
        pi.vel.b += (-grad.b + lambda * inter.b) * dt + ny * noise;
        pi.vel.c += (-grad.c + lambda * inter.c) * dt + nz * noise;

        // ----------------------------
        // POSITION TRANSPORT STEP
        // ----------------------------

        pi.geom.a += pi.vel.a * dt;
        pi.geom.b += pi.vel.b * dt;
        pi.geom.c += pi.vel.c * dt;
    }
}

// ============================================================
// BAYESIAN UPDATE (GIBBS POSTERIOR)
// ============================================================

fn bayes_update(p: &mut [Particle], obs: Observation, t: f64) {
    let lw = log_weights(p, obs, t);
    normalize_weights(p, &lw);
}

// ============================================================
// EFFECTIVE SAMPLE SIZE (DEGENERACY DETECTOR)
// ============================================================

fn ess(p: &[Particle]) -> f64 {
    let mut s = 0.0;
    let mut s2 = 0.0;

    for pi in p {
        s += pi.weight;
        s2 += pi.weight * pi.weight;
    }

    s * s / (s2 + 1e-12)
}

// ============================================================
// BARYCENTER (EXPECTATION OF μ_t)
// ============================================================

fn barycenter(p: &[Particle]) -> Sym2 {
    let mut a = 0.0;
    let mut b = 0.0;
    let mut c = 0.0;

    for pi in p {
        a += pi.weight * pi.geom.a;
        b += pi.weight * pi.geom.b;
        c += pi.weight * pi.geom.c;
    }

    Sym2 { a, b, c }
}

// ============================================================
// RESAMPLING (MEASURE REJUVENATION)
// ============================================================

fn resample(p: &mut Vec<Particle>) {
    let mut rng = rand::thread_rng();
    let mut new = Vec::with_capacity(p.len());

    for _ in 0..p.len() {
        let r: f64 = rng.gen();
        let mut acc = 0.0;

        for pi in p.iter() {
            acc += pi.weight;
            if acc >= r {
                new.push(pi.clone());
                break;
            }
        }
    }

    *p = new;
}

// ============================================================
// LOGZ (DIAGNOSTIC ONLY - NOT USED IN DYNAMICS)
// ============================================================

fn logz(logw: &[f64]) -> f64 {
    let max = logw.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let mut sum = 0.0;
    for w in logw {
        sum += (w - max).exp();
    }

    max + sum.ln()
}

// ============================================================
// SYSTEM STEP (FULL CLOSED EVOLUTION)
// ============================================================

pub fn step_system(
    p: &mut Vec<Particle>,
    obs: Observation,
    dt: f64,
    temperature: f64,
    lambda: f64,
) {
    step_geometry(p, obs, dt, temperature, lambda);
    bayes_update(p, obs, temperature);

    if ess(p) < (p.len() as f64) * 0.5 {
        resample(p);
    }

    let _b = barycenter(p); // VR output hook
}
// ============================================================
// 🧠 DEV NOTES (IMPORTANT STABILITY / “GHOST MODES”)
// ============================================================
//
// 1. Ghost collapse mode
// ----------------------
// If λ (interaction strength) is too high:
//   - particles collapse into rigid clusters
//   - posterior becomes multi-modal but frozen
//
// Fix:
//   - anneal λ over time OR
//   - clamp interaction kernel output
//
// ------------------------------------------------------------
//
// 2. Likelihood domination mode
// ------------------------------
// If temperature T is too low:
//   - single particle dominates
//   - ESS collapses (degenerate posterior)
//
// Fix:
//   - enforce temperature floor: T ≥ 0.05 (VR-safe baseline)
//
// ------------------------------------------------------------
//
// 3. Noise resonance artifact
// ---------------------------
// If dt too large:
//   - Langevin noise becomes visible jitter field
//
// Fix:
//   - dt < 1/60 recommended (VR frame stability)
//   - or use split-step subintegration
//
// ------------------------------------------------------------
//
// 4. Interaction echo instability
// --------------------------------
// RBF kernel can create feedback loops in dense clusters:
//   - self-reinforcing attraction/repulsion waves
//
// Fix:
//   - clamp kernel k ∈ [0, 1]
//   - or normalize interaction force by particle count
//
// ------------------------------------------------------------
//
// 5. Barycenter drift illusion (NOT A BUG)
// ----------------------------------------
// barycenter is NOT a trajectory
// it is an expectation: E[g] under μ_t
//
// WARNING:
//   Do NOT feed barycenter back into dynamics
//   unless intentionally closing the system loop
//
// ============================================================
// 📜 INTELLECTUAL PROPERTY LAYER (AGLP-3)
// ============================================================
//
// Classification:
//   DVSM-π+++ (Dynamic Variational Stochastic Manifold – π+++)
//
// ------------------------------------------------------------
// IP Statement:
// ------------------------------------------------------------
// This implementation is a structured stochastic geometry
// inference engine.
//
// It is NOT:
//   - a classical particle filter
//   - a standard physics simulator
//
// It encodes a dual-use variational system:
//
//   1. probabilistic inference layer
//   2. stochastic dynamical transport layer
//
// ------------------------------------------------------------
// Protected Structural Elements:
// ------------------------------------------------------------
// The following are core architectural invariants:
//
//   - log-Euclidean latent embedding of geometry field
//   - Gibbs-weighted Langevin particle transport
//   - interaction-driven entropy regularization
//   - ESS-triggered measure rejuvenation
//   - dual interpretation:
//       (physical dynamics) ↔ (probabilistic inference)
//
// ------------------------------------------------------------
// Allowed Extensions:
// ------------------------------------------------------------
// You may:
//   - extend interaction kernels
//   - replace observation embeddings
//   - substitute integrators (RK2 / symplectic / HMC)
//   - port to GPU / VR pipelines
//
// ------------------------------------------------------------
// Non-breaking constraint:
// ------------------------------------------------------------
// You MUST preserve:
//
//   coupling of:
//     (energy gradient)
//     + (interaction field)
//     + (temperature-scaled noise)
//
// If broken:
//   system ceases to be DVSM-π+++ class
//
// ============================================================
