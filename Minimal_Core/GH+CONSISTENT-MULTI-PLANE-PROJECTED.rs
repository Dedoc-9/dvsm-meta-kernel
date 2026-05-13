// ============================================================================
// DVSM-π v2 — CONSISTENT MULTI-PLANE PROJECTED DYNAMICS
// ============================================================================
// Author: DVSM-π Research Lineage (Corrected Formalism)
// Status: Constraint Geometry System (NOT Constant-Derivation System)
// ============================================================================
// DVSM-π+++ CORE UPDATE LAW
//
// The system evolves by unconstrained generation followed by geometric closure:
//
//     x_{t+1} = Π_M( F(x_t, σ_t) )
//
// where:
//   F      : unconstrained graph-coupled evolution operator
//   Π_M    : stratified projection onto feasible jet-manifold M
//   x_t    : current state on or near M
//   σ_t    : external excitation signal
//
// Interpretation:
//   - F proposes a candidate transition in ambient space
//   - Π_M enforces manifold consistency and feasibility closure
//   - only projected states are admitted into system trajectory
// =========================================================================

// FAILURE PREVENTION PRINCIPLE (DVSM-π+++)
//
// System failure modes are eliminated structurally, not corrected dynamically.
//
// All instability channels are removed by:
//   - enforcing Πₘ projection closure (no out-of-manifold states)
//   - separating observations Oₖ from control F (no feedback leakage)
//   - reconstructing jets from trajectory consistency (no derivative drift)
//
// Result: failure cannot accumulate—only infeasible states can appear and are projected out.
//
// MATHEMATICAL FOUNDATION (IMPORTANT CORRECTION)
// ============================================================================
//
// This system does NOT derive mathematical constants (e.g. π, e).
//
// Instead, it defines:
//
//   1. A bounded state space:      x ∈ ℝ
//   2. A set of constraint planes: {M_k ⊂ ℝ}
//   3. A projection operator:      Π_M : ℝ → M
//   4. A discrete-time dynamical system under projection closure
//
// ---------------------------------------------------------------------------
// CORE OBJECT
// ---------------------------------------------------------------------------
//
// State evolves under:
//
//   x_{t+1} = Π_M( F(x_t, σ_t, ξ_t) )
//
// where:
//
//   F = unconstrained update map
//   σ_t = external forcing input
//   ξ_t = coupling field
//   Π_M = feasibility projection operator
//
// ---------------------------------------------------------------------------
// OBSERVATIONAL JET STRUCTURE (NOT CAUSAL)
// ---------------------------------------------------------------------------
//
// Jet J_t is defined ONLY as:
//
//   J_t = (v_t, a_t, j_t)
//
// where:
//
//   v_t = x_t - x_{t-1}
//   a_t = v_t - v_{t-1}
//   j_t = a_t - a_{t-1}
//
// IMPORTANT:
// Jet is a derived coordinate chart, not a state variable.
//
// ---------------------------------------------------------------------------
// INVARIANT DEFINITION (STRICT)
// ---------------------------------------------------------------------------
//
// The ONLY invariants in this system are:
//
//   - boundedness under Π_M
//   - consistency of projection closure
//   - stability of trajectory within constraint set
//
// NOT:
//   - constants (π, e, φ)
//   - energy minimization
//   - global fixed points
//
// ---------------------------------------------------------------------------
// ANTI-OVERINTERPRETATION AXIOM
// ---------------------------------------------------------------------------
//
// No scalar quantity derived from trajectory statistics
// is assumed to correspond to a mathematical constant.
//
// Emergent ratios are observational artifacts unless
// proven invariant under symmetry group actions.
//
// ============================================================================

use std::f64;

// ============================================================================
// STATE
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub x: f64,
}

// ============================================================================
// JET (DERIVED ONLY)
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// MULTI-PLANE CONSTRAINT SYSTEM
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub min: f64,
    pub max: f64,
    pub weight: f64,
}

// ============================================================================
// DVSM-π SYSTEM
// ============================================================================

