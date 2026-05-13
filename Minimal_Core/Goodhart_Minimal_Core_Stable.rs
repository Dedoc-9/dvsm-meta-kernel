// ============================================================================
// DVSM — GROUNDED CONTROL-BOUNDED CONSTRAINED DYNAMICS CORE
// ============================================================================
// Author: Daniel J. Dillberg
// Evolution Stage: Stability Layer Rebalanced (Post-Goodhart Hardening)
// Status: Mathematical Endpoint Form (Minimal, Constraint-Complete Kernel)
// ============================================================================
//
// ============================================================================
// SYSTEM EVOLUTION (REFINED SEMANTIC HISTORY)
// ============================================================================
//
// The system evolves through three conceptual regimes:
//
// ---------------------------------------------------------------------------
// (1) POINTWISE OPTIMIZATION REGIME
// ---------------------------------------------------------------------------
// x_{t+1} = F(x_t) - λ · S(x_t)
//
// Interpretation:
//   - Stability enforced via scalar penalties
//   - System optimizes a pointwise objective
//
// Failure mode:
//   - Goodhart collapse (metric becomes target)
//   - Exploitability via boundary saturation
//
// ---------------------------------------------------------------------------
// (2) TRAJECTORY REGULARIZATION REGIME
// ---------------------------------------------------------------------------
// x_{t+1} = F(x_t) - λ₁‖a_t‖² - λ₂‖j_t‖² - λ₃Δ_t - λ₄H_t
//
// Interpretation:
//   - Adds derivative-aware smoothing (velocity/accel/jerk)
//   - Introduces drift memory (H_t)
//
// Failure mode:
//   - Over-damping under sustained input
//   - Loss of expressive dynamics ("trajectory freezing")
//   - Implicit return to scalar optimization pressure
//
// Key issue:
//   Still fundamentally a PENALTY system (not constraint system)
//
// ---------------------------------------------------------------------------
// (3) CONSTRAINED CONTROL GEOMETRY REGIME (CURRENT FORM)
// ---------------------------------------------------------------------------
// x̃_{t+1} = F_A(x_t, σ_t) + γ · (σ_t − P(x_t))
//
// x_{t+1}  = Π_M( x̃_{t+1} )
//
// Interpretation:
//   - Dynamics are NOT optimized
//   - Dynamics are GENERATED freely (x̃)
//   - Validity is enforced via projection Π_M
//
// Core shift:
//   FROM: penalized evolution
//   TO:   feasibility-constrained evolution
//
// Properties:
//   • excitation is preserved (γ term)
//   • stability is enforced geometrically (Π_M)
//   • no scalar "objective pressure" exists in dynamics
//
// ============================================================================
// KEY PRINCIPLE
// ============================================================================
//
// Stability is NOT achieved by minimizing energy.
//
// Stability is achieved by:
//
//   restricting evolution to a feasible trajectory manifold
//   while preserving bounded excitation degrees of freedom
//
// ============================================================================
//
// ============================================================================
//
// FUNDAMENTAL MATHEMATICAL ENDPOINT
// ----------------------------------------------------------------------------
// The DVSM core is now expressed as:
//
//     x_{t+1} = Π_M( x_t + η(σ_t - x_t) + γ(σ_t - P(x_t)) )
//
// WHERE:
//
//   x_t        : system state
//   σ_t        : external input signal
//   P(x_t)     : expected input manifold (state-conditioned prior)
//   η          : contraction coefficient (stability)
//   γ          : excitation preservation coefficient
//   Π_M        : feasibility projection operator (constraints + bounds)
//
// ============================================================================
//
// CORE DESIGN PRINCIPLE
// ----------------------------------------------------------------------------
// Stability is NOT achieved by minimizing energy.
//
// Stability is achieved by:
//
//     restricting evolution to a feasible trajectory manifold
//     while preserving bounded excitation degrees of freedom
//
// ============================================================================
//
// GOODHART RESISTANCE STATEMENT
// ----------------------------------------------------------------------------
// Optimization targets are explicitly separated from observables:
//
//     observables → do NOT define control objective
//     control     → operates on constrained geometry
//
// This prevents:
//
//     metric = objective collapse
//
// by ensuring:
//
//     metrics remain epiphenomenal projections
//
// ============================================================================
//
// SYSTEM CLASSIFICATION
// ----------------------------------------------------------------------------
// DVSM is a:
//
//   - discrete-time constrained dynamical system
//   - control-affine update process
//   - projection-stabilized contraction map
//   - trajectory-feasible evolution engine
//
// NOT:
//
//   - reward optimizer
//   - penalty minimization system
//   - scalar objective maximizer
//
// ============================================================================
//
// ENDPOINT INTENT
// ----------------------------------------------------------------------------
// This represents the minimal stable form of DVSM:
//
//   contraction (η)
// + excitation (γ)
// + feasibility projection (Π_M)
//
// Any additional complexity (Δ, H, j, etc.)
// must reduce to or derive from this structure.
//
// ============================================================================

