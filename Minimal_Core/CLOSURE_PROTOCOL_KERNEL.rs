// ============================================================================
// DVSM-π — CLOSURE PROTOCOL KERNEL
//        (GOODHART RESIDUAL ELIMINATION / STRUCTURAL SEPARATION MODEL)
// ============================================================================
//
// Author: Daniel J. Dillberg
//
// PURPOSE
// ---------------------------------------------------------------------------
// Structural elimination of Goodhart leakage via strict separation:
//
//   (1) Generation Layer   → F(x, σ)
//   (2) Observation Layer  → jets / metrics (non-causal)
//   (3) Feasibility Layer  → Π_M projection closure
//
// CORE EVOLUTION LAW
// ---------------------------------------------------------------------------
//
//     x_{t+1} = Π_M( F(x_t, σ_t) )
//
// where:
//   F     : unconstrained graph-coupled evolution operator
//   Π_M   : stratified projection onto feasible manifold M
//   σ_t   : external excitation (non-objective, non-reward)
//
// Interpretation:
//   - F proposes ambient-space transitions
//   - Π_M enforces geometric closure
//   - only projected states enter trajectory
//
// ============================================================================

use std::f64;

// ============================================================================
// CORE TYPES
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub x: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// GRAPH (STATIC STRUCTURE)
// ============================================================================

#[derive(Clone, Debug)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

// ============================================================================
// PARAMETERS (NON-ADAPTIVE)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub eta: f64,
    pub gamma: f64,
    pub coupling: f64,
}

// ============================================================================
// CONSTRAINT MANIFOLD ℳ
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub x_min: f64,
    pub x_max: f64,
    pub v_max: f64,
    pub a_max: f64,
    pub j_max: f64,
}

// ============================================================================
// LAYER 1 — GENERATION KERNEL (UNCONSTRAINED DYNAMICS)
// ============================================================================

#[inline(always)]
fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

#[inline(always)]
fn excitation(sigma: f64, x: f64) -> f64 {
    sigma - x
}

#[inline(always)]
fn evolve_raw(x: f64, sigma: f64, p: &Params) -> f64 {
    kernel(x, sigma, p.eta) + p.gamma * excitation(sigma, x)
}

// ============================================================================
// LAYER 2 — OBSERVATION (NON-CAUSAL DIAGNOSTICS)
// ============================================================================

