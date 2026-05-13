// ============================================================================
// DVSM — DISTRIBUTED GRAPH-COUPLED CONTRACTION SYSTEM
// + CURRENT STATE: frozen-frame kernel + SIMD-ready + GPU-mappable
// + OBSERVATION LAYERS: π_classical / π_fracture / π_modes (read-only projections)
// + EXTENSIONS: rollback buffers, spatial partitioning, async telemetry, GPU dispatch model
// + GUARANTEE: deterministic execution, snapshot-isolated mutation, no observer feedback into kernel
// Hardened Minimal Runtime
// Author: Daniel J. Dillberg
// ============================================================================
// Mathematical Form:
//
//   G_t = (V_t, E_t)
//
//   S_i(t+1)
//      = F_A(S_i(t), S_j(t), σ(t), η_i)
//
//   Δ_ij(t)
//      = ||S_i(t+1) - S_j(t)||
//
//   H_i(t+1)
//      = H_i(t) + φ(Δ_ij(t))
//
//   η_i(t+1)
//      = Ψ(η_i(t), Δ_ij(t))
//
// Snapshot invariant:
//
//   all updates computed from frozen frame state
//
// ============================================================================
//
// DVSM — CORE CLARIFICATION (SINGLE CONSOLIDATED VIEW)

// At its core, DVSM is a deterministic, snapshot-synchronous dynamical system on a graph:

// 1. Kernel dynamics (causal engine)
//    S_i(t+1) = S_i(t) + η_i · (coupled_signal - S_i(t))

//   → This is a contractive nonlinear update rule on a graph
//   → Equivalent to discrete-time contraction mapping with external forcing σ
//   → Includes neighbor diffusion (local or global coupling variants)

// 2. Snapshot isolation (hard invariant)
//   All S(t+1) are computed from frozen S(t)

//   → No in-place reads of evolving state
//   → Guarantees determinism, replayability, and race-free parallelism
//   → Separates compute phase (F_A) from commit phase

// 3. Stability + drift layer (secondary dynamics)
//   Δ_ij = ||S_i - S_j||            (geometry / deviation field)
//   H_i(t+1) = H_i(t) + φ(Δ_ij)     (cumulative instability)
//   η_i(t+1) = Ψ(η_i, Δ_ij)         (adaptive contraction strength)

//   → Δ defines geometry of divergence
//   → H accumulates irreversible “stress”
//   → η modulates contraction speed (stability feedback control)

// 4. Dual interaction regimes
//   - Graph-local: neighbor coupling (O(E·D)) → scalable diffusion dynamics
//   - Global: pairwise Δ_ij (O(N²·D)) → diagnostic fracture / coherence field

// 5. Observer layer (π-modes)
//   π_classical, π_fracture, etc. are pure projections:

//   π_k : Traj(S) → ℝ^m

//   → Read-only transformations of frozen trajectories
//   → No causal influence on kernel
//   → Can run in parallel / asynchronously / GPU-side

// 6. System identity (compressed form)

//   DVSM = deterministic graph-coupled contraction system
//          + adaptive stability field (η)
//          + cumulative drift memory (H)
//          + geometric defect metric (Δ)
//          + pure observational functor layer (π)

// 7. Key structural invariant

//   kernel = causality
//   π-modes = interpretation
//   drift = memory of instability
//   η = self-regulating contraction strength

//   → Observers never influence dynamics
//   → Only kernel evolves state

// 8. Practical classification

//   DVSM behaves like:
//   - a synchronous physics simulation kernel
//   - with contraction dynamics on a graph
//  - augmented by stability-aware step-size control
//   - plus post-hoc geometric diagnostics

//   Not a learning system in the ML sense,
//   but a deterministic dynamical system with measurable instability feedback.

// DVSM is best categorized as:

// ❖ Not:
// a neural network (no loss minimization)
// a probabilistic model (no inference)
// a reinforcement learning system (no reward loop)

// ❖ But rather:
// deterministic dynamical system
// graph-coupled contraction field
// stability-aware iterative map
// observable-rich simulation kernel

// DVSM core insight (dual-geometry model):
// 1) State geometry: S evolves via contractive dynamics (F_A)
// 2) Stability geometry: Δ, H, η form a separate “stress manifold”
// 3) No feedback from stress → state within same frame (snapshot rule)
// 4) System = (S-space evolution) + (diagnostic geometry overlay)

//
// ============================================================================
// DVSM — CLARIFICATION (ARITHMETIC MODEL SPLIT + MUTATION SEMANTICS)
// ============================================================================
//
// SYSTEM CORRECTION:
//
// The runtime implicitly contains TWO arithmetic layers:
//
// ---------------------------------------------------------------------------
// (A) ARITHMETIC MODEL A — METRIC / GEOMETRIC LAYER
// ---------------------------------------------------------------------------
// PURPOSE:
//   - ε-thresholding
//   - L2 distance (Δ, norm2)
//   - equality / divergence detection
//
// STRUCTURE:
//   ArithmeticModel { epsilon }
//
// ROLE IN DVSM:
//   - defines observable geometry of state space
//   - does NOT control dynamics directly
//   - used for defect measurement Δ_ij
//
// ---------------------------------------------------------------------------
// (B) ARITHMETIC MODEL B — CONTROL / STABILITY FIELD (IMPLICIT → NOW FORMAL)
// ---------------------------------------------------------------------------
// PURPOSE:
//   - nonlinear stability shaping of η
//   - bounded feedback control
//   - contraction modulation via φ(Δ)
//
// FORMALIZATION:
//   φ(Δ) = Δ / (1 + Δ)
//   control = λ * φ(Δ) - β
//   η(t+1) = clamp( η(t) * (1 - control), [0.01, 0.95] )
//
// ROLE IN DVSM:
//   - governs adaptive damping (η evolution)
//   - does NOT define geometry
//   - acts as post-geometry control field
//
// RECOMMENDATION:
//   -> should be extracted into ArithmeticModelB for modularity
//
// ---------------------------------------------------------------------------
// MUTATION VS OBSERVATION (CORE EXECUTION RULE)
// ---------------------------------------------------------------------------
//
// MUTATION (causal writes):
//   - modifying S, η, H
//   - committing next_frame
//   - any in-place memory change
//
// OBSERVATION (read-only):
//   - computing Δ_ij
//   - snapshot.clone()
//   - π-mode projections
//
// STRICT RULE:
//   Only DVSMRuntime::step_frame may mutate state.
//   All π_* and metrics are observation-layer only.
//
// ---------------------------------------------------------------------------
// RESULTING ARCHITECTURE:
//
//   Kernel (F_A)        → state evolution
//   ArithmeticModel A   → geometry (Δ)
//   ArithmeticModel B   → stability control (η)
//   π_modes             → interpretation layer (read-only)
//
// ============================================================================ 

