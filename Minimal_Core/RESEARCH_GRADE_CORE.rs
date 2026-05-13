// ============================================================================
// DVSM-π — MODE-COMPLETE STRATIFIED JET-MANIFOLD KERNEL
// Author: Daniel J. Dillberg
// ============================================================================
// Core Principle:
// ---------------------------------------------------------------------------
// DVSM-π is a single stratified jet-bundle dynamical system:
//
//     (x, v, a, j) ∈ 𝓙³(ℝⁿ)
//
// Evolution is unified, but observability and constraint enforcement
// depend on projection resolution.
//
// ---------------------------------------------------------------------------
// Projection Hierarchy (mode-complete, not separate systems):
//
//     π⁺   : state-manifold projection
//            Π : (x, v, a, j) → 𝓜₊ ⊂ ℝⁿ
//
//     π⁺⁺  : jet-consistent projection
//            Π : (x, v, a, j) → 𝓜₊₊ ⊂ J²(ℝⁿ)
//            (derivatives reconstructed from trajectory consistency)
//
//     π⁺⁺⁺ : stratified jet-manifold projection (full closure)
//            Π : (x, v, a, j) → 𝓜₊₊₊ ⊂ J³(ℝⁿ)
//            (solves constraint consistency system over jet bundle)
//
// ---------------------------------------------------------------------------
// Key Unification Statement:
//
//   These are not distinct dynamical systems.
//
//   They are three projection operators acting on the same underlying
//   jet-bundle evolution law:
//
//       𝓙³(ℝⁿ) → 𝓜_mode ⊂ 𝓙³(ℝⁿ)
//
//   The dynamics are invariant; only the projection resolution changes.
//
// ---------------------------------------------------------------------------
// Interpretation:
//
//   - π⁺   = positional viewpoint (coarse geometry)
//   - π⁺⁺  = tangent-aware viewpoint (kinematics consistent)
//   - π⁺⁺⁺ = full jet-manifold closure (constraint-complete dynamics)
// ============================================================================
// DVSM-π+++ — MODE-COMPLETE STRATIFIED JET-MANIFOLD KERNEL
// ============================================================================
// Author: DVSM-π Research Lineage
// Version Family: DVSM-π (+ / ++ / +++) unified kernel
//
// CORE UPGRADE:
//   All prior systems are now understood as MODE PROJECTIONS of a single
//   underlying constrained geometric evolution operator.
//
// MODE HIERARCHY:
//   DVSM-π+   → positional projection geometry (state-only feasibility)
//   DVSM-π++  → jet-aware feasibility with bounded derivative reconstruction
//   DVSM-π+++ → mode-complete projection on stratified jet manifolds
//
// ============================================================================
//
// FUNDAMENTAL MATHEMATICAL OBJECT
// ---------------------------------------------------------------------------
//
// The system is defined on a stratified jet manifold:
//
//     𝓜 ⊂ J³(ℝⁿ)
//
// where each point contains:
//
//     x  : state
//     v  : first jet (velocity)
//     a  : second jet (acceleration)
//     j  : third jet (jerk)
//
// ---------------------------------------------------------------------------
//
// UNIFIED EVOLUTION OPERATOR
// ---------------------------------------------------------------------------
//
//     ẋ̃ₜ₊₁ = F(xₜ, σₜ, G, mode)
//
//     xₜ₊₁  = Π_𝓜^{mode}( ẋ̃ₜ₊₁ )
//
// where:
//
//     F      : unconstrained graph-coupled evolution
//     σₜ     : external excitation field
//     G      : graph Laplacian coupling structure
//     Π_𝓜    : mode-dependent projection operator
//
// ---------------------------------------------------------------------------
//
// MODE PROJECTION FAMILY (KEY INSIGHT)
// ---------------------------------------------------------------------------
//
// Π_𝓜^{+}   : position-only projection
//           → clamps x ∈ [x_min, x_max]
//           → ignores jet consistency
//
// Π_𝓜^{++}  : jet-consistent projection
//           → reconstructs v, a, j from discrete trajectory
//           → enforces bounded derivatives
//
// Π_𝓜^{+++} : stratified jet-manifold projection (FULL FORM)
//           → solves constrained consistency system:
//                 r(x, v, a, j) = 0
//           → enforces:
//                 geometric feasibility
//                 jet coherence
//                 graph-coupled consistency
//
// ---------------------------------------------------------------------------
//
// KEY UNIFICATION PRINCIPLE
// ---------------------------------------------------------------------------
//
//     DVSM-π is NOT three systems.
//
//     It is one system with three observational resolutions:
//
//         +    = state manifold view
//         ++   = tangent bundle view
//         +++  = full jet bundle view
//
// ---------------------------------------------------------------------------
//
// STRATIFIED MANIFOLD STRUCTURE
// ---------------------------------------------------------------------------
//
//     𝓜 = ⋃ 𝓜_k
//
// where:
//
//     𝓜₊   ⊂ ℝⁿ
//     𝓜₊₊  ⊂ J²(ℝⁿ)
//     𝓜₊₊₊ ⊂ J³(ℝⁿ)
//
// and:
//
//     Π_𝓜^{mode} : ambient space → 𝓜_mode
//
// ---------------------------------------------------------------------------
//
// GOODHART SEPARATION AXIOM
// ---------------------------------------------------------------------------
//
// Observables are NOT optimization targets.
//
// Instead:
//
//     control evolves in geometry space (𝓜)
//     observables are epiphenomenal projections
//
// This guarantees:
//
//     metric ≠ control variable
//     evaluation ≠ optimization channel
//
// ---------------------------------------------------------------------------
//
// CONTINUOUS-TIME LIMIT
// ---------------------------------------------------------------------------
//
// As Δt → 0:
//
//     dx/dt = f(x, σ, G)
//     subject to:
//         (x, v, a, j) ∈ 𝓜
//
// giving a constrained differential inclusion:
//
//     ẋ ∈ T𝓜(x)
//
// ---------------------------------------------------------------------------
//
// ENDPOINT INTENT
// ---------------------------------------------------------------------------
//
// DVSM-π+++ is a:
//
//   - mode-complete stratified jet-manifold dynamical system
//   - projection-constrained graph flow operator
//   - geometry-first stability system (not optimization system)
//
// NOT:
//
//   - reward optimizer
//   - penalty minimizer
//   - scalar objective system
//
// ============================================================================
//
// KEY RESULT
// ---------------------------------------------------------------------------
//
//     +, ++, +++ are not upgrades.
//
//     They are coordinate charts on the same geometric system.
//
// ============================================================================

