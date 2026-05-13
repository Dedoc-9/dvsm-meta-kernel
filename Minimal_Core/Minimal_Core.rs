// ============================================================================
// DVSM — DISTRIBUTED GRAPH-COUPLED CONTRACTION SYSTEM
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