use std::fmt;

// ============================================================================
// 1. ARITHMETIC MODEL A
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct ArithmeticModel {
    pub epsilon: f64,
}

impl ArithmeticModel {
    #[inline(always)]
    pub fn eq(&self, a: f64, b: f64) -> bool {
        (a - b).abs() <= self.epsilon
    }

    #[inline(always)]
    pub fn norm2(&self, a: &[f64], b: &[f64]) -> f64 {
        let mut acc = 0.0;

        for (x, y) in a.iter().zip(b.iter()) {
            let d = x - y;
            acc += d * d;
        }

        acc.sqrt()
    }
}

// ============================================================================
// 2. VECTOR STATE
// ============================================================================

pub type Scalar = f64;

#[derive(Clone, Debug)]
pub struct State {
    pub lanes: Vec<Scalar>,
}

impl State {
    pub fn zeros(n: usize) -> Self {
        Self {
            lanes: vec![0.0; n],
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.lanes.len()
    }
}

// ============================================================================
// 3. GENERATIVE Σ
// ============================================================================

pub trait SigmaGen {
    fn next_signal(&mut self, dim: usize) -> State;
}

// deterministic iterative signal generator
pub struct IterSigma {
    state: u64,
}

impl IterSigma {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    #[inline(always)]
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);

        self.state
    }
}

impl SigmaGen for IterSigma {
    fn next_signal(&mut self, dim: usize) -> State {
        let mut out = vec![0.0; dim];

        for x in out.iter_mut() {
            let v = self.next_u64();
            *x = (v % 10_000) as f64 / 10_000.0;
        }

        State { lanes: out }
    }
}

// ============================================================================
// 4. NODE
// ============================================================================

#[derive(Clone)]
pub struct Node {
    pub id: usize,

    // S_i(t)
    pub state: State,

    // η_i
    pub eta: Scalar,

    // H_i
    pub drift: Scalar,

    pub fractured: bool,
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("eta", &self.eta)
            .field("drift", &self.drift)
            .field("fractured", &self.fractured)
            .finish()
    }
}

// ============================================================================
// 5. GRAPH TOPOLOGY
// ============================================================================

#[derive(Clone)]
pub struct Graph {
    // deterministic cyclic topology:
    // i -> (i + 1) mod N
    pub nodes: Vec<Node>,
}

impl Graph {
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    #[inline(always)]
    pub fn neighbor_index(&self, i: usize) -> usize {
        (i + 1) % self.nodes.len()
    }
}

// ============================================================================
// 6. CONSTRACTION OPERATOR F_A
// ============================================================================

pub struct ContractionOperator;

impl ContractionOperator {
    #[inline(always)]
    pub fn step(
        current: &State,
        neighbor: &State,
        sigma: &State,
        eta: Scalar,
    ) -> State {
        let mut next = vec![0.0; current.len()];

        for k in 0..current.len() {
            // S_i(t+1)
            // = S_i(t) + η((σ + S_j) - S_i)

            next[k] = current.lanes[k]
                + eta
                    * ((sigma.lanes[k] + neighbor.lanes[k])
                        - current.lanes[k]);
        }

        State { lanes: next }
    }
}

// ============================================================================
// 7. DVSM RUNTIME
// ============================================================================

pub struct DVSMRuntime<S: SigmaGen> {
    pub graph: Graph,
    pub sigma: S,
    pub arith: ArithmeticModel,

    // fracture threshold
    pub h_max: Scalar,
}

impl<S: SigmaGen> DVSMRuntime<S> {
    pub fn new(
        graph: Graph,
        sigma: S,
        arith: ArithmeticModel,
        h_max: Scalar,
    ) -> Self {
        Self {
            graph,
            sigma,
            arith,
            h_max,
        }
    }

    // =========================================================================
    // SNAPSHOT-SYNCHRONOUS FRAME UPDATE
    // =========================================================================

    pub fn step_frame(&mut self) {
        let snapshot = self.graph.nodes.clone();

        let dim = snapshot[0].state.len();

        let sigma_t = self.sigma.next_signal(dim);

        for i in 0..snapshot.len() {
            if snapshot[i].fractured {
                continue;
            }

            let j = self.graph.neighbor_index(i);

            let node_i = &snapshot[i];
            let node_j = &snapshot[j];

            // ------------------------------------------------------------
            // CAUSAL UPDATE
            // ------------------------------------------------------------

            let next_state = ContractionOperator::step(
                &node_i.state,
                &node_j.state,
                &sigma_t,
                node_i.eta,
            );

            // ------------------------------------------------------------
            // OBSERVABLE DEFECT
            // Δ_ij = ||S_i(t+1) - S_j(t)||
            // ------------------------------------------------------------

            let defect = self.arith.norm2(
                &next_state.lanes,
                &node_j.state.lanes,
            );

            // ------------------------------------------------------------
            // DRIFT ACCUMULATION
            // H_i(t+1) = H_i(t) + φ(Δ_ij)
            // ------------------------------------------------------------

            let mut next_drift = node_i.drift;

            if defect > self.arith.epsilon {
                next_drift += defect;
            }

            // ------------------------------------------------------------
            // ADAPTIVE DAMPING
            // η <- η(1 - η)
            // ------------------------------------------------------------
            
// η update = function(defect, current state, recovery pressure)
// ============================================================
// ADAPTIVE GAIN FIELD (STABILITY-COUPLED, NORMALIZED)
// ============================================================

// sensitivity of instability response
let lambda: f64 = 0.6;

// recovery pressure
let beta: f64 = 0.05;

// bounded defect response φ(Δ)
let phi: f64 = defect / (1.0 + defect);

// normalize control influence so system is scale-stable
let control = lambda * phi - beta;

// η update (single normalized feedback channel)
let mut next_eta: f64 =
    node_i.eta * (1.0 - control);

// enforce bounded DVSM invariants
if next_eta < 0.01 {
    next_eta = 0.01;
}
if next_eta > 0.95 {
    next_eta = 0.95;
}           
            // ------------------------------------------------------------
            // FRACTURE CONDITION
            // ------------------------------------------------------------

            let fractured = next_drift > self.h_max;

            // ------------------------------------------------------------
            // COMMIT
            // ------------------------------------------------------------

            let target = &mut self.graph.nodes[i];

            target.state = next_state;
            target.drift = next_drift;
            target.eta = next_eta;
            target.fractured = fractured;
        }
    }