// ============================================================================
// DVSM-π — PROJECTED JET-CONSTRAINED MULTI-NODE CONTROL SYSTEM (Definition 1)
// ============================================================================

use std::collections::HashMap;

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
// SYSTEM PARAMETERS
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub eta: f64,
    pub gamma: f64,
    pub coupling: f64,
}

// ============================================================================
// BOUNDS (JET SPACE CONSTRAINTS)
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
// KERNEL DYNAMICS
// ============================================================================

#[inline(always)]
pub fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// EXCITATION MODEL
// ============================================================================

#[inline(always)]
pub fn excitation(sigma: f64, p: f64) -> f64 {
    sigma - p
}

// ============================================================================
// DISCRETE JET
// ============================================================================

pub fn jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;
    let a = v - v_prev;
    let j = a - (v_prev);

    Jet { v, a, j }
}

// ============================================================================
// HARD CONSTRAINT SET (M)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Constraint {
    pub bounds: Bounds,
}

impl Constraint {
    pub fn project_x(&self, x: f64) -> f64 {
        x.clamp(self.bounds.x_min, self.bounds.x_max)
    }

    pub fn in_bounds(&self, j: &Jet) -> bool {
        j.v.abs() <= self.bounds.v_max
            && j.a.abs() <= self.bounds.a_max
            && j.j.abs() <= self.bounds.j_max
    }
}

// ============================================================================
// “CLOSEST FEASIBLE JET PROJECTION” (SOFT PROJECTION)
// ============================================================================
//
// Instead of hard clamp → we do bounded correction toward feasibility.
// This approximates projection onto convex box in jet-space.
// ============================================================================

#[inline(always)]
fn project_jet(j: Jet, b: &Bounds) -> Jet {
    Jet {
        v: j.v.clamp(-b.v_max, b.v_max),
        a: j.a.clamp(-b.a_max, b.a_max),
        j: j.j.clamp(-b.j_max, b.j_max),
    }
}

// ============================================================================
// LYAPUNOV ENERGY FUNCTION
// ============================================================================
//
// E = quadratic jet energy + state energy
// ============================================================================

fn lyapunov_energy(x: f64, j: &Jet, b: &Bounds) -> f64 {
    let state_energy = x * x;
    let jet_energy =
        (j.v / b.v_max).powi(2)
        + (j.a / b.a_max).powi(2)
        + (j.j / b.j_max).powi(2);

    state_energy + jet_energy
}

// ============================================================================
// LYAPUNOV CERTIFICATE CHECK
// ============================================================================

fn lyapunov_valid(e_prev: f64, e_next: f64) -> bool {
    e_next <= e_prev + 1e-6
}

// ============================================================================
// SINGLE-NODE DVSM-π STEP (CONSTRAINED FEASIBILITY FORM)
// ============================================================================

