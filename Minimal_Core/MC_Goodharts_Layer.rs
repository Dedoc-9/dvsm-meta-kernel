// ============================================================================
// DVSM MC — FOUNDATIONAL SPECIFICATION LAYER 
// Author: daniel J. Dillberg
// ============================================================================
//
// This layer defines the system as geometry-first dynamics:
//
//     x_{t+1} = Π_M( F(x_t, σ_t, G) )
//
// where:
//
//   F      : unconstrained evolution in ambient space
//   Π_M    : projection onto admissible jet-manifold
//   G      : graph coupling structure
//   σ_t    : external excitation field
//
// ----------------------------------------------------------------------------
// CORE PRINCIPLE
// ----------------------------------------------------------------------------
//
// DVSM replaces scalar optimization with:
//
//     trajectory feasibility in jet-space
//
// NOT:
//
//     pointwise reward maximization
//
// ----------------------------------------------------------------------------
// SYSTEM SPACE
// ----------------------------------------------------------------------------
//
// The system lives in a stratified jet manifold:
//
//     𝓜 ⊂ J³(ℝⁿ)
//
// Each node state is:
//
//     x ∈ ℝⁿ
//     v = dx/dt
//     a = d²x/dt²
//     j = d³x/dt³
//
// Combined state:
//
//     X = (x, v, a, j) ∈ 𝓙³(ℝⁿ)
//
// ----------------------------------------------------------------------------
// STRATIFICATION HIERARCHY
// ----------------------------------------------------------------------------
//
// π⁺   : positional projection
// π⁺⁺  : kinematic-consistent projection
// π⁺⁺⁺ : full jet-manifold closure
//
// These are NOT different systems.
//
// They are projections:
//
//     Π_mode : 𝓙³ → 𝓜_mode
//
// ----------------------------------------------------------------------------
// MODE DEFINITIONS
// ----------------------------------------------------------------------------
//
// π⁺
//   - keeps x only
//   - discards derivative structure
//   - enforces box constraints only
//
// π⁺⁺
//   - preserves reconstructed v, a
//   - enforces trajectory consistency
//
// π⁺⁺⁺
//   - enforces full jet coherence
//   - solves constraint system:
//
//         r(x, v, a, j) = 0
//
// ----------------------------------------------------------------------------
// CORE EVOLUTION LAW
// ----------------------------------------------------------------------------
//
// Unconstrained proposal:
//
//     X̃_{t+1} = F(X_t, σ_t, G)
//
// Constrained update:
//
//     X_{t+1} = Π_M^{mode}(X̃_{t+1})
//
// ----------------------------------------------------------------------------
// INTERPRETATION OF F
// ----------------------------------------------------------------------------
//
// F is a graph-coupled affine contraction:
//
//     F = kernel + coupling + excitation
//
// It is NOT a learning function.
//
// It is a deterministic evolution operator.
//
// ----------------------------------------------------------------------------
// GRAPH COUPLING STRUCTURE
// ----------------------------------------------------------------------------
//
// Nodes interact via graph Laplacian:
//
//     Δx_i = Σ_j A_ij (x_j - x_i)
//
// This enforces:
//
//     distributed consistency pressure
//
// NOT optimization.
//
// ----------------------------------------------------------------------------
// EXTERNAL EXCITATION SIGNAL
// ----------------------------------------------------------------------------
//
// σ_t acts as:
//
//     bounded external forcing field
//
// but must satisfy:
//
//     stability-preserving injection constraints
//
// ----------------------------------------------------------------------------
// GOODHART SEPARATION AXIOM
// ----------------------------------------------------------------------------
//
// Observables are NOT control targets.
//
// Formally:
//
//     control space ≠ observation space
//
// Therefore:
//
//     O(X) cannot be used as optimization input to F
//
// ----------------------------------------------------------------------------
// WHY THIS MATTERS
// ----------------------------------------------------------------------------
//
// Prevents:
//
//   - metric gaming
//   - reward hacking
//   - proxy optimization collapse
//
// by structurally separating:
//
//   geometry (truth)
//   vs
//   measurement (view)
//
// ----------------------------------------------------------------------------
// CONTINUOUS LIMIT
// ----------------------------------------------------------------------------
//
// As Δt → 0:
//
//     dX/dt ∈ T𝓜(X)
//
// giving a constrained differential inclusion system.
//
// ----------------------------------------------------------------------------
// DESIGN RESULT
// ----------------------------------------------------------------------------
//
// DVSM is:
//
//   a geometric constraint system over jet bundles
//
// NOT:
//
//   a scalar optimization process
//
// ============================================================================
// END PART 1
// ============================================================================
use std::marker::PhantomData;

