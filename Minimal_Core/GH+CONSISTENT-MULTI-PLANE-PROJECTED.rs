// ============================================================================
// DVSM-π — UNIFIED CONSOLIDATED CORE (v1 + v2 + GH ADDENDA)
// Author: Daniel J. Dillberg
// ============================================================================
// Status: Constraint Geometry + Graph-Coupled + Measure-Lifted System
// ============================================================================
// ============================================================================
// DVSM-π — UNIFIED CONSTRAINT GEOMETRY SYSTEM (INTRO / DEV NOTES)
// ============================================================================
//
// SYSTEM SUMMARY
//
// DVSM-π is a constrained dynamical system operating on a coupled state graph
// under strict geometric feasibility enforcement.
//
// The system evolves according to:
//
//     x_{t+1} = Π_M( F(x_t, σ_t, G_t) )
//
// where:
//   F     : unconstrained local evolution operator (proposal dynamics)
//   Π_M   : projection operator enforcing manifold feasibility
//   G_t   : structural graph coupling (non-objective interaction)
//   σ_t   : external excitation signal (not inferred, not optimized)
//
// ---------------------------------------------------------------------------
// CORE DESIGN PRINCIPLE
// ---------------------------------------------------------------------------
//
// This system is NOT:
//
//   - an optimizer
//   - a learning system
//   - a reward-maximizing process
//   - a probabilistic model of inference
//
// It IS:
//
//   - a constrained geometric evolution system
//   - a projection-closed dynamical flow
//   - a graph-coupled state propagation field
//
// ---------------------------------------------------------------------------
// ARCHITECTURAL INVARIANTS
// ---------------------------------------------------------------------------
//
// (I1) NO OPTIMIZATION LOOP EXISTS
//     There is no objective function, loss, reward, or utility signal.
//
// (I2) PROJECTION IS FINAL AUTHORITY
//     Any invalid state is removed via Π_M after proposal generation.
//     No soft constraints or penalty gradients are used.
//
// (I3) GRAPH IS STRUCTURAL ONLY
//     Edges define influence topology, not performance pressure.
//
// (I4) OBSERVATIONS ARE NON-CAUSAL
//     Jet quantities (v, a, j) are reconstructed diagnostics only.
//
// (I5) NO DERIVATIVE FEEDBACK
//     No velocity/acceleration/jerk terms influence state evolution.
//
// ---------------------------------------------------------------------------
// FAILURE MODEL (EXPLICIT)
// ---------------------------------------------------------------------------
//
// Instability can only arise through:
//
//   - divergence before projection (handled by Π_M)
//   - degenerate graph coupling (topological imbalance)
//   - unbounded excitation σ_t (external forcing)
//
// There is no hidden optimization channel in the system design.
//
// ---------------------------------------------------------------------------
// GEOMETRIC INTERPRETATION
// ---------------------------------------------------------------------------
//
// The system evolves on a constrained manifold M embedded in ℝⁿ:
//
//   - F explores ambient space
//   - Π_M retracts onto feasibility surface
//   - trajectories are valid only after projection closure
//
// The manifold is not learned or optimized — it is enforced.
//
// ---------------------------------------------------------------------------
// IMPLEMENTATION NOTE
// ---------------------------------------------------------------------------
//
// This file merges three conceptual layers:
//
//   1. Deterministic DVSM kernel dynamics
//   2. Graph-coupled interaction field
//   3. Multi-plane geometric projection system
//
// All layers are structurally compositional, not hierarchical in control.
//
// ---------------------------------------------------------------------------
// DEV WARNING (IMPORTANT)
// ---------------------------------------------------------------------------
//
// Any modification that introduces:
//
//   - scalar scoring functions
//   - adaptive weighting based on past trajectory quality
//   - feedback from jets into state updates
//   - minimization of distance in any metric space
//
// will convert this system into an implicit optimizer,
// violating DVSM-π structural constraints.
//
// ---------------------------------------------------------------------------
// END OF INTRO / DEV NOTES
// ============================================================================

use std::f64;

// ============================================================================
// CORE STATE
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub x: f64,
}

// ============================================================================
// JET (OBSERVATIONAL ONLY)
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// GRAPH STRUCTURE
// ============================================================================

#[derive(Clone)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

// ============================================================================
// BOUNDS / CONSTRAINT SPACE
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
// MULTI-PLANE CONSTRAINT SYSTEM (v2)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub min: f64,
    pub max: f64,
    pub weight: f64,
}

// ============================================================================
// DVSM CORE SYSTEM
// ============================================================================

pub struct DVSMCore {
    pub eta: f64,
    pub gamma: f64,
    pub coupling: f64,
    pub graph: Graph,
    pub planes: Vec<Plane>,
    pub bounds: Bounds,
}

// ============================================================================
// OBSERVATION LAYER (NO CAUSAL ROLE)
// ============================================================================

#[inline(always)]
pub fn observe(x: f64) -> f64 {
    x
}

// ============================================================================
// KERNEL (CONTRACTIVE FLOW)
// ============================================================================

#[inline(always)]
fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// EXCITATION (NON-OBJECTIVE FORCE)
// ============================================================================

#[inline(always)]
fn excitation(sigma: f64, x: f64) -> f64 {
    sigma - x
}

