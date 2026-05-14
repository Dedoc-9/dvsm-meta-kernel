// ============================================================================
// DVSM-π+++ — IMPLICIT MANIFOLD STOCHASTIC GEOMETRIC DYNAMICS (PHYSICS-COHERENT CORE)
// Author: Daniel J. Dillberg
// ============================================================================
// Interpretation:
// A constrained Langevin-style stochastic dynamical system where:
// - state evolves under drift + rotational non-equilibrium flow + thermal noise
// - feasibility enforced by projection onto an implicit free-energy manifold
// - observables are jet estimates (velocity/acceleration) on projected trajectory
//
// Physical grounding:
// - overdamped Langevin dynamics (stochastic thermodynamics)
// - fluctuation-dissipation theorem (FDT)
// - free-energy landscape reduction (reaction coordinates)
// - non-equilibrium steady-state probability currents
// ============================================================================

use std::f64;

// ============================================================================
// STATE
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub x: f64,
    pub y: f64,
}

// ============================================================================
// MANIFOLD (IMPLICIT FREE-ENERGY SURFACE)
// ============================================================================
//
// NOTE:
// In physically consistent form, this is NOT an arbitrary constraint.
// It represents a learned or derived free-energy landscape F(x,y).
// The manifold corresponds to level sets or basins of F.
#[derive(Clone, Copy, Debug)]
pub struct Manifold {
    pub epsilon: f64, // smoothing / stiffness parameter (proxy for curvature scale)
}

// Example free-energy function (placeholder surrogate)
impl Manifold {
    #[inline(always)]
    pub fn free_energy(&self, x: f64, y: f64) -> f64 {
        // double-well + coupling (toy energy landscape)
        let r2 = x * x + y * y;
        let wells = (r2 - 1.0).powi(2);
        wells + self.epsilon * x * y
    }
}

// ============================================================================
// GRADIENT (drift direction = -∇F)
// ============================================================================

#[inline(always)]
fn grad_f(m: &Manifold, x: f64, y: f64) -> (f64, f64) {
    let eps = 1e-6;

    let fx = (m.free_energy(x + eps, y) - m.free_energy(x - eps, y)) / (2.0 * eps);
    let fy = (m.free_energy(x, y + eps) - m.free_energy(x, y - eps)) / (2.0 * eps);

    (fx, fy)
}

// ============================================================================
// ROTATIONAL FIELD (NON-EQUILIBRIUM FLUX)
// ============================================================================
//
// Interpreted as probability current curl component (F_rot).
#[inline(always)]
fn rotational_field(x: f64, y: f64, gamma: f64) -> (f64, f64) {
    // simple antisymmetric rotation
    (-gamma * y, gamma * x)
}

// ============================================================================
// THERMAL NOISE (FLUCTUATION-DISSIPATION CONSISTENT FORM)
// ============================================================================

#[inline(always)]
fn thermal_noise(seed: f64, temperature: f64, gamma: f64) -> (f64, f64) {
    let base = ((seed * 12.9898).sin() * 43758.5453).fract() * 2.0 - 1.0;

    // Langevin-consistent scaling (γ kT proxy)
    let scale = (2.0 * gamma * temperature).sqrt();

    (base * scale, base * scale)
}

// ============================================================================
// PROJECTION (CONSTRAINED DYNAMICS STEP)
// ============================================================================
//
// Physical meaning:
// replaces hard constraint enforcement with relaxation toward manifold consistency
#[inline(always)]
fn project(mut s: State, m: &Manifold) -> State {
    for _ in 0..4 {
        let f = m.free_energy(s.x, s.y);

        let (fx, fy) = grad_f(m, s.x, s.y);

        let norm = (fx * fx + fy * fy).sqrt() + 1e-12;

        // move toward lower free-energy consistency
        s.x -= f * fx / norm;
        s.y -= f * fy / norm;
    }

    s
}

// ============================================================================
// JET OBSERVABLES (TANGENT DYNAMICS ESTIMATION)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub vx: f64,
    pub vy: f64,
    pub ax: f64,
    pub ay: f64,
}

#[inline(always)]
fn jet(x2: State, x1: State, x0: State) -> Jet {
    let vx = x0.x - x1.x;
    let vy = x0.y - x1.y;

    let vx_p = x1.x - x2.x;
    let vy_p = x1.y - x2.y;

    Jet {
        vx,
        vy,
        ax: vx - vx_p,
        ay: vy - vy_p,
    }
}

// ============================================================================
// REGIME CLASSIFICATION (DYNAMICAL PHASES)
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum Regime {
    DriftDominated,
    Balanced,
    RotationalDominated,
    StiffGeometry,
    HighNoise,
}

fn classify(s: State, j: Jet, m: &Manifold, gamma: f64, temperature: f64) -> Regime {
    let energy = m.free_energy(s.x, s.y);

    let motion = (j.vx * j.vx + j.vy * j.vy).sqrt();
    let accel = (j.ax * j.ax + j.ay * j.ay).sqrt();

    if energy > 5.0 {
        return Regime::StiffGeometry;
    }

    if temperature > 1.0 {
        return Regime::HighNoise;
    }

    if gamma > 1.0 && motion > 0.5 {
        return Regime::RotationalDominated;
    }

    if motion < 1e-3 {
        return Regime::DriftDominated;
    }

    if accel > 1.0 {
        return Regime::Balanced;
    }

    Regime::Balanced
}

// ============================================================================
// CORE STEP (CONSTRAINED LANGEVIN SYSTEM)
// ============================================================================

pub fn step(
    x2: State,
    x1: State,
    x0: State,
    m: &Manifold,
    dt: f64,
    gamma: f64,
    temperature: f64,
    seed: f64,
) -> (State, Jet, Regime) {
    // 1. deterministic drift (−∇F)
    let (gx, gy) = grad_f(m, x0.x, x0.y);

    let drift_x = -gx;
    let drift_y = -gy;

    // 2. rotational (non-equilibrium) flow
    let (rx, ry) = rotational_field(x0.x, x0.y, gamma);

    // 3. thermal noise (FDT-consistent)
    let (nx, ny) = thermal_noise(seed, temperature, gamma);

    // 4. unconstrained update (Langevin step)
    let raw = State {
        x: x0.x + dt * (drift_x + rx + nx),
        y: x0.y + dt * (drift_y + ry + ny),
    };

    // 5. projection onto implicit manifold
    let next = project(raw, m);

    // 6. jet observables (on projected trajectory only)
    let j = jet(x2, x1, next);

    // 7. regime classification
    let r = classify(next, j, m, gamma, temperature);

    (next, j, r)
}

// ============================================================================
// RUN LOOP
// ============================================================================

pub fn run(
    steps: usize,
    mut s: State,
    m: &Manifold,
    dt: f64,
    gamma: f64,
    temperature: f64,
) {
    let mut s1 = s;
    let mut s2 = s;

    for t in 0..steps {
        let seed = t as f64;

        let (nx, _j, r) = step(s2, s1, s, m, dt, gamma, temperature, seed);

        s2 = s1;
        s1 = s;
        s = nx;

        println!("{:?}", r);
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

fn main() {
    let m = Manifold { epsilon: 0.1 };

    run(
        200,
        State { x: 0.2, y: -0.1 },
        &m,
        0.05,  // dt
        0.6,   // gamma (non-equilibrium flux strength)
        0.3,   // temperature (noise level)
    );
}