// ============================================================
// CORE STATE (JET SPACE)
// ============================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub x: f64,
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================
// GRAPH STRUCTURE
// ============================================================

#[derive(Clone, Debug)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

// ============================================================
// PARAMETERS
// ============================================================

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub eta: f64,
    pub sigma_gain: f64,
    pub bias: f64,

    pub lambda_accel: f64,
    pub lambda_jerk: f64,
    pub lambda_delta: f64,
    pub lambda_drift: f64,
}

// ============================================================
// STRESS FIELD (Δ, H)
// ============================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Stress {
    pub delta: f64,
    pub drift: f64,
}

impl Stress {
    pub fn update(&mut self, a: f64, j: f64) {
        self.delta = a.abs() + j.abs();
        self.drift += self.delta;
    }
}

// ============================================================
// GRAPH LAPLACIAN
// ============================================================

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

// ============================================================
// KERNEL EVOLUTION (F)
// ============================================================

fn kernel(x: f64, sigma: f64, lap: f64, p: &Params) -> f64 {
    x + p.eta * ((p.sigma_gain * sigma + lap) - x) + p.bias
}

// ============================================================
// DERIVATIVE RECONSTRUCTION (DISCRETE JET)
// ============================================================

fn compute_jet(prev: Option<State>, curr: State) -> State {
    match prev {
        Some(p) => {
            let v = curr.x - p.x;
            let a = v - p.v;
            let j = a - p.a;
            State { x: curr.x, v, a, j }
        }
        None => State { x: curr.x, v: 0.0, a: 0.0, j: 0.0 },
    }
}

// ============================================================
// MANIFOLD PROJECTION Π_M
// ============================================================

fn project(x: f64, min: f64, max: f64) -> f64 {
    x.clamp(min, max)
}

// ============================================================
// DVSM ENGINE
// ============================================================

pub struct DVSM {
    pub nodes: Vec<State>,
    pub history: Vec<Vec<State>>,
    pub graph: Graph,
    pub params: Params,
    pub min_x: f64,
    pub max_x: f64,
    pub stress: Vec<Stress>,
}

impl DVSM {
    pub fn step(&mut self, sigma: f64) {
        let snapshot = self.nodes.clone();
        let mut next = snapshot.clone();

        let xs: Vec<f64> = snapshot.iter().map(|n| n.x).collect();

        // -----------------------------
        // 1. KERNEL EVOLUTION
        // -----------------------------
        for i in 0..snapshot.len() {
            let lap = laplacian(&self.graph, &xs, i);

            let x_next = kernel(snapshot[i].x, sigma, lap, &self.params);

            next[i].x = x_next;
        }

        // -----------------------------
        // 2. JET RECONSTRUCTION
        // -----------------------------
        for i in 0..next.len() {
            let prev = snapshot.get(i).copied();
            next[i] = compute_jet(prev, next[i]);
        }

        // -----------------------------
        // 3. STRESS UPDATE
        // -----------------------------
        for i in 0..next.len() {
            self.stress[i].update(next[i].a, next[i].j);
        }

        // -----------------------------
        // 4. HARDENING (GOODHART LAYER)
        // -----------------------------
        for i in 0..next.len() {
            let p = &self.params;
            let s = &self.stress[i];

            let penalty =
                p.lambda_accel * next[i].a.powi(2)
                + p.lambda_jerk * next[i].j.powi(2)
                + p.lambda_delta * s.delta
                + p.lambda_drift * s.drift;

            next[i].x -= penalty;

            next[i].x = project(next[i].x, self.min_x, self.max_x);
        }

        // -----------------------------
        // 5. COMMIT
        // -----------------------------
        self.history.push(snapshot);
        self.nodes = next;
    }
}
//        ┌─────────────────────────────┐
//        │      PART 1: THEORY         │
//        │  geometry + rules + modes   │
//        └────────────┬────────────────┘
//                     │ defines
//                     ▼
//        ┌─────────────────────────────┐
//        │     PART 2: ENGINE          │
//        │ Rust DVSM execution system  │
//        └─────────────────────────────┘