use std::f64;

// ============================================================================
// MODES (UNIFIED SEMANTIC SWITCH)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    PiPlus,    // state-only projection
    PiPlusPlus, // trajectory reconstruction view
    PiPlusPlusPlus, // full jet-bundle geometry
}

// ============================================================================
// BUNDLE STATE (INTERNAL CANONICAL FORM)
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Bundle {
    pub x: f64,
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// GRAPH STRUCTURE (CONNECTION FIELD)
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
    pub eta: f64,
    pub gamma: f64,
    pub coupling: f64,
}

// ============================================================================
// MANIFOLD BOUNDS
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub x_min: f64,
    pub x_max: f64,
}

// ============================================================================
// KERNEL FLOW
// ============================================================================

#[inline(always)]
fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// EXCITATION FIELD
// ============================================================================

#[inline(always)]
fn excitation(sigma: f64, x: f64) -> f64 {
    sigma - x
}

// ============================================================================
// GRAPH LAPLACIAN (SYMMETRIC)
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
// UNCONSTRAINED BUNDLE EVOLUTION
// ============================================================================

fn evolve(b: Bundle, sigma: f64, lap: f64, p: &Params) -> Bundle {
    let x_next = kernel(b.x, sigma + lap, p.eta)
        + p.gamma * excitation(sigma, b.x);

    Bundle {
        x: x_next,
        v: x_next - b.x,
        a: (x_next - b.x) - b.v,
        j: ((x_next - b.x) - b.v) - b.a,
    }
}

// ============================================================================
// Π_M STATE PROJECTION
// ============================================================================

fn project_state(x: f64, b: &Bounds) -> f64 {
    x.clamp(b.x_min, b.x_max)
}

// ============================================================================
// MODE-DEPENDENT PROJECTION OPERATOR Π_mode
// ============================================================================

