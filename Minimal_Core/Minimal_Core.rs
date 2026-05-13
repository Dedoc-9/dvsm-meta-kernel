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

            let next_eta = node_i.eta * (1.0 - node_i.eta);

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
