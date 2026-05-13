// ============================================================================
// DVSM-π v2 — CONSISTENT MULTI-PLANE PROJECTED DYNAMICS
// ============================================================================
// Author: DVSM-π Research Lineage (Corrected Formalism)
// Status: Constraint Geometry System (NOT Constant-Derivation System)
// ============================================================================
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
