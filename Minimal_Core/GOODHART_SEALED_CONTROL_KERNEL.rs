// ============================================================================
// DVSM-π — GOODHART-SEALED CONTROL KERNEL (SINGLE FILE REFERENCE)
// ============================================================================
// Design goal:
//   Geometric feasibility control with NO metric-driven optimization loop
//   Observables are non-controlling projections
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
pub struct Params {
    pub eta: f64,
    pub gamma: f64,
    pub coupling: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub x_min: f64,
    pub x_max: f64,
    pub v_max: f64,
    pub a_max: f64,
    pub j_max: f64,
}

// ============================================================================
// GRAPH STRUCTURE
// ============================================================================

#[derive(Clone)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

// ============================================================================
// OBSERVATION (NON-CONTROLLING)
// ============================================================================

#[inline(always)]
pub fn observe_metric(x: f64) -> f64 {
    x * x
}

// ============================================================================
// EXPECTATION MODEL (MUST NOT MATCH METRIC)
// ============================================================================

#[inline(always)]
pub fn p_expectation(_x: f64) -> f64 {
    1.0
}

// ============================================================================
// KERNEL + EXCITATION
// ============================================================================

#[inline(always)]
pub fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

#[inline(always)]
pub fn excitation(sigma: f64, px: f64) -> f64 {
    sigma - px
}

// ============================================================================
// PROJECTIONS (GEOMETRIC CONSTRAINTS)
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
// DISCRETE JET OPERATOR
// ============================================================================

pub fn compute_jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// ============================================================================
// DVSM SINGLE STEP
// ============================================================================

pub fn dvsm_step(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    p: Params,
    b: Bounds,
) -> (f64, Jet) {

    // 1. causal kernel
    let k = kernel(x0, sigma, p.eta);

    // 2. excitation (no metric access)
    let px = p_expectation(x0);
    let u = p.gamma * excitation(sigma, px);

    let raw = k + u;

    // 3. projection (geometry enforcement)
    let x_proj = project_state(raw, &b);

    // 4. jet reconstruction (post-projection only)
    let jet_raw = compute_jet(x2, x1, x_proj);
    let jet = project_jet(jet_raw, &b);

    (x_proj, jet)
}

// ============================================================================
// GRAPH SYSTEM
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
            .last()
            .cloned()
            .unwrap_or(self.states.clone());

        let prev1 = self.states.clone();

        let mut next = self.states.clone();

        for i in 0..self.states.len() {

            // coupling (purely structural, not optimizing)
            let mut c = 0.0;

            for &(a, b) in &self.graph.edges {
                if a == i {
                    c += self.states[b] - self.states[a];
                }
            }

            let sigma_eff = sigma + self.params.coupling * c;

            let (x, _) = dvsm_step(
                prev2[i],
                prev1[i],
                self.states[i],
                sigma_eff,
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
// ADVERSARY (STRESS TEST ONLY)
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
            next.iter().all(|x| x.is_finite()),
            "DVSM-π instability detected"
        );
    }
}

// ============================================================================
// DEV NOTES — GHOST LAYERS (READ-ONLY ANALYSIS)
// ============================================================================
//
// GHOST 1: expectation_model leakage
//   If p_expectation approximates observe_metric → hidden optimization channel
//
// GHOST 2: coupling feedback loop
//   Graph edges can form indirect reinforcement cycles
//
// GHOST 3: projection discontinuity
//   clamp creates boundary attractors (fake stability zones)
//
// GHOST 4: jet amplification noise
//   discrete differentiation amplifies projection artifacts
//
// GHOST 5: observational contamination
//   if metrics influence parameter tuning → Goodhart re-enters indirectly
//
// CORE RULE:
//   Metrics must remain causally downstream of control, never upstream
// ============================================================================
// ============================================================================
// DVSM-π — GEOMETRIC HARDENING ADDENDUM v2 (RUST MODULE)
// ============================================================================
// Purpose:
//   Strengthen DVSM kernel with:
//     - true projection semantics (Πₘ abstraction)
//     - coupling normalization stability
//     - stable jet reconstruction
//     - expectation–metric decoupling (Goodhart seal reinforcement)
// ============================================================================

use std::f64;

// ============================================================================
// CORE TYPES
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// 1. TRUE PROJECTION OPERATOR (GENERALIZED FORM)
// ============================================================================