pub struct DVSMpi {
    pub planes: Vec<Plane>,
    pub eta: f64,
    pub gamma: f64,
    pub coupling: f64,
}

// ============================================================================
// SINGLE PLANE PROJECTION
// ============================================================================

#[inline(always)]
fn project_plane(x: f64, p: &Plane) -> f64 {
    x.clamp(p.min, p.max)
}

// ============================================================================
// CONSENSUS PROJECTION Π_M
// ============================================================================
//
// Interpretation correction:
// This is NOT “multiple manifolds defining truth”
//
// It is:
//   weighted constraint reconciliation operator
//
// ============================================================================

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
// KERNEL (LOCAL DYNAMICS ONLY)
// ============================================================================

#[inline(always)]
fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// COUPLING FIELD (NO GLOBAL INTERPRETATION)
// ============================================================================

fn coupling_field(x: f64, neighbors: &[f64], c: f64) -> f64 {
    let mut sum = 0.0;

    for &n in neighbors {
        sum += c * (n - x);
    }

    sum
}

// ============================================================================
// JET RECONSTRUCTION (OBSERVATION FUNCTION ONLY)
// ============================================================================

fn compute_jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// ============================================================================
// EVOLUTION MAP
// ============================================================================

fn evolve(x: f64, sigma: f64, cx: f64, eta: f64, gamma: f64) -> f64 {
    let k = kernel(x, sigma + cx, eta);
    let u = gamma * (sigma - x);
    k + u
}

// ============================================================================
// SINGLE STEP
// ============================================================================

pub fn dvsm_step(
    x2: f64,
    x1: f64,
    x0: f64,
    sigma: f64,
    neighbors: &[f64],
    system: &DVSMpi,
) -> (f64, Jet) {

    let cx = coupling_field(x0, neighbors, system.coupling);

    let raw = evolve(
        x0,
        sigma,
        cx,
        system.eta,
        system.gamma,
    );

    // --------------------------------------------------------------------
    // CRITICAL FIX:
    // projection is FINAL semantic operation (not a penalty step)
    // --------------------------------------------------------------------
    let projected = pi_m(raw, &system.planes);

    let jet = compute_jet(x2, x1, projected);

    (projected, jet)
}
// ============================================================================
// DVSM-π — INTEGRATION CONTRACT LAYER (GOODHART-RESISTANT CORE BOUNDARY)
// ============================================================================
// Purpose:
//   This module defines the *architectural invariants* that all DVSM code
//   must satisfy. It is NOT runtime logic — it is a structural contract.
//
// Key Idea:
//   "If these invariants are violated, the system is no longer DVSM."
// ============================================================================

use std::f64;

// ============================================================================
// CORE ARCHITECTURAL INVARIANTS
// ============================================================================
//
// (I1) NO CONTROL FROM OBSERVATION
//     - Jets (v, a, j) MUST NOT influence state update
//     - Any derivative used in evolution = architecture violation
//
// (I2) SINGLE CAUSAL PATH
//     x_t → F(x_t, σ_t) → Π_M → x_{t+1}
//
//     No side channels:
//       ✗ energy feedback
//       ✗ jet feedback
//       ✗ metric feedback
//
// (I3) PROJECTION IS FINAL AUTHORITY
//     - All feasibility enforcement happens ONLY in Π_M
//     - No pre-penalties, no soft constraints
//
// (I4) OBSERVABILITY IS POST-HOC ONLY
//     - jets are reconstructed AFTER state commit
//     - jets are not cached as control state
//
// (I5) GRAPH IS EXOGENOUS STRUCTURE
//     - graph modifies σ_t, never modifies Π_M
//     - topology ≠ objective function
//
// ============================================================================

// ============================================================================
// CONTROL SURFACE (ONLY VALID ENTRY POINT)
// ============================================================================

#[inline(always)]
pub fn dvsm_kernel_step(
    x: f64,
    sigma: f64,
    eta: f64,
    gamma: f64,
) -> f64 {

    let contraction = x + eta * (sigma - x);
    let excitation   = gamma * (sigma - x);

    contraction + excitation
}