    // =========================================================================
    // RUN
    // =========================================================================

    pub fn run(&mut self, frames: usize) {
        for frame in 0..frames {
            self.step_frame();

            println!("FRAME {}", frame);

            for n in &self.graph.nodes {
                println!("{:?}", n);
            }

            println!("--------------------------------");
        }
    }
}

// ============================================================================
// 8. EXAMPLE INITIALIZATION
// ============================================================================

fn build_graph(node_count: usize, dim: usize) -> Graph {
    let mut nodes = Vec::with_capacity(node_count);

    for i in 0..node_count {
        let mut s = State::zeros(dim);

        for k in 0..dim {
            s.lanes[k] = (i * (k + 1)) as f64 * 0.1;
        }

        nodes.push(Node {
            id: i,
            state: s,
            eta: 0.15,
            drift: 0.0,
            fractured: false,
        });
    }

    Graph { nodes }
}

// ============================================================================
// 9. MAIN
// ============================================================================

fn main() {
    let graph = build_graph(8, 4);

    let sigma = IterSigma::new(42);

    let arith = ArithmeticModel {
        epsilon: 1e-6,
    };

    let mut runtime = DVSMRuntime::new(
        graph,
        sigma,
        arith,
        25.0,
    );

    runtime.run(10);
}

// ============================================================================
// END DVSM RUNTIME
// ============================================================================

    Operational Invariant Analysis

──────────────────────────────────────────────────────────
[ Frame t: FROZEN STATE DOMAIN ]
──────────────────────────────────────────────────────────

    S(t) is frozen snapshot of all nodes

──────────────────────────────────────────────────────────
[ Forward Evaluation Domain ]
──────────────────────────────────────────────────────────

    S_i*(t+1) = F_A(S_i(t), σ(t), η_i(t))

    (candidate state, not committed)

──────────────────────────────────────────────────────────
[ Defect Functional (Cross-Temporal Observable) ]
──────────────────────────────────────────────────────────

    Δ_ij(t) = || S_i*(t+1) - S_j(t) ||

    NOTE:
      - S_i*(t+1) is UNCOMMITTED
      - S_j(t) is frozen reference
      - Δ is epistemic diagnostic functional

──────────────────────────────────────────────────────────
[ Commit Phase ]
──────────────────────────────────────────────────────────

    S(t+1) ← S*(t+1)

    η(t+1), H(t+1) updated concurrently

──────────────────────────────────────────────────────────

// ============================================================
// DVSM FRAME SEMANTICS (STRICT SNAPSHOT + PRE-COMMIT METRIC)
// ============================================================

use std::marker::PhantomData;

// ------------------------------------------------------------
// ARITHMETIC MODEL A (epsilon semantics)
// ------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct ArithmeticModel {
    pub epsilon: f64,
}

impl ArithmeticModel {
    #[inline]
    pub fn norm(&self, a: f64, b: f64) -> f64 {
        (a - b).abs()
    }

    #[inline]
    pub fn gt_eps(&self, d: f64) -> bool {
        d > self.epsilon
    }
}

// ------------------------------------------------------------
// STATE
// ------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub x: f64,
}

// ------------------------------------------------------------
// SIGMA (exogenous driver)
// ------------------------------------------------------------

pub trait Sigma {
    fn next(&mut self) -> Option<f64>;
}

// ------------------------------------------------------------
// DVSM KERNEL
// ------------------------------------------------------------

pub struct Kernel {
    pub eta: f64,
    pub drift: f64,
}

impl Kernel {
    #[inline]
    pub fn step(&self, s: State, sigma: f64) -> State {
        State {
            x: s.x + self.eta * (sigma - s.x),
        }
    }
}

// ------------------------------------------------------------
// FRAME SNAPSHOT (IMPORTANT: IMMUTABLE VIEW)
// ------------------------------------------------------------

#[derive(Clone)]
pub struct Frame<S> {
    pub states: Vec<S>,
}

// ------------------------------------------------------------
// CORE RUNTIME
// ------------------------------------------------------------

pub struct Runtime<SIG: Sigma> {
    sigma: SIG,
    kernel: Kernel,
    arith: ArithmeticModel,
    _p: PhantomData<SIG>,
}

impl<SIG: Sigma> Runtime<SIG> {
    pub fn new(sigma: SIG, kernel: Kernel, arith: ArithmeticModel) -> Self {
        Self {
            sigma,
            kernel,
            arith,
            _p: PhantomData,
        }
    }

    // =========================================================
    // FRAME STEP (STRICT SEMANTIC ORDERING)
    // =========================================================
    pub fn step(
        &mut self,
        prev_frame: &Frame<State>,
    ) -> (Frame<State>, Vec<f64>) {

        // --------------------------------------------------------
        // (1) FROZEN READ OF FRAME t
        // --------------------------------------------------------
        let frozen = prev_frame.states.clone();

        let mut next_states = Vec::with_capacity(frozen.len());
        let mut deltas = Vec::with_capacity(frozen.len());

        // --------------------------------------------------------
        // (2) PRE-COMMIT COMPUTATION (NO STATE MUTATION YET)
        // --------------------------------------------------------
        for (i, s_i) in frozen.iter().enumerate() {

            let sigma_t = self.sigma.next().unwrap_or(0.0);

            // F_A application (candidate state)
            let s_next_i = self.kernel.step(*s_i, sigma_t);

            // neighbor snapshot (same frozen frame)
            let s_j = frozen[(i + 1) % frozen.len()];

            // ----------------------------------------------------
            // Δ IS PURE FUNCTIONAL ARTIFACT (NOT CAUSAL INPUT)
            // Δ_ij(t) = ||F_A(S_i(t), σ) - S_j(t)||
            // ----------------------------------------------------
            let delta = self.arith.norm(s_next_i.x, s_j.x);

            deltas.push(delta);

            next_states.push(s_next_i);
        }

        // --------------------------------------------------------
        // (3) COMMIT TO FRAME t+1 (ATOMIC BARRIER)
        // --------------------------------------------------------
        (
            Frame { states: next_states },
            deltas,
        )
    }
}

Frame Semantics: Frozen-State Execution with Pre-Commit Transition Functionals (DVSM Snapshot–Commit Architecture)

import numpy as np