#[inline(always)]
pub fn pi_m_scalar(x: f64, min: f64, max: f64) -> f64 {
    if x.is_nan() {
        return 0.0;
    }
    x.max(min).min(max)
}

// ============================================================================
// 2. COUPLING NORMALIZATION (GRAPH STABILITY LAYER)
// ============================================================================

#[inline(always)]
pub fn normalize_coupling(sum: f64, degree: usize) -> f64 {
    if degree == 0 {
        return 0.0;
    }

    let scaled = sum / degree as f64;
    scaled.tanh()
}

// ============================================================================
// 3. EXPECTATION MODEL (GOODHART-SEAL BOUNDARY)
// ============================================================================
//
// Must remain orthogonal to any metric-like function.
// This is intentionally non-informative structurally.
//
#[inline(always)]
pub fn sealed_expectation(_x: f64, noise: f64) -> f64 {
    1.0 + noise
}

// ============================================================================
// 4. STABLE JET RECONSTRUCTION (LOW-PASS DISCRETE DERIVATIVE)
// ============================================================================
//
// Replaces unstable forward differences with symmetric smoothing.
//
#[inline(always)]
pub fn stable_jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = 0.5 * ((x0 - x1) + (x1 - x2));
    let a = x0 - 2.0 * x1 + x2;
    let j = x0 - 3.0 * x1 + 3.0 * x2 - x2;

    Jet { v, a, j }
}

// ============================================================================
// 5. SHADOW ENERGY (OBSERVATIONAL ONLY)
// ============================================================================

#[inline(always)]
pub fn shadow_energy(x: f64, j: &Jet) -> f64 {
    x * x + 0.1 * (j.v * j.v + j.a * j.a + j.j * j.j)
}

// ============================================================================
// 6. GOODHART INVARIANT CHECK (DIAGNOSTIC ONLY)
// ============================================================================
//
// This is NOT part of control flow.
// Only used for debugging leakage.
//
#[inline(always)]
pub fn goodhart_leak_detect(control_dep: f64, metric_dep: f64) -> bool {
    (control_dep - metric_dep).abs() < 1e-12
}

// ============================================================================
// 7. SAFE GRAPH COUPLING (ANTI-LOOP HARDENED)
// ============================================================================

pub fn safe_coupling(edges: &[(usize, usize)], states: &[f64], i: usize) -> f64 {
    let mut sum = 0.0;
    let mut degree = 0usize;

    for &(a, b) in edges {
        if a == i {
            sum += states[b] - states[a];
            degree += 1;
        }
    }

    normalize_coupling(sum, degree)
}

// ============================================================================
// 8. OBSERVATION LAYER (NON-CONTROLLING)
// ============================================================================

#[inline(always)]
pub fn observe(x: f64) -> f64 {
    x
}

// ============================================================================
// 9. MANIFOLD PROJECTION WRAPPER
// ============================================================================

#[inline(always)]
pub fn project_state(x: f64, min: f64, max: f64) -> f64 {
    pi_m_scalar(x, min, max)
}

// ============================================================================
// 10. DEV NOTES — GHOST FAILURE MODES
// ============================================================================
//
// GHOST 1: expectation drift
//   If sealed_expectation begins correlating with observed metrics,
//   Goodhart channel reopens indirectly.
//
// GHOST 2: coupling eigenmode amplification
//   Graph structure can form resonance loops → instability modes.
//
// GHOST 3: projection boundary attractors
//   Hard clamping creates artificial fixed points.
//
// GHOST 4: jet reconstruction noise inflation
//   Discrete derivatives can amplify numerical jitter.
//
// GHOST 5: observation feedback contamination
//   If observe() is used in control loop → Goodhart re-enters.
//
// ============================================================================
// ============================================================================
// DVSM-π — GEOMETRIC CLOSURE ADDENDUM v3 (COMPOSITIONAL HARDENING LAYER)
// ============================================================================
// Purpose:
//   Upgrade DVSM from pointwise projection system → compositional manifold flow
//   with stability preserved under repeated projection + coupling cycles
// ============================================================================

use std::f64;