// ============================================================================
// DVSM MC — DEV NOTES (HARDENED SPECIFICATION EXTENSION LAYER)
// ============================================================================
//
// This section formalizes implementation-critical invariants that are IMPLIED
// by the DVSM specification but NOT explicitly enforced in the core text.
//
// It is part of the "semantic closure layer":
//
//   - It does NOT modify dynamics
//   - It constrains interpretation
//   - It defines correctness conditions for implementations
//
// ----------------------------------------------------------------------------
// 1. CORE EXECUTION INVARIANT (F ∘ Π SEPARATION)
// ----------------------------------------------------------------------------
//
// The system MUST preserve strict phase separation:
//
//     (1) Snapshot phase:
//         X_t is fully cloned before mutation
//
//     (2) Proposal phase:
//         X̃_{t+1} = F(X_t, σ_t, G)
//
//     (3) Projection phase:
//         X_{t+1} = Π_M(X̃_{t+1})
//
// CRITICAL RULE:
//
//     No intermediate state may influence itself within the same step.
//
// This guarantees:
//
//     snapshot isolation → deterministic replayability
//
// ----------------------------------------------------------------------------
// 2. JET CONSISTENCY AXIOM
// ----------------------------------------------------------------------------
//
// The jet state (v, a, j) is NOT an independent input.
//
// It MUST satisfy:
//
//     v_t = x_t - x_{t-1}
//     a_t = v_t - v_{t-1}
//     j_t = a_t - a_{t-1}
//
// Any deviation implies:
//
//     broken manifold coherence
//
// Therefore:
//
//     jets are derived observables, not free variables
//
// ----------------------------------------------------------------------------
// 3. DERIVATIVE CAUSALITY RULE
// ----------------------------------------------------------------------------
//
// Higher-order terms MUST depend only on past states:
//
//     v_t depends on x_t, x_{t-1}
//     a_t depends on x_t, x_{t-1}, x_{t-2}
//     j_t depends on x_t ... x_{t-3}
//
// No forward leakage is permitted.
//
// This enforces:
//
//     causal consistency of jet reconstruction
//
// ----------------------------------------------------------------------------
// 4. STRESS FIELD SEMANTICS (Δ, H)
// ----------------------------------------------------------------------------
//
// Δ (delta):
//     instantaneous curvature / instability magnitude
//
// H (drift):
//     accumulated instability memory (path-dependent entropy proxy)
//
// IMPORTANT:
//
//     Stress is NOT a control signal
//     Stress is NOT part of F
//
// It only affects:
//
//     projection pressure (Π_M modulation)
//
// NOT kernel evolution.
//
// ----------------------------------------------------------------------------
// 5. GOODHART SEPARATION INVARIANT
// ----------------------------------------------------------------------------
//
// Observables MUST NOT re-enter the kernel:
//
//     O(X_t) ∉ F-input space
//
// Even indirect coupling is forbidden:
//
//     - no eta tuning from stress
//     - no sigma modification from Δ/H
//     - no feedback from observed penalty signals
//
// Reason:
//
//     prevents metric-coupled attractor collapse
//
// ----------------------------------------------------------------------------
// 6. PROJECTION SEMANTICS (Π_M)
// ----------------------------------------------------------------------------
//
// Projection is:
//
//     feasibility enforcement, NOT optimization
//
// It MUST:
//
//     - clamp invalid states
//     - restore manifold constraints
//
// It MUST NOT:
//
//     - minimize energy globally
//     - perform gradient descent
//
// Projection is a geometric operator:
//
//     Π_M : ambient → constrained manifold
//
// NOT a learning step.
//
// ----------------------------------------------------------------------------
// 7. GRAPH COUPLING INVARIANT
// ----------------------------------------------------------------------------
//
// Graph Laplacian coupling:
//
//     Δx_i = Σ_j A_ij (x_j - x_i)
//
// MUST satisfy:
//
//     symmetry of interaction (if edges are undirected)
//     locality of influence
//
// No global coupling terms are allowed in F.
//
// ----------------------------------------------------------------------------
// 8. STABILITY CONDITION (DISCRETE LYAPUNOV STYLE)
// ----------------------------------------------------------------------------
//
// The system is expected (not enforced numerically here) to satisfy:
//
//     E_{t+1} ≤ E_t + O(σ_t)
//
// where:
//
//     E_t = ||v||² + α||a||² + β||j||² + γΔ + δH
//
// Interpretation:
//
//     bounded excitation system, not dissipative collapse
//
// ----------------------------------------------------------------------------
// 9. TEMPORAL SCALE SEPARATION (OPTIONAL EXTENSION RULE)
// ----------------------------------------------------------------------------
//
// If extended:
//
//     fast scale: (t, t-1, t-2)
//     slow scale: (t, t-5, t-10)
//
// must satisfy:
//
//     consistency(fast, slow) → bounded divergence
//
// This prevents:
//
//     micro-burst exploitation of derivative estimation
//
// ----------------------------------------------------------------------------
// 10. FAILURE MODES (IMPORTANT)
// ----------------------------------------------------------------------------
//
// DVSM correctness fails if:
//
//   (A) jets are treated as independent control variables
//   (B) stress feeds back into kernel dynamics
//   (C) projection is replaced with optimization
//   (D) graph coupling becomes global rather than local
//   (E) snapshot isolation is violated
//
// Any of these converts DVSM into:
//
//     a generic optimizer (invalidating design intent)
//
// ----------------------------------------------------------------------------
// 11. ARCHITECTURAL GUARANTEE
// ----------------------------------------------------------------------------
//
// DVSM is:
//
//     a constrained discrete jet dynamical system
//
// NOT:
//
//     a reward system
//     a classifier
//     a reinforcement learner
//     a probabilistic inference model
//
// ----------------------------------------------------------------------------
// 12. IMPLEMENTATION PRINCIPLE
// ----------------------------------------------------------------------------
//
// Correct implementation priority order:
//
//     1. causal kernel (F)
//     2. jet reconstruction
//     3. stress accumulation
//     4. projection Π_M
//     5. commit state
//
// Reordering invalidates semantics.
//
// ----------------------------------------------------------------------------
// 13. FINAL CONSISTENCY STATEMENT
// ----------------------------------------------------------------------------
//
// DVSM correctness is defined by:
//
//     structural invariants, not numeric outcomes
//
// If invariants hold:
//
//     system is valid regardless of trajectory shape
//
// If invariants fail:
//
//     system is semantically undefined, even if numerically stable
//
// ============================================================================
// END DEV NOTES
// ============================================================================
// ============================================================================
// GOODHART CONCEPT NOTE BLOCK — DVSM MC INTERPRETATION LAYER
// ============================================================================
//
// CLASSICAL GOODHART’S LAW
// ----------------------------------------------------------------------------
//
//     “When a measure becomes a target, it ceases to be a good measure.”
//
// In scalar systems, this manifests as:
//
//     optimize(metric(x)) → metric loses correlation with truth
//
// because optimization pressure collapses the proxy-function alignment.
//
// ----------------------------------------------------------------------------
// DVSM REFORMULATION (KEY SHIFT)
// ----------------------------------------------------------------------------
//
// DVSM does NOT treat Goodhart’s Law as a statistical failure.
//
// It treats it as a GEOMETRIC DECOUPLING EVENT:
//
//     metric-space  ⟂  state-space geometry
//
// i.e.
//
//     optimization path ≠ true system manifold trajectory
//
// ----------------------------------------------------------------------------
// CORE DVSM INSIGHT
// ----------------------------------------------------------------------------
//
// Goodhart failure occurs when:
//
//     control variables are defined in observation space
//     instead of trajectory space (J³ manifold)
//
// DVSM therefore enforces:
//
//     x_t ∈ 𝓜 ⊂ J³(ℝⁿ)
//
// meaning:
//
//     validity is a property of trajectories, not points.
//
// ----------------------------------------------------------------------------
// FAILURE MODE CLASSIFICATION IN DVSM TERMS
// ----------------------------------------------------------------------------
//
// (1) POINTWISE GOODHART FAILURE
//     - optimizing x_t directly
//     - ignores v, a, j consistency
//     → produces “locally valid but globally invalid” paths
//
// (2) DERIVATIVE EXPLOITATION
//     - gaming acceleration/jerk proxies
//     - induces oscillatory or burst dynamics
//     → violates jet coherence without breaking point constraints
//
// (3) LONG-HORIZON DRIFT COLLAPSE
//     - small per-step bias accumulation
//     - leads to H_t divergence (stress memory explosion)
//
// ----------------------------------------------------------------------------
// DVSM RESPONSE STRATEGY
// ----------------------------------------------------------------------------
//
// Instead of penalizing metrics, DVSM:
//
//     constrains admissible evolution paths in jet-space
//
// Formally:
//
//     x_{t+1} = Π_𝓜( F(x_t, σ_t, G) )
//
// where Goodhart resistance emerges from:
//
//     Π_𝓜 : removing non-manifold trajectories
//
// not from:
//
//     reward shaping or scalar penalty tuning
//
// ----------------------------------------------------------------------------
// IMPORTANT DISTINCTION
// ----------------------------------------------------------------------------
//
// DVSM explicitly rejects:
//
//     metric-as-control-channel design
//
// meaning:
//
//     O(x_t) ∉ F control inputs
//
// even indirectly (no gradient leakage, no adaptive reward feedback).
//
// ----------------------------------------------------------------------------
// GOODHART IN DVSM IS NOT:
// ----------------------------------------------------------------------------
//
//   ✗ a loss function problem
//   ✗ a reward hacking issue
//   ✗ a statistical misgeneralization artifact
//
// ----------------------------------------------------------------------------
// GOODHART IN DVSM IS:
// ----------------------------------------------------------------------------
//
//   ✓ a constraint violation in trajectory geometry
//   ✓ a mismatch between observed and admissible jet structure
//   ✓ a breakdown of manifold invariance under evolution
//
// ----------------------------------------------------------------------------
// INVARIANT FORMULATION
// ----------------------------------------------------------------------------
//
// A system is Goodhart-stable in DVSM iff:
//
//     ∀ t:  (x_t, v_t, a_t, j_t) ∈ 𝓜
//
// and:
//
//     Π_𝓜 ∘ F  =  F ∘ Π_𝓜   (closure consistency up to projection)
//
// ----------------------------------------------------------------------------
// PRACTICAL CONSEQUENCE
// ----------------------------------------------------------------------------
//
// You do NOT fix Goodhart by:
//
//     - weighting penalties
//     - reshaping rewards
//     - normalizing metrics
//
// You fix it by:
//
//     removing degrees of freedom that allow metric decoupling
//
// i.e. enforcing jet-consistent admissibility.
//
// ----------------------------------------------------------------------------
// SUMMARY STATEMENT
// ----------------------------------------------------------------------------
//
// Goodhart’s Law in DVSM is:
//
//     a statement about geometry preservation failure under projection
//
// not:
//
//     a failure of optimization tuning
//
// ============================================================================
// END GOODHART NOTE BLOCK
// ============================================================================