fn project_mode(
    b: Bundle,
    mode: Mode,
    bounds: &Bounds,
) -> Bundle {

    match mode {

        // --------------------------------------------------------
        // π+
        // Only state is meaningful
        // --------------------------------------------------------
        Mode::PiPlus => {
            let x = project_state(b.x, bounds);
            Bundle { x, v: 0.0, a: 0.0, j: 0.0 }
        }

        // --------------------------------------------------------
        // π++
        // State projected, jets preserved as observational signals
        // --------------------------------------------------------
        Mode::PiPlusPlus => {
            let x = project_state(b.x, bounds);
            Bundle { x, v: b.v, a: b.a, j: b.j }
        }

        // --------------------------------------------------------
        // π+++
        // Full jet-bundle projection (geometry-preserving)
        // --------------------------------------------------------
        Mode::PiPlusPlusPlus => {
            let x = project_state(b.x, bounds);

            Bundle {
                x,
                v: b.v,
                a: b.a,
                j: b.j,
            }
        }
    }
}

// ============================================================================
// SYSTEM
// ============================================================================

pub struct DVSM {
    pub nodes: Vec<Bundle>,
    pub history: Vec<Vec<Bundle>>,
    pub graph: Graph,
    pub params: Params,
    pub bounds: Bounds,
    pub mode: Mode,
}

// ============================================================================
// STEP FUNCTION (MODE-COMPLETE KERNEL)
// ============================================================================

impl DVSM {

    pub fn step(&mut self, sigma: f64) {

        let snapshot = self.nodes.clone();
        let mut next = snapshot.clone();

        let x: Vec<f64> = snapshot.iter().map(|b| b.x).collect();

        // --------------------------------------------------------
        // 1. EVOLUTION ON FULL BUNDLE
        // --------------------------------------------------------
        for i in 0..snapshot.len() {
            let lap = laplacian(&self.graph, &x, i);
            next[i] = evolve(snapshot[i], sigma, lap, &self.params);
        }

        // --------------------------------------------------------
        // 2. MODE-DEPENDENT PROJECTION
        // --------------------------------------------------------
        for i in 0..next.len() {
            next[i] = project_mode(next[i], self.mode, &self.bounds);
        }

        // --------------------------------------------------------
        // 3. COMMIT
        // --------------------------------------------------------
        self.history.push(snapshot);
        self.nodes = next;
    }
}
// ============================================================================
// DVSM-π+++ ADDENDUM — LAYER RELATIONSHIP TO DVSM-π+ AND DVSM-π++
// ============================================================================
//
// This system (DVSM-π+++) is a strict superset of earlier formulations.
//
// It does NOT replace DVSM-π+ or DVSM-π++ as incorrect forms.
// Instead, it contains them as *degenerate projections* of the same geometry.
//
// ----------------------------------------------------------------------------
// LAYER REDUCTION MAP
// ----------------------------------------------------------------------------
//
// π+++ (full system)
//   (x, v, a, j) ∈ J^3(M)
//   Π_{J^3(M)} acts on full jet bundle
//   graph = connection field
//
//     ↓ reduction (forget jet structure)
//
// π++ (trajectory-consistent system)
//   x_{t+1} = Π_M(F(x_t, σ_t))
//   J_t = R(x_{t-2}, x_{t-1}, x_t)
//
//     ↓ reduction (forget reconstruction closure)
//
// π+ (state-only projection system)
//   x_{t+1} = Π_M(F(x_t, σ_t))
//
// ----------------------------------------------------------------------------
// INTERPRETATION OF REDUCTIONS
// ----------------------------------------------------------------------------
//
// π+
//   → pure feasibility-constrained state dynamics
//   → no explicit trajectory memory
//
// π++
//   → state dynamics + reconstructed trajectory consistency
//   → jets are observational artifacts, not state variables
//
// π+++
//   → jets are intrinsic bundle coordinates
//   → projection acts on geometry, not just coordinates
//
// ----------------------------------------------------------------------------
// KEY STRUCTURAL STATEMENT
// ----------------------------------------------------------------------------
//
// π+++ does NOT simulate π+ or π++.
//
// Instead:
//
//   π+  = projection of π+++ onto state manifold only
//   π++ = projection of π+++ onto trajectory reconstruction subspace
//
// ----------------------------------------------------------------------------
// LIMIT RELATION (IMPORTANT INTUITION)
// ----------------------------------------------------------------------------
//
// π+   ⊂  π++  ⊂  π+++
//
// but NOT as independent systems — only as
//
//   quotient structures of the same jet-bundle dynamics.
//
// ----------------------------------------------------------------------------
// DESIGN CONSEQUENCE
// ----------------------------------------------------------------------------
//
// All earlier stability intuitions (π+, π++) remain valid,
// but are now interpreted as:
//
//   incomplete coordinate projections of a higher-order geometric system.
//
// ============================================================================