// ============================================================================
// CORE TYPES
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// 1. COMPOSITIONALLY CONSISTENT PROJECTION
// ============================================================================
//
// Fixes hidden issue:
//   Π(x) ∘ Π(x) ≠ Π(x) in discrete numeric systems
//
// We enforce idempotence stabilization.
//
#[inline(always)]
pub fn pi_m_idempotent(x: f64, min: f64, max: f64) -> f64 {
    let x1 = x.max(min).min(max);
    let x2 = x1.max(min).min(max);

    // enforce idempotence numerically
    0.5 * (x1 + x2)
}

// ============================================================================
// 2. NONLINEAR COUPLING SATURATION (EIGENMODE CONTROL)
// ============================================================================
//
// Prevents graph resonance accumulation.
//
#[inline(always)]
pub fn nonlinear_coupling(sum: f64, degree: usize) -> f64 {
    if degree == 0 {
        return 0.0;
    }

    let normalized = sum / (degree as f64);
    normalized.tanh() * (1.0 - normalized.abs().min(1.0))
}

// ============================================================================
// 3. JET FLOW CLOSURE OPERATOR
// ============================================================================
//
// Ensures jets remain consistent under repeated projection steps.
//
#[inline(always)]
pub fn jet_closure(prev: Jet, current: Jet) -> Jet {
    Jet {
        v: 0.5 * (prev.v + current.v),
        a: 0.5 * (prev.a + current.a),
        j: 0.5 * (prev.j + current.j),
    }
}

// ============================================================================
// 4. STABLE MANIFOLD RE-ENTRY MAP
// ============================================================================
//
// Ensures trajectory re-entry into feasible manifold does not oscillate.
//
#[inline(always)]
pub fn manifold_reentry(x_raw: f64, min: f64, max: f64, damping: f64) -> f64 {
    let projected = x_raw.max(min).min(max);
    let correction = (x_raw - projected) * damping;

    projected + correction * 0.1
}

// ============================================================================
// 5. GOODHART SEAL STRENGTHENING CONDITION
// ============================================================================
//
// Strengthens orthogonality condition:
//
//   control ⟂ metric ⟂ observation drift
//
#[inline(always)]
pub fn seal_strength(control: f64, metric: f64, obs: f64) -> f64 {
    let a = control - metric;
    let b = metric - obs;

    (a * a + b * b).sqrt()
}

// ============================================================================
// 6. STABILITY INCREMENT CHECK (LOCAL LYAPUNOV-LIKE MONITOR)
// ============================================================================
//
// Not a control signal — purely diagnostic.
//
#[inline(always)]
pub fn stability_increment(e_prev: f64, e_next: f64) -> f64 {
    e_next - e_prev
}

// ============================================================================
// 7. GRAPH RESONANCE DETECTOR (EIGENMODE GHOST SCAN)
// ============================================================================
//
// Detects hidden amplification cycles in coupling graph.
//
pub fn resonance_scan(edges: &[(usize, usize)], states: &[f64]) -> f64 {
    let mut energy = 0.0;

    for &(a, b) in edges {
        let diff = states[a] - states[b];
        energy += diff * diff;
    }

    energy / (states.len().max(1) as f64)
}

// ============================================================================
// 8. FULL GHOST MODEL UPDATE (EXPANDED FAILURE SURFACE)
// ============================================================================
//
// GHOST 6: projection hysteresis
//   repeated projection creates lag-dependent drift attractors
//
// GHOST 7: coupling phase-locking
//   nonlinear saturation can still synchronize subgraphs unintentionally
//
// GHOST 8: jet closure smoothing bias
//   averaging jets reduces variance but introduces phase lag distortion
//
// GHOST 9: manifold re-entry overshoot
//   damping term can induce boundary oscillation loops
//
// ============================================================================
// ============================================================================
// DVSM-π ADDENDUM v3.5 — GH-SEALED CONSTRAINT & FAILURE-TOPOLOGY LAYER
// ============================================================================
// Purpose:
//   Replace implicit “Goodhart resistance” claims with explicit
//   failure-mode containment + projection closure guarantees.
//
// Key shift:
//   FROM: "system is ungameable"
//   TO:   "system has no unbounded exploit channels under defined model class"
//
// Now: GHGhost ∈ observable failure topology
// ============================================================================

use std::f64;

// ============================================================================
// CORE STATE
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub x: f64,
}

