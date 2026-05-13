// ============================================================================
// DVSM-π — ADDENDUM 8: CLOSURE PROTOCOL KERNEL (GOODHART RESIDUAL ELIMINATION)
// ============================================================================
// Author: Daniel J. Dillberg
// Purpose:
//   Structural elimination of Goodhart leakage via architectural separation:
//     (1) generation layer
//     (2) observation layer
//     (3) feasibility layer (Π_M)
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
// GRAPH (STATIC, NON-LEARNED)
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
// LAYER (1): GENERATION KERNEL (NO CONSTRAINTS)
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
// LAYER (2): OBSERVATION ONLY (NO FEEDBACK)
// ============================================================================

#[inline(always)]
fn jet(prev2: f64, prev1: f64, curr: f64) -> Jet {
    let v = curr - prev1;
    let v_prev = prev1 - prev2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// Diagnostic only (NEVER feeds control)
fn jet_energy(x: f64, j: &Jet) -> f64 {
    x * x + j.v * j.v + j.a * j.a + j.j * j.j
}

// ============================================================================
// LAYER (3): FEASIBILITY PROJECTION Π_M
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

// Hard constraint check (non-control)
fn feasible(x: f64, j: &Jet, b: &Bounds) -> bool {
    x >= b.x_min
        && x <= b.x_max
        && j.v.abs() <= b.v_max
        && j.a.abs() <= b.a_max
        && j.j.abs() <= b.j_max
}

// Identity-on-manifold projection property
fn project(x: f64, j: Jet, b: &Bounds) -> (f64, Jet) {
    (project_state(x, b), project_jet(j, b))
}

// ============================================================================
// GRAPH COUPLING (LINEAR ONLY — NO OPTIMIZATION SEMANTICS)
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
// EXCITATION CONSERVATION CHECK (DIAGNOSTIC ONLY)
// ============================================================================

fn excitation_energy(states: &[State]) -> f64 {
    states.iter().map(|s| s.x * s.x).sum()
}

// ============================================================================
// DVSM STEP (CLOSURE ARCHITECTURE)
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
            let raw = evolve_raw(snapshot[i].x, sigma + cx, &self.params);

            next[i].x = project_state(raw, &self.bounds);
        }

        // ----------------------------
        // OBSERVATION LAYER (NO FEEDBACK)
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
            let _ok = feasible(next[i].x, &j, &self.bounds);

            // IMPORTANT:
            // no correction loops, no optimization feedback
        }

        // ----------------------------
        // COMMIT (STATE UPDATE ONLY)
        // ----------------------------
        self.history.push(snapshot);
        self.states = next.clone();

        next
    }
}

// ============================================================================
// ADVERSARY MODEL (NON-OPTIMIZING PERTURBATION)
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
// STRESS TEST HARNESS
// ============================================================================

pub fn stress_test(system: &mut DVSM, adversary: Adversary, steps: usize, base: f64) {
    for t in 0..steps {
        let sigma = adversary.perturb(base, t);
        let out = system.step(sigma);

        debug_assert!(
            out.iter().all(|s| s.x.is_finite()),
            "DVSM closure violation: NaN/Inf detected"
        );
    }
}
// ============================================================================
// DVSM-π — FUNDAMENTAL INTRODUCTION BLOCK (GEOMETRIC CONTROL PRIMITIVE)
// ============================================================================
// Version: Closure Architecture Foundation (Post-Goodhart Separation Model)
// ============================================================================
// 1. FUNDAMENTAL OBJECT
// ============================================================================
//
// We define a system evolving over discrete time:
//
//     t ∈ ℕ
//
// with state:
//
//     x_t ∈ ℳ ⊂ ℝⁿ
//
// where ℳ is a constrained geometric manifold:
//
//     ℳ = { x | C(x) = 0, B(x) ≤ 0 }
//
// The system is NOT defined by optimization.
//
// It is defined by:
//
//     constrained evolution + geometric projection
//
// ============================================================================
// 2. FUNDAMENTAL INPUT STRUCTURE
// ============================================================================
//
// External signal:
//
//     σ_t ∈ ℝ
//
// Interpretation:
//
//     σ_t is NOT a target
//     σ_t is NOT a reward
//     σ_t is NOT a loss signal
//
// It is only:
//
//     a perturbation field acting on dynamics
//
// ============================================================================
// 3. CORE EVOLUTION PRINCIPLE
// ============================================================================
//
// DVSM-π defines a two-stage update:
//
//     (A) Unconstrained evolution:
//         x̃_{t+1} = F_A(x_t, σ_t)
//
//     (B) Geometric feasibility enforcement:
//         x_{t+1} = Π_ℳ(x̃_{t+1})
//
// Where:
//
//     F_A      : causal update kernel (not an optimizer)
//     Π_ℳ      : projection onto admissible state manifold
//
// ============================================================================
// 4. KEY STRUCTURAL PRINCIPLE
// ============================================================================
//
// Stability is NOT achieved via minimization.
//
// Stability is defined as:
//
//     invariance of trajectories under projection closure
//
// i.e.
//
//     if x_t ∈ ℳ, then x_{t+1} ∈ ℳ for all t
//
// This is a *closure condition*, not an optimization result.
//
// ============================================================================
// 5. JET STRUCTURE (DERIVED OBSERVABLES ONLY)
// ============================================================================
//
// Higher-order structure:
//
//     v_t = Δx_t
//     a_t = Δ²x_t
//     j_t = Δ³x_t
//
// These form a discrete jet:
//
//     J_t = (x_t, v_t, a_t, j_t)
//
// IMPORTANT:
//
//     Jets are NOT state variables.
//     Jets are NOT used in control.
//
// They are purely observational geometry.
//
// ============================================================================
// 6. GOODHART SEPARATION PRINCIPLE
// ============================================================================
//
// The system enforces strict causal separation:
//
//     generation layer  → produces x̃_{t+1}
//     projection layer  → enforces ℳ constraints
//     observation layer → computes jets, metrics, energies
//
// CRITICAL RULE:
//
//     Observables NEVER feed back into generation.
//
// This prevents:
//
//     metric → control coupling
//     optimization leakage
//     Goodhart collapse pathways
//
// ============================================================================
// 7. FUNDAMENTAL INVARIANT (CLOSURE PROPERTY)
// ============================================================================
//
// The defining property of DVSM-π:
//
//     Π_ℳ(Π_ℳ(x)) = Π_ℳ(x)
//
// and system evolution satisfies:
//
//     x_{t+1} ∈ ℳ  ∀ t
//
// This is the system’s stability definition.
//
// NOT:
//
//     convergence
//     minimization
//     equilibrium seeking
//
// ============================================================================
// 8. INTERPRETATION SUMMARY
// ============================================================================
//
// DVSM-π is:
//
//     a constrained dynamical system on a stratified manifold
//
// where:
//
//     dynamics = unconstrained generation
//     stability = geometric closure
//     metrics   = non-causal observables
//
// ============================================================================
// 9. CORE INTENT (IMPORTANT)
// ============================================================================
//
// The system explicitly avoids:
//
//     - scalar objectives
//     - reward functions
//     - loss minimization
//     - gradient alignment
//
// Instead it enforces:
//
//     structural feasibility + projection closure
//
// ============================================================================
// END OF FUNDAMENTAL BLOCK
// ============================================================================