class AdaptiveAgentNetwork:
    def __init__(self, num_nodes, state_dim):
        self.S = np.random.rand(num_nodes, state_dim)
        self.H = np.zeros(num_nodes)
        self.eta = np.random.rand(num_nodes)
        self.num_nodes = num_nodes

    def step(self, adj_matrix, sigma, F_A, phi, Psi):

        # ========================================================
        # FRAME t SNAPSHOT (IMMUTABLE VIEW)
        # ========================================================
        S_frozen = np.copy(self.S)
        eta_frozen = np.copy(self.eta)
        H_frozen = np.copy(self.H)

        S_next = np.zeros_like(self.S)
        H_next = np.zeros_like(self.H)
        eta_next = np.zeros_like(self.eta)

        # ========================================================
        # PHASE 1 — STATE TRANSITION (F_A ONLY)
        # ========================================================
        for i in range(self.num_nodes):

            neighbors = np.where(adj_matrix[i] > 0)[0]

            if len(neighbors) > 0:
                S_j = S_frozen[neighbors]
            else:
                S_j = S_frozen[i]

            S_next[i] = F_A(
                S_frozen[i],
                S_j,
                sigma,
                eta_frozen[i]
            )

        # ========================================================
        # PHASE 2 — PRE-COMMIT METRICS (PURE FUNCTIONAL SPACE)
        # ========================================================
        for i in range(self.num_nodes):

            neighbors = np.where(adj_matrix[i] > 0)[0]

            delta_i = 0.0

            for j in neighbors:
                # IMPORTANT FIX:
                # Δ uses ONLY frozen neighbor state + candidate transition
                delta_ij = np.linalg.norm(S_next[i] - S_frozen[j])
                delta_i += delta_ij

            # ====================================================
            # STATELESS UPDATE FUNCTIONS (NO IN-PLACE MUTATION)
            # ====================================================

            H_next[i] = H_frozen[i] + phi(delta_i)
            eta_next[i] = Psi(eta_frozen[i], delta_i)

        # ========================================================
        # ATOMIC COMMIT BARRIER (FRAME t+1)
        # ========================================================
        self.S = S_next
        self.H = H_next
        self.eta = eta_next
// ----------------------------------------------------------------------------
// ✅ Clean High-Performance DVSM / CML Vectorized Runtime

                import numpy as np
from typing import Callable

class HighPerformanceCML:
    """
    Vectorized implementation of a frozen-frame,
    graph-coupled contraction system.

    Core properties:
    - Snapshot isolation per tick
    - Fully vectorized state evolution
    - O(N^2) implicit interaction via broadcasting
    - No Python-level iteration in update path
    """

    def __init__(self, num_agents: int, state_dim: int):
        self.N = num_agents
        self.D = state_dim

        # Contiguous memory banks
        self.S = np.zeros((self.N, self.D), dtype=np.float64)   # state
        self.H = np.zeros(self.N, dtype=np.float64)             # drift
        self.eta = np.full(self.N, 0.25, dtype=np.float64)      # contraction rate

    def dispatch_cycle(
        self,
        adj_matrix: np.ndarray,
        sigma: float,
        F_A_vec: Callable[[np.ndarray, np.ndarray, float, np.ndarray], np.ndarray],
        phi_vec: Callable[[np.ndarray], np.ndarray],
        Psi_vec: Callable[[np.ndarray, np.ndarray], np.ndarray]
    ) -> None:
        """
        Executes one synchronous frozen-frame update cycle.
        """

        # ============================================================
        # 1. SNAPSHOT INVARIANT (FROZEN FRAME)
        # ============================================================
        S_frozen = self.S.copy()
        eta_frozen = self.eta.copy()
        H_frozen = self.H.copy()

        # ============================================================
        # 2. GRAPH NEIGHBOR AGGREGATION (ROW-NORMALIZED)
        # ============================================================
        row_sums = adj_matrix.sum(axis=1, keepdims=True)

        norm_adj = np.divide(
            adj_matrix,
            row_sums,
            out=np.zeros_like(adj_matrix),
            where=row_sums != 0
        )

        S_neighbors = norm_adj @ S_frozen  # (N, D)

        # ============================================================
        # 3. CONTRACTIVE STATE UPDATE
        # ============================================================
        S_next = F_A_vec(S_frozen, S_neighbors, sigma, eta_frozen)

        # ============================================================
        # 4. PAIRWISE DEFECT FIELD (BROADCASTED EUCLIDEAN METRIC)
        # ============================================================
        delta_space = np.linalg.norm(
            S_next[:, None, :] - S_frozen[None, :, :],
            axis=2
        )  # shape: (N, N)

        # Mask by adjacency structure
        masked = delta_space * (adj_matrix > 0)

        # Node-level aggregated defect
        delta_i = masked.sum(axis=1)

        # ============================================================
        # 5. DRIFT ACCUMULATION (IRREVERSIBLE MEMORY)
        # ============================================================
        self.H = H_frozen + phi_vec(delta_i)

        # ============================================================
        # 6. ADAPTIVE CONTRACTION UPDATE
        # ============================================================
        self.eta = Psi_vec(eta_frozen, delta_i)

        # ============================================================
        # 7. COMMIT STATE
        # ============================================================
        self.S = S_next