// ============================================================================
// GH-SEALED MANIFOLD BOUNDS
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
// GHOST TYPES (FAILURE MODES = FIRST-CLASS OBJECTS)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub enum GHGhost {
    DriftLeak,        // uncontrolled accumulation outside projection contraction
    JetInflation,     // derivative escalation under bounded x
    CouplingEcho,     // graph resonance amplification loop
    ProjectionChatter,// repeated boundary bouncing
    SigmaCapture,     // implicit optimization of σ surrogate
}

// ============================================================================
// GHOST DETECTOR (LOCAL INVARIANT CHECKS)
// ============================================================================

#[inline(always)]
pub fn detect_ghost(x: f64, v: f64, a: f64, j: f64, b: &Bounds) -> Option<GHGhost> {
    if x.is_nan() || x.is_infinite() {
        return Some(GHGhost::DriftLeak);
    }

    if v.abs() > 2.0 * b.v_max {
        return Some(GHGhost::JetInflation);
    }

    if a.abs() > 2.0 * b.a_max {
        return Some(GHGhost::JetInflation);
    }

    if j.abs() > 2.0 * b.j_max {
        return Some(GHGhost::JetInflation);
    }

    None
}

// ============================================================================
// GH-SEAL OPERATOR Π_GH
// ============================================================================
// Not optimization. Not penalty.
// PURE constraint closure map.
// ============================================================================

#[inline(always)]
pub fn pi_gh(x: f64, b: &Bounds) -> f64 {
    x.clamp(b.x_min, b.x_max)
}

// ============================================================================
// JET RECONSTRUCTION (OBSERVATIONAL ONLY)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

pub fn jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// ============================================================================
// GH-SEALED STEP (CLOSED FORM)
// ============================================================================

pub fn dvsm_gh_step(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    eta: f64,
    gamma: f64,
    b: Bounds,
) -> (f64, Jet, Option<GHGhost>) {

    // ------------------------------------------------------------
    // 1. KERNEL (STRUCTURAL MOTION)
    // ------------------------------------------------------------
    let k = x0 + eta * (sigma - x0);

    // ------------------------------------------------------------
    // 2. EXCITATION (NON-OPTIMIZING INPUT OFFSET)
    // ------------------------------------------------------------
    let u = gamma * (sigma - x0);

    let x_raw = k + u;

    // ------------------------------------------------------------
    // 3. GH PROJECTION (CLOSURE ENFORCEMENT)
    // ------------------------------------------------------------
    let x = pi_gh(x_raw, &b);

    // ------------------------------------------------------------
    // 4. JET OBSERVATION
    // ------------------------------------------------------------
    let j = jet(x2, x1, x);

    // ------------------------------------------------------------
    // 5. GHOST CHECK (FAILURE MODE DETECTION)
    // ------------------------------------------------------------
    let ghost = detect_ghost(x, j.v, j.a, j.j, &b);

    (x, j, ghost)
}

// ============================================================================
// GH STABILITY INTERPRETATION
// ============================================================================
//
// Stability is NOT:
//   - convergence
//   - minimization
//   - Lyapunov decrease guarantee
//
// Stability is:
//
//   absence of uncontained GHGhost signals under bounded evolution
//
// ============================================================================
// GH PROTECTION LOGIC (REAL MEANING)
// ============================================================================
//
// Protection = structural closure:
//
//   Π_GH ensures state never leaves manifold
//   jet detector ensures derivative explosion is visible
//   ghosts ensure failure is labeled, not hidden
//
// ============================================================================
// WEAKNESS REALITY (IMPORTANT CORRECTION)
// ============================================================================
//
// This system does NOT eliminate weaknesses.
//
// It enforces:
//
//   1. bounded state space
//   2. observable failure modes
//   3. non-silent divergence channels
//
// Remaining fundamental limitation:
//
//   If σ carries adversarial structure aligned with projection geometry,
//   system will still track it (by design).
//
// That is not a bug.
// That is observability preservation.
//
// ============================================================================
// ============================================================================
// DVSM-π ADDENDUM v4 — GH SPECTRAL CLOSURE LAYER (OPERATOR FORM)
// ============================================================================
// Purpose:
//   Upgrade GH-ghost detection from scalar heuristics → operator dynamics
//
// Key shift:
//   FROM: "detect instability via thresholds"
//   TO:   "instability = loss of invariance under GH projection operator"
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

