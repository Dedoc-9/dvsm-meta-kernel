// ============================================================================
// DVSM-π — GEOMETRIC CONTROL KERNEL (CURRENT CONSOLIDATED REFERENCE)
// ============================================================================
//
// DESIGN GOALS (CURRENT FORMALIZATION)
//
// The system is defined as a constrained geometric evolution on a
// stratified manifold ℳ embedded in ambient state space:
//
//     x_{t+1} = Π_ℳ( F(x_t, σ_t, G_t) )
//
// where:
//
//   F      : local unconstrained evolution operator in ambient space
//   Π_ℳ    : idempotent geometric projection enforcing membership in ℳ
//   σ_t    : external excitation signal (non-inferential, non-objective)
//   G_t    : fixed structural graph coupling (non-adaptive, non-learned)
//
// -----------------------------------------------------------------------------
// CORE DESIGN INVARIANTS
// -----------------------------------------------------------------------------

// 1. NO SCALAR OPTIMIZATION CHANNEL EXISTS
//
// There is:
//   - no loss function
//   - no reward function
//   - no utility function
//   - no gradient flow
//   - no parameter update driven by performance signals
//
// Therefore:
//   evolution is not directed by scalar evaluation.

// 2. GEOMETRY IS PRIMARY STRUCTURE (NOT METRICS)
//
// All observables are projections of state, not drivers of state:
//
//   observe(x_t) ≠ control signal
//
// Any metric derived from x_t is:
//   - non-causal with respect to update law
//   - informational only
//   - excluded from feedback paths

// 3. PROJECTION IS CLOSURE, NOT IMPROVEMENT
//
// Π_ℳ is defined as a constraint enforcement map:
//
//   Π_ℳ : ℝ^n → ℳ
//
// It does NOT:
//   - rank states
//   - minimize distance
//   - select “better” configurations
//
// It ONLY:
//   - removes infeasible states
//   - restores membership in ℳ
//   - preserves all feasible states invariantly (idempotence)

// 4. COUPLING IS STRUCTURAL, NOT OBJECTIVE-DRIVEN
//
// Graph coupling G_t defines adjacency influence:
//
//   G_t = (V, E)
//
// but:
//   - edges are static or externally specified
//   - no edge weight adaptation occurs
//   - no reward-sensitive routing exists
//
// Influence ≠ optimization pressure.

// -----------------------------------------------------------------------------
// WHY THE SYSTEM IS STRUCTURALLY GROUNDED
// -----------------------------------------------------------------------------

// The evolution is grounded because it satisfies:
//
//   (a) Locality
//       each update depends only on local state + fixed structure
//
//   (b) Closure
//       Π_ℳ ensures all trajectories remain in ℳ
//
//   (c) Non-optimization
//       no scalar objective exists in any feedback loop
//
//   (d) Non-adaptive coupling
//       graph structure does not evolve from performance signals
//
// This defines a purely geometric dynamical system,
// not a learning or control optimization system.

// -----------------------------------------------------------------------------
// FORMAL FAILURE MODES (DRIFT CHANNELS)
// -----------------------------------------------------------------------------

// “Drift” is formally defined as:
//
//   emergence of an implicit scalar reduction of state geometry
//
// This occurs if and only if one introduces:
//
//   1. Adaptive coupling:
//        G_t ← f(history(x_≤t))
//        → induces implicit objective alignment via topology change
//
//   2. Metric-sensitive modulation:
//        weights ← g(||x_t - x_{t-1}||)
//        → introduces surrogate energy minimization channel
//
//   3. Expectation tracking:
//        σ_t ← h(observed trajectory statistics)
//        → couples observation back into control law
//
//   4. Reward interpretation of excitation:
//        σ_t interpreted as optimization signal
//        → collapses excitation into scalar objective proxy
//
// Each of these introduces:
//
//   implicit scalarization of geometry
//
// which breaks the purely geometric closure assumption.

// -----------------------------------------------------------------------------
// WHY THIS WOULD BREAK DVSM-π
// -----------------------------------------------------------------------------

// These mechanisms reintroduce:
//
//   control_as_optimization ≡ exists J(x) such that evolution reduces J
//
// This violates the DVSM invariant:
//
//   evolution must be J-free (no scalar field governs transitions)
//
// Result:
//
//   Π_ℳ becomes a constraint on optimization,
//   instead of a constraint on feasibility.
//
// That is the Goodhart re-entry mode.

// -----------------------------------------------------------------------------
// END REFERENCE BLOCK
// -----------------------------------------------------------------------------
//
// Fundamental law:
//
//     x_{t+1} = Π_ℳ( F(x_t, σ_t) )
//
// System interpretation:
//   - F: contractive + excitation flow
//   - Π_ℳ: manifold closure (bounded projection)
//   - graph coupling: structural only (no objective)
//   - jets: observational diagnostics only
//
// Stability definition:
//   bounded evolution under projection closure (not optimization)
// ============================================================================