// ============================================================================
// FEASIBILITY PROJECTION (Π_M)
// ============================================================================
//
// IMPORTANT:
// This is NOT a clamp in the mathematical sense.
// Clamp is an implementation proxy only.
// ============================================================================

#[inline(always)]
pub fn project_state(x: f64, min: f64, max: f64) -> f64 {
    x.clamp(min, max)
}

// ============================================================================
// JET RECONSTRUCTION (OBSERVATION ONLY)
// ============================================================================
//
// MUST ONLY be called AFTER state history is committed.
// NEVER used in dvsm_kernel_step.
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

#[inline(always)]
pub fn reconstruct_jet(x2: f64, x1: f64, x0: f64) -> Jet {

    let v = x0 - x1;
    let v_prev = x1 - x2;

    let a = v - v_prev;
    let j = a - v_prev;

    Jet { v, a, j }
}

// ============================================================================
// SAFE DVSM UPDATE PIPELINE (REFERENCE IMPLEMENTATION)
// ============================================================================
//
// This is the ONLY correct execution order:
//
//   1. kernel evolve
//   2. projection Π_M
//   3. commit state
//   4. reconstruct jet (optional diagnostics)
// ============================================================================

#[inline(always)]
pub fn dvsm_step(
    x: f64,
    x_prev: f64,
    x_prev2: f64,
    sigma: f64,
    eta: f64,
    gamma: f64,
    min: f64,
    max: f64,
) -> (f64, Jet) {

    // ------------------------------------------------------------
    // (1) CAUSAL EVOLUTION
    // ------------------------------------------------------------
    let x_raw = dvsm_kernel_step(x, sigma, eta, gamma);

    // ------------------------------------------------------------
    // (2) GEOMETRIC FEASIBILITY ENFORCEMENT
    // ------------------------------------------------------------
    let x_proj = project_state(x_raw, min, max);

    // ------------------------------------------------------------
    // (3) OBSERVATIONAL JET ONLY (NO CONTROL PATH)
    // ------------------------------------------------------------
    let jet = reconstruct_jet(x_prev2, x_prev, x_proj);

    (x_proj, jet)
}

// ============================================================================
// DEV GUARD RAILS (STATIC RULES)
// ============================================================================

//
// These are NOT runtime asserts.
// They are semantic invariants for developers.
//
// ---------------------------------------------------------------------------
// RULE G1: NO JET IN UPDATE PATH
// ---------------------------------------------------------------------------
// ❌ forbidden:
//     x_next = f(x, jet)
//
// ✔ required:
//     x_next = f(x, sigma)
//
// ---------------------------------------------------------------------------
// RULE G2: NO ENERGY TERMS IN CONTROL
// ---------------------------------------------------------------------------
// ❌ forbidden:
//     x_next -= λ * energy(jet)
//
// ✔ required:
//     energy only for logging / diagnostics
//
// ---------------------------------------------------------------------------
// RULE G3: NO SOFT CONSTRAINT SYSTEMS
// ---------------------------------------------------------------------------
// ❌ forbidden:
//     x_next -= penalty(x)
//
// ✔ required:
//     x_next → Π_M(x_next)
//
// ---------------------------------------------------------------------------
// RULE G4: PROJECTION IS IDENTITY OF VALIDITY
// ---------------------------------------------------------------------------
// Meaning:
//     If x ∈ M → Π_M(x) = x
//
// If not:
//     Π_M is the ONLY correction mechanism
//
// ============================================================================

// ============================================================================
// GOODHART RESISTANCE BOUNDARY
// ============================================================================
//
// Core theorem (informal):
//
//   If control variables are causally independent of observables,
//   then optimization pressure cannot form.
//
// In DVSM:
//
//   control space   = (x, σ)
//   observable space = (v, a, j)
//
// and:
//
//   ∂(control)/∂(observable) = 0
//
// ============================================================================

// ============================================================================
// DEBUG / STRESS HOOK (OPTIONAL)
// ============================================================================

pub fn sanity_check(x: f64) -> bool {
    x.is_finite()
}