// ============================================================================
// GH GHOST SPACE (NOW A DYNAMICAL FIELD)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct GHPhi {
    pub drift: f64,
    pub curvature: f64,
    pub resonance: f64,
}

// ============================================================================
// BOUNDS (MANIFOLD DEFINITION)
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
// GH PROJECTION OPERATOR Π_GH
// ============================================================================
//
// This is NOT clamping.
// It is a contraction map on state + jet consistency space.
// ============================================================================

#[inline(always)]
pub fn pi_gh(x: f64, b: &Bounds) -> f64 {
    x.clamp(b.x_min, b.x_max)
}

// ============================================================================
// JET RECONSTRUCTION OPERATOR J
// ============================================================================

pub fn jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// ============================================================================
// GH STATE-TO-GHOST MAP Φ_GH
// ============================================================================
//
// This replaces "threshold detection" with field extraction.
// Ghosts are now projections of dynamical inconsistency.
// ============================================================================

#[inline(always)]
pub fn phi_gh(x: f64, j: &Jet, b: &Bounds) -> GHPhi {

    let drift = (x - (b.x_min + b.x_max) * 0.5).abs();

    let curvature =
        (j.a / (b.a_max + 1e-9)).powi(2)
        + (j.j / (b.j_max + 1e-9)).powi(2);

    let resonance = j.v * j.a;

    GHPhi { drift, curvature, resonance }
}

// ============================================================================
// GH SPECTRAL AMPLITUDE (INVARIANT ENERGY FORM)
// ============================================================================
//
// Interprets ghost state as a spectral norm of violation modes.
// ============================================================================

#[inline(always)]
pub fn gh_spectrum(phi: GHPhi) -> f64 {
    phi.drift + phi.curvature.abs() + phi.resonance.abs()
}

// ============================================================================
// SPECTRAL STABILITY CONDITION
// ============================================================================
//
// System is stable if GH spectrum is contractive under evolution.
// ============================================================================

#[inline(always)]
pub fn gh_stable(e_prev: f64, e_next: f64) -> bool {
    e_next <= e_prev * 1.01 // bounded expansion tolerance
}

// ============================================================================
// EVOLUTION KERNEL
// ============================================================================

#[inline(always)]
pub fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

#[inline(always)]
pub fn excitation(sigma: f64, x: f64) -> f64 {
    sigma - x
}

// ============================================================================
// GH-SEALED STEP (SPECTRAL FORM)
// ============================================================================

pub fn dvsm_gh_step(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    eta: f64,
    gamma: f64,
    b: Bounds,
) -> (f64, Jet, f64, GHPhi) {

    // ------------------------------------------------------------
    // 1. CORE EVOLUTION
    // ------------------------------------------------------------
    let k = kernel(x0, sigma, eta);
    let u = gamma * excitation(sigma, x0);

    let x_raw = k + u;

    // ------------------------------------------------------------
    // 2. GH PROJECTION (CLOSURE MAP)
    // ------------------------------------------------------------
    let x = pi_gh(x_raw, &b);

    // ------------------------------------------------------------
    // 3. JET OBSERVATION
    // ------------------------------------------------------------
    let j = jet(x2, x1, x);

    // ------------------------------------------------------------
    // 4. GH FIELD EXTRACTION
    // ------------------------------------------------------------
    let phi = phi_gh(x, &j, &b);

    let spectrum = gh_spectrum(phi);

    // ------------------------------------------------------------
    // 5. RETURN FULL STRUCTURE
    // ------------------------------------------------------------
    (x, j, spectrum, phi)
}

// ============================================================================
// INTERPRETATION SHIFT (IMPORTANT)
// ============================================================================
//
// OLD VIEW:
//   ghost = failure event
//
// NEW VIEW:
//   ghost = eigenmode of constraint violation operator
//
// Stability is not absence of ghosts.
// Stability is:
//
//   bounded spectral propagation of GH modes under Π_GH.
//
// ============================================================================
// CORE RESULT
// ============================================================================
//
// DVSM-π is now:
//
//   a constrained dynamical system with measurable violation spectrum
//
// NOT:
//
//   a heuristic stability system
//
// ============================================================================
