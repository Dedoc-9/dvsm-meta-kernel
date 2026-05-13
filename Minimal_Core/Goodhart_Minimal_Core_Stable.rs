// ============================================================================
// DVSM — GROUNDED CONTROL-BOUNDED CONSTRAINED DYNAMICS CORE
// ============================================================================
// Author: Daniel J. Dillberg
// Stage: DVSM-π Unified Constraint Kernel (Post-Goodhart Closure)
// ============================================================================
//
// CORE FORM:
//
//     x_{t+1} = Π_M( F(x_t, σ_t, G) )
//
// Interpretation:
//   F      : unconstrained causal evolution (ambient space)
//   Π_M    : projection onto admissible jet-manifold M
//   G      : graph coupling structure
//   σ_t    : external excitation field
//
// Stability Principle:
//   Stability is a property of admissible trajectories in jet-space,
//   not an optimization objective.
//
// ============================================================================
// GOODHART PRINCIPLE (INTERNALIZED FORM)
// ============================================================================
//
// Goodhart is NOT treated as a failure of optimization.
//
// It is treated as:
//
//     geometric decoupling between:
//         - observation space O(x)
//         - admissible trajectory manifold 𝓜 ⊂ J³(ℝⁿ)
//
// Therefore:
//
//     control operates in 𝓜
//     observations are projections O(𝓜)
//
// and NEVER inputs to F.
//
// ============================================================================
// SYSTEM SPACE
// ============================================================================
//
// State lives in jet bundle:
//
//     X = (x, v, a, j) ∈ J³(ℝⁿ)
//
// where:
//
//     x : position
//     v : velocity
//     a : acceleration
//     j : jerk
//
// Admissible manifold:
//
//     𝓜 ⊂ J³(ℝⁿ)
//
// ============================================================================
// CORE DESIGN SHIFT
// ============================================================================
//
// OLD:
//   x_{t+1} = F(x_t) - λ·penalties
//
// NEW:
//   x̃_{t+1} = F(x_t, σ_t, G)
//   x_{t+1}  = Π_M(x̃_{t+1})
//
// No scalar optimization exists in control loop.
//
// ============================================================================
// ENDPOINT PRINCIPLE
// ============================================================================
//
// Stability = invariance under Π_M ∘ F
// NOT minimization of energy
//
// ============================================================================

use std::collections::VecDeque;

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
// GRAPH STRUCTURE
// ============================================================================

#[derive(Clone, Debug)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

// ============================================================================
// PARAMETERS
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub eta: f64,     // contraction
    pub gamma: f64,   // excitation
    pub coupling: f64,
}

// ============================================================================
// BOUNDS (MANIFOLD CONSTRAINTS)
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
// CAUSAL KERNEL (F)
// ============================================================================

#[inline(always)]
fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// GRAPH LAPLACIAN (COUPLING FIELD)
// ============================================================================

fn laplacian(graph: &Graph, x: &[f64], i: usize) -> f64 {
    let mut sum = 0.0;
    let mut deg = 0.0;

    for &(a, b) in &graph.edges {
        if a == i {
            sum += x[b] - x[a];
            deg += 1.0;
        }
        if b == i {
            sum += x[a] - x[b];
            deg += 1.0;
        }
    }

    if deg > 0.0 { sum / deg } else { 0.0 }
}

// ============================================================================
// EXCITATION MODEL
// ============================================================================

#[inline(always)]
fn excitation(sigma: f64, expected: f64) -> f64 {
    sigma - expected
}

// ============================================================================
// DISCRETE JET RECONSTRUCTION (CONSISTENT FORM)
// ============================================================================
//
// IMPORTANT:
// Jet is derived from trajectory, NOT stored as independent state.
// ============================================================================

fn compute_jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// ============================================================================
// MANIFOLD PROJECTION Π_M
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

// ============================================================================
// LYAPUNOV VIEW (OBSERVATIONAL ONLY)
// ============================================================================
//
// NOTE:
// This is NOT part of control.
// It is a diagnostic invariant check only.
// ============================================================================