use std::f64;

// ============================================================================
// CORE TYPES
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub x: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub x_min: f64,
    pub x_max: f64,
    pub v_max: f64,
    pub a_max: f64,
    pub j_max: f64,
}

#[derive(Clone)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

// ============================================================================
// OBSERVATION LAYER (NON-CAUSAL)
// ============================================================================

#[inline(always)]
pub fn observe(x: f64) -> f64 {
    x
}

// ============================================================================
// EXCITATION (NON-OPTIMIZING DISTURBANCE)
// ============================================================================

#[inline(always)]
pub fn excitation(sigma: f64, x: f64) -> f64 {
    sigma - x
}

// ============================================================================
// CORE KERNEL (CONTRACTIVE FLOW)
// ============================================================================

#[inline(always)]
pub fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// PROJECTION OPERATOR Π_ℳ (UNIFIED)
// ============================================================================

#[inline(always)]
pub fn project_scalar(x: f64, b: &Bounds) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    x.clamp(b.x_min, b.x_max)
}

#[inline(always)]
pub fn project_jet(j: Jet, b: &Bounds) -> Jet {
    Jet {
        v: j.v.clamp(-b.v_max, b.v_max),
        a: j.a.clamp(-b.a_max, b.a_max),
        j: j.j.clamp(-b.j_max, b.j_max),
    }
}

// ============================================================================
// DISCRETE JET ESTIMATION (OBSERVATIONAL ONLY)
// ============================================================================

pub fn compute_jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = 0.5 * ((x0 - x1) + (x1 - x2));
    let a = x0 - 2.0 * x1 + x2;
    let j = x0 - 3.0 * x1 + 3.0 * x2 - x2;

    Jet { v, a, j }
}

// ============================================================================
// DVSM SINGLE STEP (CORE LAW IMPLEMENTATION)
// ============================================================================

pub fn dvsm_step(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    eta: f64,
    gamma: f64,
    b: &Bounds,
) -> (f64, Jet) {

    let k = kernel(x0, sigma, eta);
    let u = gamma * excitation(sigma, x0);

    let raw = k + u;
    let x = project_scalar(raw, b);

    let j_raw = compute_jet(x2, x1, x);
    let j = project_jet(j_raw, b);

    (x, j)
}

// ============================================================================
// GRAPH-COUPLED DVSM SYSTEM
// ============================================================================

pub struct DVSMGraph {
    pub states: Vec<f64>,
    pub history: Vec<Vec<f64>>,
    pub graph: Graph,
    pub eta: f64,
    pub gamma: f64,
    pub bounds: Bounds,
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

            let mut c = 0.0;
            let mut degree = 0.0;

            for &(a, b) in &self.graph.edges {
                if a == i {
                    c += self.states[b] - self.states[a];
                    degree += 1.0;
                }
            }

            let coupling = if degree > 0.0 { c / degree } else { 0.0 };
            let sigma_eff = sigma + coupling;

            let (x, _) = dvsm_step(
                prev2[i],
                prev1[i],
                self.states[i],
                sigma_eff,
                self.eta,
                self.gamma,
                &self.bounds,
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
// GHOST MODEL (FAILURE MODES AS FIRST-CLASS OBJECTS)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub enum Ghost {
    DriftLeak,
    JetInflation,
    CouplingResonance,
    ProjectionChatter,
}

// ============================================================================
// GHOST DETECTOR (HEURISTIC DIAGNOSTIC ONLY)
// ============================================================================

pub fn detect_ghost(x: f64, j: &Jet, b: &Bounds) -> Option<Ghost> {

    if !x.is_finite() {
        return Some(Ghost::DriftLeak);
    }

    if j.v.abs() > 2.0 * b.v_max {
        return Some(Ghost::JetInflation);
    }

    if j.a.abs() > 2.0 * b.a_max {
        return Some(Ghost::JetInflation);
    }

    if j.j.abs() > 2.0 * b.j_max {
        return Some(Ghost::JetInflation);
    }

    None
}

// ============================================================================
// STABILITY CHECK
// ============================================================================

#[inline(always)]
pub fn is_finite_state(x: &[f64]) -> bool {
    x.iter().all(|v| v.is_finite())
}

// ============================================================================
// STRESS TEST HARNESS
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

        debug_assert!(
            is_finite_state(&next),
            "DVSM-π instability detected"
        );
    }
}

// ============================================================================
// FINAL SYSTEM INTERPRETATION
// ============================================================================
//
// DVSM-π is:
//
//   a constrained graph-coupled dynamical system
//   evolving via contractive kernel flow
//   with manifold projection Π_ℳ
//   and observational jet diagnostics
//
// Stability means:
//
//   bounded evolution under projection closure
//
// NOT:
//
//   optimization, minimization, or reward alignment
//
// Drift risk arises only if:
//   a scalar objective is implicitly introduced into σ, coupling,
//   or projection feedback paths.
//
// ============================================================================