// ============================================================================
// COUPLING FIELD (GRAPH STRUCTURE ONLY)
// ============================================================================

fn coupling_field(x: f64, neighbors: &[f64], c: f64) -> f64 {
    let mut sum = 0.0;
    for &n in neighbors {
        sum += c * (n - x);
    }
    sum
}

// ============================================================================
// MULTI-PLANE PROJECTION Π_M
// ============================================================================

fn project_plane(x: f64, p: &Plane) -> f64 {
    x.clamp(p.min, p.max)
}

fn pi_m(x: f64, planes: &[Plane]) -> f64 {
    let mut num = 0.0;
    let mut den = 0.0;

    for p in planes {
        let px = project_plane(x, p);
        num += px * p.weight;
        den += p.weight;
    }

    num / (den + 1e-12)
}

// ============================================================================
// BOUNDS PROJECTION Π_ℳ (JET SPACE)
// ============================================================================

fn project_bounds(x: f64, b: &Bounds) -> f64 {
    x.clamp(b.x_min, b.x_max)
}

fn project_jet(j: Jet, b: &Bounds) -> Jet {
    Jet {
        v: j.v.clamp(-b.v_max, b.v_max),
        a: j.a.clamp(-b.a_max, b.a_max),
        j: j.j.clamp(-b.j_max, b.j_max),
    }
}

// ============================================================================
// JET RECONSTRUCTION (OBSERVATIONAL ONLY)
// ============================================================================

fn compute_jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// ============================================================================
// EVOLUTION CORE (UNCONSTRAINED PROPOSAL)
// ============================================================================

fn evolve(x: f64, sigma: f64, cx: f64, eta: f64, gamma: f64) -> f64 {
    let k = kernel(x, sigma + cx, eta);
    let u = gamma * (sigma - x);
    k + u
}

// ============================================================================
// GRAPH COUPLING
// ============================================================================

fn coupling(graph: &Graph, states: &[f64], i: usize, c: f64) -> f64 {
    let mut acc = 0.0;
    let mut deg = 0.0;

    for &(a, b) in &graph.edges {
        if a == i {
            acc += states[b] - states[a];
            deg += 1.0;
        }
    }

    if deg > 0.0 { c * acc / deg } else { 0.0 }
}

// ============================================================================
// SINGLE STEP EVOLUTION
// ============================================================================

pub fn dvsm_step(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    neighbors: &[f64],
    system: &DVSMCore,
) -> (f64, Jet) {

    let idx_coupling = coupling_field(x0, neighbors, system.coupling);

    let raw = evolve(
        x0,
        sigma,
        idx_coupling,
        system.eta,
        system.gamma,
    );

    // ------------------------------------------------------------
    // PROJECT TO MULTI-PLANE MANIFOLD
    // ------------------------------------------------------------
    let mut projected = pi_m(raw, &system.planes);

    // fallback safety: enforce bounds too
    projected = project_bounds(projected, &system.bounds);

    let jet = compute_jet(x2, x1, projected);
    let jet = project_jet(jet, &system.bounds);

    (projected, jet)
}

// ============================================================================
// GRAPH SYSTEM WRAPPER
// ============================================================================

pub struct DVSMGraph {
    pub states: Vec<f64>,
    pub history: Vec<Vec<f64>>,
    pub graph: Graph,
    pub system: DVSMCore,
}

impl DVSMGraph {

    pub fn step(&mut self, sigma: f64) -> Vec<f64> {

        let prev2 = self.history
            .last()
            .cloned()
            .unwrap_or(self.states.clone());

        let prev1 = self.states.clone();
        let mut next = self.states.clone();

        for i in 0..self.states.len() {

            let mut neigh = vec![];
            for &(a, b) in &self.graph.edges {
                if a == i {
                    neigh.push(self.states[b]);
                }
            }

            let (x, _) = dvsm_step(
                prev2[i],
                prev1[i],
                self.states[i],
                sigma,
                &neigh,
                &self.system,
            );

            next[i] = x;
        }

        self.history.push(self.states.clone());
        self.states = next.clone();

        next
    }
}

// ============================================================================
// ADVERSARY (STRESS ONLY)
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
// STABILITY CHECK
// ============================================================================

pub fn is_finite_state(x: &[f64]) -> bool {
    x.iter().all(|v| v.is_finite())
}

// ============================================================================
// STRESS TEST
// ============================================================================

pub fn stress_test(
    system: &mut DVSMGraph,
    adversary: Adversary,
    steps: usize,
    base_sigma: f64,
) {
    for t in 0..steps {
        let sigma = adversary.perturb(base_sigma, t);
        let next = system.step(sigma);

        debug_assert!(is_finite_state(&next));
    }
}

// ============================================================================
// OPTIONAL: GH MEASURE LAYER (Addendum 6 style)
// ============================================================================

pub struct Density {
    pub p: Vec<f64>,
}

pub fn normalize(d: &mut Density) {
    let sum: f64 = d.p.iter().sum();
    if sum > 0.0 {
        for v in &mut d.p {
            *v /= sum;
        }
    }
}

pub fn entropy(d: &Density) -> f64 {
    let mut s = 0.0;
    for &p in &d.p {
        if p > 1e-12 {
            s -= p * p.log2();
        }
    }
    s
}