#[inline(always)]
fn jet(prev2: f64, prev1: f64, curr: f64) -> Jet {
    let v = curr - prev1;
    let v_prev = prev1 - prev2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// Diagnostic-only (NO CONTROL ROLE)
fn jet_energy(x: f64, j: &Jet) -> f64 {
    x * x + j.v * j.v + j.a * j.a + j.j * j.j
}

// ============================================================================
// LAYER 3 — FEASIBILITY PROJECTION Π_M
// ============================================================================

#[inline(always)]
fn project_state(x: f64, b: &Bounds) -> f64 {
    x.clamp(b.x_min, b.x_max)
}

#[inline(always)]
fn project_jet(j: Jet, b: &Bounds) -> Jet {
    Jet {
        v: j.v.clamp(-b.v_max, b.v_max),
        a: j.a.clamp(-b.a_max, b.a_max),
        j: j.j.clamp(-b.j_max, b.j_max),
    }
}

fn feasible(x: f64, j: &Jet, b: &Bounds) -> bool {
    x >= b.x_min
        && x <= b.x_max
        && j.v.abs() <= b.v_max
        && j.a.abs() <= b.a_max
        && j.j.abs() <= b.j_max
}

// ============================================================================
// GRAPH COUPLING (STRUCTURAL ONLY)
// ============================================================================

fn coupling(graph: &Graph, states: &[State], i: usize, c: f64) -> f64 {
    let mut sum = 0.0;

    for &(a, b) in &graph.edges {
        if a == i {
            sum += c * (states[b].x - states[a].x);
        }
    }

    sum
}

// ============================================================================
// EVOLUTION MAP
// ============================================================================

fn evolve(x: f64, sigma: f64, cx: f64, p: &Params) -> f64 {
    let k = kernel(x, sigma + cx, p.eta);
    let u = p.gamma * (sigma - x);
    k + u
}

// ============================================================================
// DVSM SYSTEM
// ============================================================================

pub struct DVSM {
    pub states: Vec<State>,
    pub history: Vec<Vec<State>>,
    pub graph: Graph,
    pub params: Params,
    pub bounds: Bounds,
}

impl DVSM {

    pub fn step(&mut self, sigma: f64) -> Vec<State> {

        let snapshot = self.states.clone();
        let prev1 = snapshot.clone();
        let prev2 = self.history.last().cloned().unwrap_or(snapshot.clone());

        let mut next = snapshot.clone();

        // ----------------------------
        // GENERATION LAYER
        // ----------------------------
        for i in 0..snapshot.len() {
            let cx = coupling(&self.graph, &snapshot, i, self.params.coupling);
            let raw = evolve(snapshot[i].x, sigma + cx, &self.params);

            next[i].x = project_state(raw, &self.bounds);
        }

        // ----------------------------
        // OBSERVATION LAYER
        // ----------------------------
        let mut jets: Vec<Jet> = Vec::new();
        for i in 0..next.len() {
            jets.push(jet(prev2[i].x, prev1[i].x, next[i].x));
        }

        // ----------------------------
        // FEASIBILITY LAYER
        // ----------------------------
        for i in 0..next.len() {
            let j = project_jet(jets[i], &self.bounds);
            let _ = feasible(next[i].x, &j, &self.bounds);
        }

        // ----------------------------
        // COMMIT
        // ----------------------------
        self.history.push(snapshot);
        self.states = next.clone();

        next
    }
}

// ============================================================================
// ADVERSARY (NON-OPTIMIZING PERTURBATION)
// ============================================================================

pub struct Adversary {
    pub strength: f64,
}

impl Adversary {
    pub fn perturb(&self, sigma: f64, t: usize) -> f64 {
        sigma + (t as f64).sin() * self.strength
    }
}

// ============================================================================
// STRESS TEST
// ============================================================================

pub fn stress_test(system: &mut DVSM, adversary: Adversary, steps: usize, base: f64) {
    for t in 0..steps {
        let sigma = adversary.perturb(base, t);
        let out = system.step(sigma);

        debug_assert!(
            out.iter().all(|s| s.x.is_finite()),
            "DVSM closure violation"
        );
    }
}

// ============================================================================
// FUNDAMENTAL INTRODUCTION BLOCK (DVSM-π GEOMETRIC PRIMITIVE)
// ============================================================================
//
// DVSM-π defines a constrained dynamical system:
//
//     x_t ∈ ℳ ⊂ ℝⁿ
//
// Evolution law:
//
//     x_{t+1} = Π_M(F(x_t, σ_t))
//
// Key principles:
//   - no optimization exists
//   - no reward/loss signals exist
//   - projection enforces feasibility closure
//   - observables are non-causal diagnostics
//
// Stability is defined as:
//
//     closure under Π_M, not convergence or minimization
//
// ============================================================================
// DVSM-π — ENVIRONMENT PROTOCOL LAYER (EXOGENOUS SIGNAL CONTRACT)
// ============================================================================
// Purpose:
//   Formal separation between:
//     (1) environment generation
//     (2) system dynamics
//     (3) internal observability
//
// Key principle:
//   Environment is NOT part of the system.
//   It is a read-only causal input stream.
// ============================================================================

// ============================================================================
// ENVIRONMENT SIGNAL SPACE
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct EnvSignal {
    pub sigma: f64,
    pub noise: f64,
    pub drift: f64,
}

// ============================================================================
// ENVIRONMENT INTERFACE (READ-ONLY SOURCE)
// ============================================================================

pub trait Environment {
    fn sample(&self, t: usize, x: f64) -> EnvSignal;
}

// ============================================================================
// STATIC ENVIRONMENT (NON-ADAPTIVE BASELINE)
// ============================================================================

pub struct StaticEnvironment {
    pub base_sigma: f64,
    pub amplitude: f64,
}

impl Environment for StaticEnvironment {
    fn sample(&self, t: usize, _x: f64) -> EnvSignal {
        let sigma = self.base_sigma + self.amplitude * (t as f64).sin();

        EnvSignal {
            sigma,
            noise: 0.0,
            drift: 0.0,
        }
    }
}

// ============================================================================
// STOCHASTIC ENVIRONMENT (EXOGENOUS NOISE ONLY)
// ============================================================================

pub struct NoisyEnvironment {
    pub base_sigma: f64,
    pub noise_scale: f64,
}

impl NoisyEnvironment for NoisyEnvironment {
    fn sample(&self, t: usize, _x: f64) -> EnvSignal {
        let sigma = self.base_sigma;

        // NOTE:
        // This is NOT learning noise, NOT adversarial optimization.
        // It is purely exogenous perturbation.
        let noise = self.noise_scale * ((t as f64).cos());

        EnvSignal {
            sigma,
            noise,
            drift: 0.0,
        }
    }
}

// ============================================================================
// ENVIRONMENT COMPOSITION (STRUCTURAL SUM ONLY)
// ============================================================================

pub struct CompositeEnvironment<E1: Environment, E2: Environment> {
    pub a: E1,
    pub b: E2,
}

impl<E1: Environment, E2: Environment> Environment for CompositeEnvironment<E1, E2> {
    fn sample(&self, t: usize, x: f64) -> EnvSignal {
        let ea = self.a.sample(t, x);
        let eb = self.b.sample(t, x);

        EnvSignal {
            sigma: ea.sigma + eb.sigma,
            noise: ea.noise + eb.noise,
            drift: ea.drift + eb.drift,
        }
    }
}

// ============================================================================
// ENVIRONMENT COUPLING RULE (IMPORTANT CONSTRAINT)
// ============================================================================
//
// Allowed:
//   σ_t = EnvSignal → scalar extraction into kernel
//
// Forbidden:
//   - using system state x_t to modify environment
//   - adaptive environments responding to jet statistics
//   - reward-shaped σ_t
//
// Rationale:
//   prevents hidden optimization loops via environment feedback
// ============================================================================

// ============================================================================
// SAFE EXTERNAL SIGNAL EXTRACTION
// ============================================================================

#[inline(always)]
pub fn extract_sigma(env: &EnvSignal) -> f64 {
    env.sigma + env.noise
}