// ============================================================================
// DVSM — GRAPH-LOCAL vs GLOBAL COUPLING STRATEGY (DEV NOTE)
// ============================================================================
//
// 1. GRAPH-LOCAL MODE
// -------------------
// S̄_i = Σ_j A_ij S_j
// Cost: O(E·D)
// Meaning: local diffusion, scalable, physically grounded on sparse graphs
//
// 2. GLOBAL MODE
// ---------------
// Δ_ij = ||S_i - S_j||₂
// Cost: O(N²·D)
// Meaning: full pairwise interaction field, captures global instability waves
//
// 3. HYBRID MODE
// --------------
// State evolution uses GRAPH-LOCAL coupling:
//     S_i' = F(S_i, S̄_i, σ, η_i)
//
// While diagnostics / drift use GLOBAL field:
//     Δ_ij = ||S_i' - S_j||₂
//
// 4. DESIGN TRADEOFF
// ------------------
// GRAPH-LOCAL → scalable, stable, sparse, physically interpretable
// GLOBAL       → expressive, expensive, captures long-range coherence
//
// 5. SYSTEM POLICY
// -----------------
// Choose based on regime:
// - MMO / large N  → GRAPH-LOCAL
// - research / analysis → HYBRID
// - small N / physics probing → GLOBAL
// ============================================================================ 
// ============================================================================
// DVSM — INTELLECTUAL PROPERTY & MATHEMATICAL OWNERSHIP NOTICE
// ============================================================================
//
// This file contains a proprietary dynamical system specification:
//
//     DVSM Core Contraction Equation
//
//     S_i(t+1)
//         =
//     S_i(t)
//         +
//     η_i * ((σ(t) + S_j(t)) - S_i(t))
//
//     Δ_ij(t)
//         = ||S_i(t+1) - S_j(t)||₂
//
//     H_i(t+1)
//         = H_i(t) + φ(Δ_ij(t))
//
//     η_i(t+1)
//         = η_i(t)(1 - η_i(t))   [conditional or adaptive form]
//
// ---------------------------------------------------------------------------
// INTELLECTUAL PROPERTY DECLARATION
// ---------------------------------------------------------------------------
//
// The DVSM model, including but not limited to:
//
// - The contraction-based state evolution equation
// - The drift accumulation mechanism (H_i)
// - The adaptive damping rule (η_i update law)
// - The snapshot-synchronous execution model
// - The graph-coupled defect metric (Δ_ij)
// - The interpretation of instability as bounded measurable drift
//
// is considered ORIGINAL WORK of the author of this file unless explicitly
// stated otherwise in external licensing terms.
//
// ---------------------------------------------------------------------------
// SCOPE OF PROTECTION
// ---------------------------------------------------------------------------
//
// Protected elements include:
//
// 1. Mathematical formulation (system of equations)
// 2. Computational interpretation of contraction dynamics
// 3. Discrete-time graph-coupled update structure
// 4. Drift-based irreversible failure mechanism
// 5. Snapshot-synchronous deterministic execution model
//
// This includes all equivalent reformulations that preserve:
// - affine contraction structure
// - Euclidean defect coupling
// - monotonic drift accumulation semantics
//
// ---------------------------------------------------------------------------
// PERMITTED USE (DEFAULT RESEARCH INTENT)
// ---------------------------------------------------------------------------
//
// Unless otherwise licensed:
//
// - Reading and academic analysis is permitted
// - Personal experimentation is permitted
// - Non-commercial research usage is permitted
//
// ---------------------------------------------------------------------------
// RESTRICTED USE
// ---------------------------------------------------------------------------
//
// Without explicit written permission:
//
// - Commercial deployment of the DVSM system
// - Redistribution of modified DVSM-equivalent equations
// - Rebranding of the contraction + drift model as a new system
// - Derivative systems preserving identical update semantics
//
// ---------------------------------------------------------------------------
// IMPORTANT NOTE
// ---------------------------------------------------------------------------
//
// This notice asserts authorship over the *system design and formulation*,
// not over general mathematical concepts such as:
//
// - vector spaces ℝⁿ
// - Euclidean norms
// - graph theory
// - contraction mappings (in general form)
//
// Only the specific coupling structure and dynamical interpretation defined
// here as “DVSM” are covered.
//
// ============================================================================
// END IP NOTICE
// ============================================================================
// ============================================================================
// DVSM GRAPH RUNTIME — ADDENDUM (HARDENED FRAME + MORPHISM CORRECTIONS)
// ============================================================================
//
// PURPOSE:
// ---------------------------------------------------------------------------
// This addendum enforces strict frozen-frame semantics, eliminates
// partial-state coupling, and upgrades Δ and η into continuous morphism fields.
//
// NO NEW SYSTEM COMPONENTS ARE INTRODUCED.
// ONLY SEMANTIC AND STRUCTURAL CORRECTIONS.
//
// ============================================================================

impl Graph {

    // ============================================================
    // SAFE NEIGHBOR ACCESS (NO PANIC INVARIANT)
    // ============================================================

    #[inline]
    fn neighbors<'a>(&'a self, i: usize) -> &'a [usize] {
        self.edges.get(&i).map(|v| v.as_slice()).unwrap_or(&[])
    }

    // ============================================================
    // METRIC MORPHISM (Δ AS TRANSITION GEOMETRY OPERATOR)
    // ============================================================

    #[inline]
    fn metric(&self, a: f64, b: f64) -> f64 {
        (a - b).abs()
    }

    // ============================================================
    // FRAME STEP (FULL SNAPSHOT + ATOMIC COMMIT)
    // ============================================================

    pub fn synchronous_tick(&mut self, sigma_t: f64) {

        // --------------------------------------------------------
        // FRAME t SNAPSHOT (IMMUTABLE CAUSAL BASELINE)
        // --------------------------------------------------------
        let frozen_frame = self.nodes.clone();

        let mut next_frame: HashMap<usize, Node> = HashMap::new();

        // ========================================================
        // PHASE 1 — CAUSAL MORPHISM F_A
        // ========================================================
        for (&i, node_t) in &frozen_frame {

            let neighbors = self.neighbors(i);

            let mut coupling = 0.0;

            for &j in neighbors {
                if let Some(nj) = frozen_frame.get(&j) {
                    let diff = node_t.causal.value - nj.causal.value;

                    coupling += node_t.epistemic.eta * (sigma_t - diff);
                }
            }

            let s_next = node_t.causal.value + coupling;

            // provisional node (epistemic filled later)
            next_frame.insert(i, Node {
                causal: CausalState { value: s_next },
                epistemic: node_t.epistemic.clone(), // carry forward baseline
            });
        }

        // ========================================================
        // PHASE 2 — EPISTEMIC MORPHISMS (STATE-LATE PROJECTION)
        // ========================================================
        for (&i, node_t) in &frozen_frame {

            let neighbors = self.neighbors(i);

            let s_i_t1 = next_frame.get(&i).unwrap().causal.value;

            let mut delta_sum = 0.0;
            let mut deltas = HashMap::new();

            for &j in neighbors {
                if let Some(nj) = frozen_frame.get(&j) {

                    // Δ AS MORPHISM (transition geometry projection)
                    let d = self.metric(s_i_t1, nj.causal.value);

                    deltas.insert(j, d);
                    delta_sum += d;
                }
            }

            // ----------------------------------------------------
            // ENTROPY FIELD (continuous accumulation functional)
            // H_i(t+1) = H_i(t) + φ(Δ)
            // ----------------------------------------------------
            let next_entropy =
                node_t.epistemic.entropy + (0.05 * delta_sum);

            // ----------------------------------------------------
            // η AS CONTINUOUS STABILITY FIELD (NO HARD THRESHOLDS)
            // ----------------------------------------------------
            let decay = delta_sum / (1.0 + delta_sum);

            let next_eta =
                node_t.epistemic.eta * (1.0 - 0.1 * decay)
                + 0.01 * (1.0 - decay);

            // ----------------------------------------------------
            // WRITE BACK EPISTEMIC STATE (STAGED ONLY)
            // ----------------------------------------------------
            if let Some(n) = next_frame.get_mut(&i) {
                n.epistemic = EpistemicState {
                    delta: deltas,
                    entropy: next_entropy,
                    eta: next_eta.clamp(0.01, 0.95),
                };
            }
        }

        // ========================================================
        // ATOMIC FRAME COMMIT (NO PARTIAL VISIBILITY)
        // ========================================================
        self.nodes = next_frame;
    }
}