fn lyapunov(x: f64, j: &Jet, b: &Bounds) -> f64 {
    x * x
        + (j.v / b.v_max).powi(2)
        + (j.a / b.a_max).powi(2)
        + (j.j / b.j_max).powi(2)
}

// ============================================================================
// DVSM NODE UPDATE (π-CORE STEP)
// ============================================================================

fn dvsm_step_node(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    lap: f64,
    p: Params,
    b: Bounds,
    expected: f64,
) -> (f64, Jet) {

    // ------------------------------------------------------------
    // 1. CAUSAL KERNEL EVOLUTION
    // ------------------------------------------------------------
    let k = kernel(x0, sigma + lap, p.eta);

    // ------------------------------------------------------------
    // 2. EXCITATION (NO OPTIMIZATION ROLE)
    // ------------------------------------------------------------
    let u = p.gamma * excitation(sigma, expected);

    let x_raw = k + u;

    // ------------------------------------------------------------
    // 3. STATE PROJECTION (Π_M)
    // ------------------------------------------------------------
    let x_proj = project_state(x_raw, &b);

    // ------------------------------------------------------------
    // 4. JET RECONSTRUCTION (OBSERVATION)
    // ------------------------------------------------------------
    let j_raw = compute_jet(x2, x1, x_proj);

    // ------------------------------------------------------------
    // 5. JET PROJECTION (MANIFOLD CLOSURE)
    // ------------------------------------------------------------
    let j_proj = project_jet(j_raw, &b);

    (x_proj, j_proj)
}

// ============================================================================
// DVSM GRAPH SYSTEM
// ============================================================================

pub struct DVSMGraph {
    pub states: Vec<f64>,
    pub history: VecDeque<Vec<f64>>,
    pub graph: Graph,
    pub params: Params,
    pub bounds: Bounds,
    pub expected_input: f64,
}

impl DVSMGraph {

    pub fn step(&mut self, sigma: f64) -> Vec<f64> {

        let prev2 = self.history
            .get(self.history.len().saturating_sub(2))
            .cloned()
            .unwrap_or(self.states.clone());

        let prev1 = self.states.clone();

        let mut next = self.states.clone();

        let xs: Vec<f64> = self.states.clone();

        for i in 0..self.states.len() {

            let lap = laplacian(&self.graph, &xs, i);

            let (x, _jet) = dvsm_step_node(
                prev2[i],
                prev1[i],
                self.states[i],
                sigma,
                lap,
                self.params,
                self.bounds,
                self.expected_input,
            );

            next[i] = x;
        }

        self.history.push_back(self.states.clone());
        self.states = next.clone();

        next
    }
}

// ============================================================================
// ADVERSARY MODEL (BOUNDED EXCITATION PERTURBATION)
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
// STRESS TEST HARNESS (INVARIANT CHECK ONLY)
// ============================================================================

pub fn stress_test(
    system: &mut DVSMGraph,
    adv: Adversary,
    steps: usize,
    base_sigma: f64,
) {

    for t in 0..steps {
        let sigma = adv.perturb(base_sigma, t);
        let _ = system.step(sigma);

        debug_assert!(
            system.states.iter().all(|x| x.is_finite()),
            "DVSM invariant violation: non-finite state"
        );
    }
}

// ============================================================================
// GOODHART NOTE (EMBEDDED CONCEPTUAL LAYER)
// ============================================================================
//
// Goodhart’s Law in DVSM:
//
//   Not a failure of optimization.
//
//   A failure of representational closure.
//
// If:
//
//   O(x) becomes control input
//
// then:
//
//   manifold projection is bypassed → instability emerges.
//
// DVSM prevents this by design:
//
//   O(x) ∉ F(x)
//
// Observations are epiphenomenal only.
//
// ============================================================================
// FINAL SYSTEM CLASSIFICATION
// ============================================================================
//
// DVSM is:
//
//   - constrained dynamical system on jet manifold
//   - graph-coupled contraction flow
//   - projection-stabilized evolution operator
//   - observation-decoupled control system
//
// NOT:
//
//   - optimization system
//   - reward engine
//   - penalty minimizer
//
// ============================================================================
// END FILE
// ============================================================================