pub fn dvsm_pi_step(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    p: Params,
    b: Bounds,
) -> (f64, Jet) {

    // ------------------------------------------------------------
    // 1. CAUSAL KERNEL EVOLUTION (F_A)
    // ------------------------------------------------------------
    let k = kernel(x0, sigma, p.eta);

    // ------------------------------------------------------------
    // 2. EXCITATION (NO MANIFOLD BIAS)
    // ------------------------------------------------------------
    let u = p.gamma * (sigma - x0);

    let x_raw = k + u;

    // ------------------------------------------------------------
    // 3. STATE SPACE PROJECTION (Π_M)
    // ------------------------------------------------------------
    let x_proj = x_raw.clamp(b.x_min, b.x_max);

    // ------------------------------------------------------------
    // 4. JET COMPUTATION (OBSERVATION ONLY)
    // ------------------------------------------------------------
   jet(x2, x1, x0)

    // OR:

    J_raw   = jet(x2, x1, x0)
    J_proj  = jet(x2', x1', x0')

    // ------------------------------------------------------------
    // 5. JET FEASIBILITY PROJECTION (Π_M IN JET SPACE)
    // ------------------------------------------------------------
    let j_proj = Jet {
        v: j_raw.v.clamp(-b.v_max, b.v_max),
        a: j_raw.a.clamp(-b.a_max, b.a_max),
        j: j_raw.j.clamp(-b.j_max, b.j_max),
    };

    // ------------------------------------------------------------
    // 6. RETURN (NO SCALAR OBJECTIVE)
    // ------------------------------------------------------------
    (x_proj, j_proj)
}

// ============================================================================
// MULTI-NODE GRAPH DVSM-π
// ============================================================================

pub struct DVSMGraph {
    pub states: Vec<f64>,
    pub history: Vec<Vec<f64>>,
    pub graph: Graph,
    pub params: Params,
    pub bounds: Bounds,
}

impl DVSMGraph {
    pub fn step(&mut self, sigma: f64) -> Vec<f64> {

        let prev2 = self.history
            .get(self.history.len().saturating_sub(3))
            .cloned()
            .unwrap_or(self.states.clone());

        let prev1 = self.states.clone();

        let mut next = self.states.clone();

        // --------------------------------------------------------
        // NODE UPDATE (COUPLED DYNAMICS)
        // --------------------------------------------------------
        for i in 0..self.states.len() {

            let mut coupling = 0.0;

            for &(a, b) in &self.graph.edges {
                if a == i {
                    coupling += self.params.coupling * (self.states[b] - self.states[a]);
                }
            }

            let (x, _, _) = dvsm_pi_step(
                prev2[i],
                prev1[i],
                self.states[i],
                sigma + coupling,
                self.params,
                self.bounds,
            );

            next[i] = x;
        }

        self.history.push(self.states.clone());
        self.states = next.clone();

        next
    }
}

// ============================================================================
// ADVERSARIAL σ̃(t) STRESS HARNESS
// ============================================================================

pub struct Adversary {
    pub strength: f64,
}

impl Adversary {

    pub fn perturb(&self, sigma: f64, t: usize) -> f64 {

        // bounded worst-case directional oscillation
        let spike = (t as f64).sin() * self.strength;
        sigma + spike
    }

    pub fn worst_case_bounds(&self, sigma: f64) -> (f64, f64) {
        (sigma - self.strength, sigma + self.strength)
    }
}

// ============================================================================
// STRESS TEST RUNNER
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

        // simple invariant check
        debug_assert!(next.iter().all(|x| x.is_finite()));
    }
}
// ============================================================================
// DVSM-π — PROJECTED JET-CONSTRAINED DYNAMICAL SYSTEM (CORE ENGINE Model)
// ============================================================================

use std::f64;

// ============================================================================
// STATE + JET SPACE
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

// ============================================================================
// SYSTEM PARAMETERS
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub eta: f64,     // kernel contraction
    pub gamma: f64,   // excitation strength
}

// ============================================================================
// BOUNDS (MANIFOLD DEFINITION M)
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
// KERNEL (CAUSAL CORE)
// ============================================================================

#[inline(always)]
pub fn kernel_fa(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// EXPECTATION MODEL P(x)
// (must remain NON-optimizing, observational only)
// ============================================================================

#[inline(always)]
pub fn p_expectation(_x: f64) -> f64 {
    1.0
}

// ============================================================================
// EXCITATION TERM
// ============================================================================

#[inline(always)]
pub fn excitation(sigma: f64, px: f64) -> f64 {
    sigma - px
}

// ============================================================================
// JET COMPUTATION (DISCRETE 3RD ORDER)
// ============================================================================

pub fn compute_jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;

    let a = v - v_prev;
    let j = a - (v_prev);

    Jet { v, a, j }
}