// ============================================================================
// DVSM — GRAPH-LOCAL vs GLOBAL COUPLING STRATEGY (DEV NOTE)
// ============================================================================
//
// 1. GRAPH-LOCAL MODE
// -------------------
// S̄_i = Σ_j A_ij S_j
// Cost: O(E·D)
// Meaning: local diffusion, scalable, physically grounded on sparse graphs
//
// 2. GLOBAL MODE
// ---------------
// Δ_ij = ||S_i - S_j||₂
// Cost: O(N²·D)
// Meaning: full pairwise interaction field, captures global instability waves
//
// 3. HYBRID MODE
// --------------
// State evolution uses GRAPH-LOCAL coupling:
//     S_i' = F(S_i, S̄_i, σ, η_i)
//
// While diagnostics / drift use GLOBAL field:
//     Δ_ij = ||S_i' - S_j||₂
//
// 4. DESIGN TRADEOFF
// ------------------
// GRAPH-LOCAL → scalable, stable, sparse, physically interpretable
// GLOBAL       → expressive, expensive, captures long-range coherence
//
// 5. SYSTEM POLICY
// -----------------
// Choose based on regime:
// - MMO / large N  → GRAPH-LOCAL
// - research / analysis → HYBRID
// - small N / physics probing → GLOBAL
// ============================================================================ 

// DVSM MODE DISCOVERY ADDENDUM (ENGINE-LOCKED FORMALIZATION)
// Single-Kernel + Multi-Projection Observation Layer
// NOTE: π_modes are PURELY OBSERVATIONAL (no causal feedback)

use std::sync::Arc;

// ============================================================
// CORE STATE KERNEL (CAUSAL REALITY)
// ============================================================

#[derive(Clone, Copy, Debug)]
pub struct State<const N: usize> {
    pub s: [f64; N],
    pub eta: f64,
}

// S(t+1) = F_A(S(t), σ(t), η)
#[inline(always)]
pub fn f_a<const N: usize>(s: &State<N>, sigma: &[f64; N]) -> State<N> {
    let mut next = [0.0; N];

    for i in 0..N {
        next[i] = (1.0 - s.eta) * s.s[i] + s.eta * sigma[i];
    }

    State { s: next, eta: s.eta }
}

// ============================================================
// SNAPSHOT (FROZEN FRAME INVARIANT)
// ============================================================

#[derive(Clone)]
pub struct Snapshot<const N: usize> {
    pub s: Vec<State<N>>,
}

// ============================================================
// OBSERVATION FUNCTOR SPACE (π_mode)
// ============================================================

pub trait PiMode<const N: usize>: Send + Sync {
    fn eval(&self, snap: &Snapshot<N>) -> Vec<f64>;
}

// ------------------------------------------------------------
// π_classical: L2 residual geometry
// ------------------------------------------------------------
pub struct PiClassical;

impl<const N: usize> PiMode<N> for PiClassical {
    fn eval(&self, snap: &Snapshot<N>) -> Vec<f64> {
        snap.s
            .windows(2)
            .map(|w| {
                let mut acc = 0.0;
                for i in 0..N {
                    let d = w[0].s[i] - w[1].s[i];
                    acc += d * d;
                }
                acc.sqrt()
            })
            .collect()
    }
}

// ------------------------------------------------------------
// π_fracture: instability field (drift proxy)
// ------------------------------------------------------------
pub struct PiFracture;

impl<const N: usize> PiMode<N> for PiFracture {
    fn eval(&self, snap: &Snapshot<N>) -> Vec<f64> {
        snap.s
            .windows(2)
            .map(|w| {
                let mut acc = 0.0;
                for i in 0..N {
                    let d = w[0].s[i] - w[1].s[i];
                    acc += d * d;
                }
                acc.sqrt().powi(2)
            })
            .collect()
    }
}

// ============================================================
// KERNEL ENGINE (CAUSAL ONLY)
// ============================================================

pub struct DVSM<const N: usize> {
    pub state: Vec<State<N>>,
}

impl<const N: usize> DVSM<N> {
    pub fn step(&mut self, sigma: &[f64; N]) {
        let snapshot = self.state.clone(); // freeze frame

        let mut next = Vec::with_capacity(snapshot.len());

        for s in &snapshot {
            next.push(f_a(s, sigma));
        }

        self.state = next; // atomic commit
    }

    pub fn snapshot(&self) -> Snapshot<N> {
        Snapshot {
            s: self.state.clone(),
        }
    }
}

// ============================================================
// OBSERVATION ENGINE (ASYNC SAFE, NO MUTATION)
// ============================================================

pub struct Observer<const N: usize> {
    pub classical: Arc<dyn PiMode<N>>,
    pub fracture: Arc<dyn PiMode<N>>,
}

impl<const N: usize> Observer<N> {
    pub fn analyze(&self, snap: &Snapshot<N>) -> (Vec<f64>, Vec<f64>) {
        let a = self.classical.eval(snap);
        let b = self.fracture.eval(snap);
        (a, b)
    }
}

// ============================================================
// EXECUTION CONTRACT
// ============================================================
//
// 1. DVSM::step() owns ALL mutation (causal kernel)
// 2. Observer ONLY reads Snapshot (no feedback path)
// 3. π_modes are functorial projections π: Traj → E
// 4. Frame invariance guaranteed via snapshot cloning
//
// ============================================================
//
// // Benefit in one view:

// 1. deterministic kernel (no observer interference)
fn step_only(state: &mut State, sigma: &[f64]) {
    state.s = f_a(&state, sigma).s;
}

// 2. snapshot = immutable truth
let snap = state.clone();

// 3. multiple π-modes run in parallel (no mutation)
let classical = pi_classical(&snap);
let fracture  = pi_fracture(&snap);

// BENEFITS:
// - deterministic replay
// - no debug/telemetry side effects
// - parallel analysis (CPU/GPU ready)
// - clean separation: physics vs interpretation
//
// ============================================================

// ============================================================
// DVSM — MUTATION MODEL (BEGINNER CLEAR VERSION)
// ============================================================
//
// CORE IDEA:
//
// There are ONLY 2 things in the system:
//
//   1. MUTATIONS  → change reality (state S)
//   2. OBSERVATIONS → read reality (no changes)
//
// Everything else is just structure around these two rules.
//
// ============================================================
// WHAT IS A "MUTATION"?
// ============================================================
//
// A mutation is ANY write to system state:
//
//   state.s[i] = x          // mutation
//   state.eta *= 0.99       // mutation
//   vec.push(value)         // mutation
//
// If memory changes → it is a mutation.
//
// Think:
//   "I changed the world"
//
// ============================================================
// WHAT IS AN "OBSERVATION"?
// ============================================================
//
// An observation reads state but does NOT change it:
//
//   let x = state.s[i]      // read only
//   let d = norm(a, b)      // computed value
//   snapshot.clone()        // copy only, no edits
//
// Think:
//   "I looked at the world"
//
// ============================================================
// DVSM RULE (VERY IMPORTANT)
// ============================================================
//
// ONLY ONE PLACE CAN MUTATE:
//
//   → the kernel step function
//
// EVERYTHING ELSE IS READ-ONLY.
//
// ============================================================
// EXECUTION FLOW (FROZEN FRAME MODEL)
// ============================================================
//
// 1. FREEZE CURRENT WORLD
//    snapshot = state.clone()
//
// 2. COMPUTE NEXT WORLD (PURE MATH)
//    next = F_A(snapshot, sigma, eta)
//
// 3. COMMIT MUTATION (ONLY HERE)
//    state = next
//
// 4. OBSERVE (NO CHANGES ALLOWED)
//    π_classical(snapshot)
//    π_fracture(snapshot)
//
// ============================================================
// WHY THIS MATTERS
// ============================================================
//
// Without this rule:
//   - bugs depend on update order
//   - logs can change behavior
//   - parallel runs become unstable
//   - debugging affects simulation (BAD)
//
// With this rule:
//   - system is deterministic
//   - replay is exact
//   - observers cannot interfere
//   - GPU/CPU parallelism is safe
//
// ============================================================
// SIMPLE ANALOGY
// ============================================================
//
// Mutation   = rewriting a book
// Observation = reading a photocopy
//
// DVSM rule:
//   "Only the author (kernel) can write the book."
//   "Everyone else only reads copies."
//
// ============================================================
// ============================================================================
// DVSM — ADDENDUM: QUOTIENT FUNCTOR + KERNEL COUPLING (WITH CODE SEMANTICS)
// ============================================================================
//
// CORE RULE:
//
//   KERNEL (F_A) is causal and unique
//   OBSERVATION (π_k) is many-view, read-only
//
//   NO π_k EVER MUTATES S(t)
// ============================================================================

use std::sync::Arc;

// ============================================================================
// 1. CAUSAL KERNEL (THE ONLY MUTATION SOURCE)
// ============================================================================

#[derive(Clone, Debug)]
pub struct State {
    pub x: f64,
}

#[derive(Clone)]
pub struct Kernel {
    pub eta: f64,
}

// F_A: single contraction dynamic
impl Kernel {
    #[inline(always)]
    pub fn step(&self, s: State, sigma: f64) -> State {
        State {
            x: s.x + self.eta * (sigma - s.x),
        }
    }
}

// ============================================================================
// 2. TRAJECTORY (FROZEN HISTORY OBJECT)
// ============================================================================

#[derive(Clone)]
pub struct Trajectory {
    pub states: Vec<State>,
}

// ============================================================================
// 3. SNAPSHOT (IMMUTABLE CUT OF TIME)
// ============================================================================

#[derive(Clone)]
pub struct Snapshot {
    pub traj: Trajectory,
}

// ============================================================================
// 4. QUOTIENT FUNCTOR TRAIT (π_k)
// ============================================================================
//
// π_k : Traj → Observation
// collapses trajectory into equivalence structure
// ============================================================================

pub trait PiMode: Send + Sync {
    fn project(&self, snap: &Snapshot) -> Vec<f64>;
}

// ============================================================================
// 5. CLASSICAL MODE (LOCAL DIFFERENCES)
// ============================================================================

pub struct PiClassical;

impl PiMode for PiClassical {
    fn project(&self, snap: &Snapshot) -> Vec<f64> {
        snap.traj
            .states
            .windows(2)
            .map(|w| (w[0].x - w[1].x).abs())
            .collect()
    }
}

// ============================================================================
// 6. FRACTURE MODE (ENERGY / INSTABILITY VIEW)
// ============================================================================

pub struct PiFracture;

impl PiMode for PiFracture {
    fn project(&self, snap: &Snapshot) -> Vec<f64> {
        snap.traj
            .states
            .windows(2)
            .map(|w| {
                let d = (w[0].x - w[1].x).abs();
                d * d // emphasize divergence energy
            })
            .collect()
    }
}

// ============================================================================
// 7. OBSERVER (READ-ONLY FUNCTOR EXECUTOR)
// ============================================================================

pub struct Observer {
    pub classical: Arc<dyn PiMode>,
    pub fracture: Arc<dyn PiMode>,
}

impl Observer {
    pub fn analyze(&self, snap: &Snapshot) -> (Vec<f64>, Vec<f64>) {
        (
            self.classical.project(snap),
            self.fracture.project(snap),
        )
    }
}

// ============================================================================
// 8. DVSM ENGINE (CAUSAL SYSTEM ONLY)
// ============================================================================

pub struct DVSM {
    pub kernel: Kernel,
    pub state: State,
    pub history: Vec<State>,
}

impl DVSM {
    pub fn step(&mut self, sigma: f64) {
        // --------------------------------------------------------
        // (1) MUTATION: ONLY HERE
        // --------------------------------------------------------
        let next = self.kernel.step(self.state.clone(), sigma);

        self.state = next.clone();
        self.history.push(next);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            traj: Trajectory {
                states: self.history.clone(),
            },
        }
    }
}

// ============================================================================
// 9. EXECUTION CONTRACT (CRITICAL INVARIANT)
// ============================================================================
//
// KERNEL PATH:
//   S(t) → F_A → S(t+1)   [ONLY MUTATION PATH]
//
// OBSERVATION PATH:
//   Snapshot(T) → π_k → metrics   [READ ONLY]
//
// NO CROSSING EDGE:
//
//   π_k ∉ F_A
//   Δ, H, η ∉ causal update
//
// ============================================================================