// ============================================================================
// PROJECTION: STATE SPACE
// ============================================================================

#[inline(always)]
fn project_state(x: f64, b: &Bounds) -> f64 {
    x.clamp(b.x_min, b.x_max)
}

// ============================================================================
// PROJECTION: JET SPACE (HARD CONSTRAINT ENFORCEMENT)
// ============================================================================

#[inline(always)]
fn project_jet(j: Jet, b: &Bounds) -> Jet {
    Jet {
        v: j.v.clamp(-b.v_max, b.v_max),
        a: j.a.clamp(-b.a_max, b.a_max),
        j: j.j.clamp(-b.j_max, b.j_max),
    }
}

// ============================================================================
// LYAPUNOV ENERGY (STABILITY CERTIFICATE)
// ============================================================================

#[inline(always)]
fn feasibility_check(x: f64, j: &Jet, b: &Bounds) -> bool {
    x >= b.x_min
        && x <= b.x_max
        && j.v.abs() <= b.v_max
        && j.a.abs() <= b.a_max
        && j.j.abs() <= b.j_max
}

// ============================================================================
// DVSM-π STEP (SINGLE NODE)
// ============================================================================

pub fn dvsm_pi_step(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    params: Params,
    bounds: Bounds,
) -> (f64, Jet, f64) {

    // ------------------------------------------------------------
    // 1. KERNEL EVOLUTION
    // ------------------------------------------------------------
    let kernel = kernel_fa(x0, sigma, params.eta);

    // ------------------------------------------------------------
    // 2. BOUNDED EXCITATION
    // ------------------------------------------------------------
    let px = p_expectation(x0);
    let u = params.gamma * excitation(sigma, px);

    let raw = kernel + u;

    // ------------------------------------------------------------
    // 3. STATE PROJECTION
    // ------------------------------------------------------------
    let x_next = project_state(raw, &bounds);

    // ------------------------------------------------------------
    // 4. JET COMPUTATION
    // ------------------------------------------------------------
    let jet = compute_jet(x2, x1, x_next);

    // ------------------------------------------------------------
    // 5. JET PROJECTION
    // ------------------------------------------------------------
    let jet_proj = project_jet(jet, &bounds);

    // ------------------------------------------------------------
    // 6. LYAPUNOV CERTIFICATE
    // ------------------------------------------------------------
    let e = lyapunov_energy(x_next, &jet_proj, &bounds);

    (x_next, jet_proj, e)
}

// ============================================================================
// MULTI-NODE GRAPH SYSTEM
// ============================================================================

#[derive(Clone)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

pub struct DVSMGraph {
    pub states: Vec<f64>,
    pub history: Vec<Vec<f64>>,
    pub graph: Graph,
    pub params: Params,
    pub bounds: Bounds,
}

// ============================================================================
// GRAPH STEP (COUPLED DYNAMICS)
// ============================================================================

impl DVSMGraph {
    pub fn step(&mut self, sigma: f64) -> Vec<f64> {

        let prev2 = self.history
            .last()
            .cloned()
            .unwrap_or(self.states.clone());

        let prev1 = self.states.clone();

        let mut next = self.states.clone();

        for i in 0..self.states.len() {

            let mut coupling = 0.0;

            for &(a, b) in &self.graph.edges {
                if a == i {
                    coupling += self.states[b] - self.states[a];
                }
            }

            let (x, _, _) = dvsm_pi_step(
                prev2[i],
                prev1[i],
                self.states[i],
                sigma + coupling,
                self.params,
                self.bounds,
            );

            next[i] = x;
        }

        self.history.push(self.states.clone());
        self.states = next.clone();

        next
    }
}

// ============================================================================
// ADVERSARIAL INPUT MODEL σ̃(t)
// ============================================================================

pub struct Adversary {
    pub strength: f64,
}

impl Adversary {
    pub fn perturb(&self, sigma: f64, t: usize) -> f64 {
        let oscillation = (t as f64).sin() * self.strength;
        sigma + oscillation
    }
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
        let _ = system.step(sigma);

        debug_assert!(
            system.states.iter().all(|x| x.is_finite()),
            "DVSM-π: numerical instability detected"
        );
    }
}