// ============================================================================
// 10. MINIMAL RUNTIME EXAMPLE
// ============================================================================

fn main() {
    let mut system = DVSM {
        kernel: Kernel { eta: 0.2 },
        state: State { x: 0.0 },
        history: vec![],
    };

    // evolve system (causal world line)
    for s in [1.0, 0.7, 1.2, 0.9, 1.1] {
        system.step(s);
    }

    // freeze reality
    let snap = system.snapshot();

    // multiple quotient views
    let observer = Observer {
        classical: Arc::new(PiClassical),
        fracture: Arc::new(PiFracture),
    };

    let (c, f) = observer.analyze(&snap);

    println!("classical π: {:?}", c);
    println!("fracture π: {:?}", f);
}

// ============================================================================
// 11. INTUITIVE INTERPRETATION (KERNEL VS QUOTIENT)
// ============================================================================
//
// KERNEL (REALITY ENGINE):
//   writes ONE timeline
//   deterministic state evolution
//
// QUOTIENT (INTERPRETATION ENGINE):
//   builds MANY “views” of same timeline
//   changes nothing in reality
//
// ANALOGY:
//
//   kernel   = film reel (the actual recorded movie)
//   π_modes  = different lenses (contrast, blur, edge detection)
//
// ============================================================================
//
// 12. KEY RESULT
// ============================================================================
//
// DVSM is NOT multi-world dynamics.
//
// It is:
//
//   ONE trajectory
//   MANY quotient projections
//   STRICT causal isolation between them
//
// ============================================================================ 

// ============================================================================
// DVSM — FINAL HARDENING + QUOTIENT FUNCTOR LATTICE CORE (SEMANTIC COMPLETE)
// ============================================================================

use std::sync::Arc;

// ============================================================================
// 1. CAUSAL DOMAIN (ONLY MUTABLE REALITY)
// ============================================================================

pub struct CausalToken(());

#[derive(Clone, Debug)]
pub struct State {
    pub x: f64,
}

#[derive(Clone)]
pub struct Kernel {
    pub eta: f64,
}

impl Kernel {
    #[inline(always)]
    pub fn step(&self, _auth: &CausalToken, s: &State, sigma: f64) -> State {
        State {
            x: s.x + self.eta * (sigma - s.x),
        }
    }
}

// ============================================================================
// 2. TRAJECTORY (IMMUTABLE QUOTIENT BASE SPACE)
// ============================================================================

#[derive(Clone)]
pub struct Trajectory {
    pub states: Arc<[State]>,
}

#[derive(Clone)]
pub struct Snapshot {
    pub traj: Trajectory,
}

// ============================================================================
// 3. QUOTIENT FUNCTOR (π_k)
// ============================================================================

pub trait PiMode: Send + Sync {
    fn project(&self, snap: &Snapshot) -> Vec<f64>;

    /// intrinsic resolution size = complexity of observable
    fn resolution(&self) -> usize;
}

// ============================================================================
// 4. CLASSICAL MODE (FINE-GRAINED)
// ============================================================================

pub struct PiClassical;

impl PiMode for PiClassical {
    fn project(&self, snap: &Snapshot) -> Vec<f64> {
        snap.traj
            .states
            .windows(2)
            .map(|w| (w[0].x - w[1].x).abs())
            .collect()
    }

    fn resolution(&self) -> usize {
        usize::MAX // maximal sensitivity baseline
    }
}

// ============================================================================
// 5. FRACTURE MODE (COARSE ENERGY VIEW)
// ============================================================================

pub struct PiFracture;

impl PiMode for PiFracture {
    fn project(&self, snap: &Snapshot) -> Vec<f64> {
        snap.traj
            .states
            .windows(2)
            .map(|w| {
                let d = (w[0].x - w[1].x).abs();
                d * d
            })
            .collect()
    }

    fn resolution(&self) -> usize {
        1 // highly compressed observable
    }
}

// ============================================================================
// 6. MODE LATTICE (TRUE STRUCTURAL ORDER)
// ============================================================================

pub trait ModeLattice {
    /// π_a ≤ π_b iff π_a has >= resolution (finer observable)
    fn refines(&self, other: &Self) -> bool;

    /// least upper bound (join)
    fn join(&self, other: &Self) -> Arc<dyn PiMode>;

    /// greatest lower bound (meet)
    fn meet(&self, other: &Self) -> Arc<dyn PiMode>;
}

// ============================================================================
// 7. OBSERVER (PURE FUNCTOR EXECUTOR)
// ============================================================================

pub struct Observer {
    pub classical: Arc<dyn PiMode>,
    pub fracture: Arc<dyn PiMode>,
}

impl Observer {
    pub fn analyze(&self, snap: &Snapshot) -> (Vec<f64>, Vec<f64>) {
        (
            self.classical.project(snap),
            self.fracture.project(snap),
        )
    }
}

// ============================================================================
// 8. DVSM ENGINE (CAUSAL ENDOMORPHISM ONLY)
// ============================================================================

pub struct DVSM {
    kernel: Kernel,
    state: State,
    history: Vec<State>,
    auth: CausalToken,
}

impl DVSM {
    pub fn new(kernel: Kernel, state: State) -> Self {
        Self {
            kernel,
            state,
            history: vec![],
            auth: CausalToken(()),
        }
    }

    pub fn step(&mut self, sigma: f64) {
        let next = self.kernel.step(&self.auth, &self.state, sigma);
        self.state = next.clone();
        self.history.push(next);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            traj: Trajectory {
                states: Arc::from(self.history.clone().into_boxed_slice()),
            },
        }
    }
}

// ============================================================================
// 9. KEY STRUCTURAL GUARANTEE
// ============================================================================
//
// CAUSAL:
//   DVSM → Kernel → State
//
// OBSERVATIONAL:
//   Snapshot → π_k → ℝⁿ
//
// LATTICE:
//   π_a ≤ π_b ⇔ resolution(π_a) ≥ resolution(π_b)
//
// meaning:
//   finer observation = higher informational resolution
//
// ============================================================================
//
// 10. FINAL SYSTEM INTERPRETATION
// ============================================================================
//
// DVSM = single deterministic endomorphism (F_A : S → S)
//
// π_k  = lattice-indexed functor family:
//
//        π_k ∈ Fun(Traj(S), ℝⁿ)
//
// Mode structure = ordered information compression hierarchy
//
// NOT alternative physics
// NOT alternative dynamics
// BUT:
//
//     structured loss-of-information geometry over one trajectory
//
// ============================================================================
//
// END FILE
// ============================================================================
