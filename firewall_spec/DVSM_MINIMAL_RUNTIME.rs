============================================================
DVSM MINIMAL RUNTIME — INTRO BLOCK + DEV NOTES + FUNDAMENTALS
Author: Daniel J. Dillberg
============================================================
/*!
# DVSM Minimal Core — Execution Model (v0.1)

## INTRO BLOCK

This file defines the *compressed operational core* of the DVSM system.

All categorical, sheaf-theoretic, and higher-stack constructs have been
eliminated in favor of a single measurable substrate:

- State vectors (S)
- Scalar contraction (η)
- Drift accumulation (H)

The system is intentionally **memoryless per step**, except for a bounded
scalar drift ledger used only for failure detection.

The runtime is a deterministic discrete dynamical system:

    S_{t+1} = S_t + η ((σ_t + S_neighbor) - S_t)

All higher abstractions (connections, torsion, holonomy, curvature) are
reduced to observable Euclidean error accumulation.

---

## WHAT CHANGED IN THIS FILE (FROM PREVIOUS LAYERS)

### 1. Removed abstract categorical layers
Deleted:
- ωₜ (connection form)
- δₜ (torsion tensor)
- Čech cohomology checks
- inverse limits / derived functors
- stackification / Grothendieck topology
- sheaf gluing logic

Replaced with:
- single scalar drift accumulator (H)
- local Euclidean defect measurement (Δ)

---

### 2. Replaced topology with direct neighbor sampling
Old model:
- global cover + Čech nerve + overlap consistency

New model:
- single neighbor state interaction per step

Result:
- O(N²) global consistency checks removed (conceptually)
- replaced with local O(1) interaction (implementation-dependent)

---

### 3. Replaced curvature logic with threshold arithmetic
Old model:
- Ω_J(t), δ_t, holonomy transport laws

New model:
- if Δ > ε → increase drift H, shrink η

Interpretation:
- curvature = error magnitude
- torsion = adaptive damping response
- holonomy = accumulated irreversible drift

---

## DEV NOTES (PRODUCTION ENGINEERING ISSUES)

### 1. Neighbor selection ambiguity (CRITICAL)

Current logic:

    snapshot.values().find(|&&s| s != node.state)

#### Problem:
- unstable under consensus
- self-matching fallback causes invalid excitation:

    σ + S_self

#### Fix requirement:
Use explicit graph adjacency or deterministic peer index:

    neighbor_id ∈ adjacency_list[i]

OR:

    select nearest neighbor by metric distance, not equality

---

### 2. Floating-point equality instability (CRITICAL)

Current logic:

    s != node.state

#### Problem:
- f32 comparisons are not stable across:
  - SIMD reorderings
  - compiler optimizations
  - GPU/CPU divergence

#### Fix:

Replace with epsilon metric:

    |s - node.state| > ε_cmp

or better:

    use L2 distance threshold

---

### 3. Complexity bottleneck (HIGH)

Current structure:
- implicit scan over all snapshot nodes
- leads to O(N²) interaction pattern

#### Fix options:

A. Spatial hashing (recommended)
    - bucket nodes by quantized state space

B. Fixed topology graph
    - adjacency list per node

C. Sparse interaction kernel
    - only k-nearest neighbors

---

## ARITHMETIC FUNDAMENTALS (CORE MODEL ASSUMPTIONS)

### 1. State space
Each node lives in:

    S ∈ ℝⁿ

Typically:
- n = 2, 4, 8, 16 depending on signal resolution

---

### 2. Update rule is affine contraction

    S_{t+1} = (1 - η) S_t + η (σ_t + S_j)

Interpretation:
- convex interpolation between:
  - current state
  - external excitation field

---

### 3. Contraction coefficient η

    η ∈ (0, 1)

Behavior:
- η → 0 : frozen node (no adaptation)
- η → 1 : fully reactive node (unstable if noisy)

Stability condition:

    0 < η < 0.5  (practical stability regime)

---

### 4. Drift accumulation H

Defined as:

    H_{t+1} = H_t + Δ_{ij}

where:

    Δ_{ij} = ||S_i - S_j||

Interpretation:
- irreversible disagreement ledger
- monotonic increasing scalar
- acts as failure detector, not corrective signal

---

### 5. Failure condition

System transitions to FRACTURED state when:

    H_i > H_max

Meaning:
- node has accumulated too much irreducible inconsistency
- cannot be stabilized via local contraction alone

---

## SUMMARY OF SYSTEM BEHAVIOR

This is no longer a geometric stack system.

It is:

> A bounded, contractive, distributed dynamical system with
> scalar drift-based failure detection.

All higher categorical structures were projections of:

- error propagation
- contraction dynamics
- bounded accumulation

---

## NEXT ENGINEERING STEP (OPTIONAL)

If extending this system further, the only meaningful upgrades are:

1. Replace neighbor selection with explicit graph topology
2. Add deterministic synchronization barrier (if distributed)
3. Replace scalar drift H with vector-valued residual memory
4. Introduce stochastic input noise model for stability testing
*/
   
OVERVIEW
--------
This file implements a minimal distributed consensus-style
dynamical system.

Each node:
- Maintains a state vector in R^4
- Updates via contraction toward external + neighbor signals
- Accumulates drift when disagreement exceeds tolerance
- Is removed if drift exceeds a fixed budget

SYSTEM TYPE
-----------
This is a bounded-error consensus dynamical system:
- Not categorical
- Not geometric
- Not proof-based

It is purely:
- iterative
- numerical
- threshold-driven
*/

/*
============================================================
WHAT CHANGED (FROM PREVIOUS ABSTRACT VERSION)
============================================================

1. ALL ABSTRACT STRUCTURES REMOVED
   - No stacks
   - No sheaves
   - No torsion/curvature/holonomy
   - No derived limits or obstruction classes

2. REDUCED TO 2 LAYERS ONLY

   Layer 1: Node dynamics
   - state update equation
   - defect measurement
   - drift accumulation
   - failure threshold

   Layer 2: Network orchestration
   - synchronous tick
   - snapshot-based coupling
   - node pruning

3. ALL “GEOMETRY” REPLACED WITH MEASUREMENT
   - Čech defect → Euclidean distance
   - holonomy → accumulated drift
   - curvature → defect magnitude
   - torsion → η adaptation rule

4. EXECUTION MODEL CHANGED
   - From declarative / algebraic reasoning
   - To deterministic simulation loop
*/

/*
============================================================
DEV NOTES (IMPLEMENTATION MODEL)
============================================================

NODE UPDATE
-----------
S_{t+1} = S_t + η * ((σ + S_neighbor) - S_t)

Interpretation:
- convex interpolation toward combined influence
- η controls responsiveness vs inertia

DEFECT (ERROR SIGNAL)
---------------------
Δ = ||S_{t+1} - S_neighbor||

Used for:
- detecting disagreement
- triggering drift accumulation

DRIFT MODEL
-----------
H += Δ  if Δ > ε

Interpretation:
- tracks persistent instability
- ignores small noise

ADAPTATION RULE
---------------
η = η * (1 - η)

Effect:
- high η decays over time
- system self-stabilizes under stress

FAILURE CONDITION
-----------------
If H > H_budget:
    node is removed

NETWORK MODEL
-------------
- synchronous updates
- snapshot-based neighbor reads
- avoids ordering bias

GRAPH MODEL
-----------
Currently implicit (placeholder):
- first differing node used as neighbor
- should be replaced with explicit adjacency graph
*/

/*
============================================================
ARITHMETIC FUNDAMENTALS
============================================================

CORE OPERATIONS
---------------
1. VECTOR ARITHMETIC (ℝ⁴)
   - addition
   - subtraction
   - scalar interpolation

2. CONVEX COMBINATION
   x' = (1 - η)x + ηy

3. EUCLIDEAN METRIC
   d(x, y) = sqrt(sum_i (x_i - y_i)^2)

4. THRESHOLD LOGIC
   - ε: noise tolerance boundary
   - H_budget: system failure limit

SYSTEM BEHAVIOR
--------------
- continuous evolution (state update)
- discrete control (threshold checks)
- adaptive damping (η update)
*/

use std::collections::HashMap;

/* ============================================================
   LAYER 1 — CORE NODE DYNAMICS
   ============================================================ */

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStatus {
    Stable,
    Fractured,
}

pub struct DvsmoCoreNode {
    pub state: [f32; 4],
    pub eta: f32,
    pub accumulated_drift: f32,

    epsilon: f32,
    drift_budget: f32,
}

impl DvsmoCoreNode {
    pub fn new(state: [f32; 4], eta: f32, epsilon: f32, drift_budget: f32) -> Self {
        Self {
            state,
            eta,
            accumulated_drift: 0.0,
            epsilon,
            drift_budget,
        }
    }

    pub fn step(&mut self, sigma: &[f32; 4], neighbor: &[f32; 4]) -> NodeStatus {
        // State update (convex interpolation form)
        let mut next = [0.0f32; 4];

        for i in 0..4 {
            let excitation = sigma[i] + neighbor[i];
            next[i] = self.state[i] + self.eta * (excitation - self.state[i]);
        }

        // Euclidean defect
        let mut defect = 0.0;
        for i in 0..4 {
            let d = next[i] - neighbor[i];
            defect += d * d;
        }
        defect = defect.sqrt();

        // Drift accumulation
        if defect > self.epsilon {
            self.accumulated_drift += defect;
            self.eta *= 1.0 - self.eta;
        }

        self.state = next;

        if self.accumulated_drift > self.drift_budget {
            NodeStatus::Fractured
        } else {
            NodeStatus::Stable
        }
    }
}

/* ============================================================
   LAYER 2 — NETWORK ORCHESTRATION
   ============================================================ */

pub struct DVSMNetwork {
    pub nodes: HashMap<usize, DvsmoCoreNode>,
}

impl DVSMNetwork {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, id: usize, node: DvsmoCoreNode) {
        self.nodes.insert(id, node);
    }

    pub fn tick(&mut self, sigma: [f32; 4]) {
        // Snapshot states (prevents update ordering bias)
        let snapshot: HashMap<usize, [f32; 4]> =
            self.nodes.iter().map(|(id, n)| (*id, n.state)).collect();

        let mut fractured = vec![];

        for (id, node) in self.nodes.iter_mut() {
            // Minimal neighbor coupling (placeholder logic)
            let neighbor_state = snapshot
                .values()
                .find(|&&s| s != node.state)
                .unwrap_or(&node.state);

            let status = node.step(&sigma, neighbor_state);

            if status == NodeStatus::Fractured {
                fractured.push(*id);
            }
        }

        // Remove failed nodes
        for id in fractured {
            self.nodes.remove(&id);
        }
    }

    pub fn global_state(&self) -> Vec<[f32; 4]> {
        self.nodes.values().map(|n| n.state).collect()
    }
}

/* ============================================================
   EXECUTION HARNESS
   ============================================================ */

fn main() {
    let mut net = DVSMNetwork::new();

    net.add_node(1, DvsmoCoreNode::new([1.0, 0.0, 0.0, 0.0], 0.25, 0.01, 10.0));
    net.add_node(2, DvsmoCoreNode::new([0.9, 0.1, 0.0, 0.0], 0.30, 0.01, 10.0));
    net.add_node(3, DvsmoCoreNode::new([0.8, 0.2, 0.0, 0.0], 0.20, 0.01, 10.0));

    let signal_stream = [
        [0.5, 0.5, 0.0, 0.0],
        [0.6, 0.4, 0.1, 0.0],
        [10.0, -5.0, 2.0, 1.0],
    ];

    for (t, sigma) in signal_stream.iter().enumerate() {
        net.tick(*sigma);
        println!("Step {}: {:?}", t, net.global_state());

        if net.nodes.is_empty() {
            println!("System collapse (all nodes fractured).");
            break;
        }
    }
}
use std::collections::HashMap;

/* ============================================================
   LAYER 1 — CORE NODE DYNAMICS (LOCAL SYSTEM)
   ============================================================ */

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStatus {
    Stable,
    Fractured,
}

pub struct DvsmoCoreNode {
    pub state: [f32; 4],
    pub eta: f32,
    pub accumulated_drift: f32,

    epsilon: f32,
    drift_budget: f32,
}

impl DvsmoCoreNode {
    pub fn new(state: [f32; 4], eta: f32, epsilon: f32, drift_budget: f32) -> Self {
        Self {
            state,
            eta,
            accumulated_drift: 0.0,
            epsilon,
            drift_budget,
        }
    }

    pub fn step(
        &mut self,
        sigma: &[f32; 4],
        neighbor: &[f32; 4],
    ) -> NodeStatus {

        // --- state update ---
        let mut next = [0.0f32; 4];

        for i in 0..4 {
            let excitation = sigma[i] + neighbor[i];
            next[i] = self.state[i] + self.eta * (excitation - self.state[i]);
        }

        // --- defect measurement (Δ) ---
        let mut defect = 0.0;
        for i in 0..4 {
            let d = next[i] - neighbor[i];
            defect += d * d;
        }
        defect = defect.sqrt();

        // --- drift accumulation ---
        if defect > self.epsilon {
            self.accumulated_drift += defect;
            self.eta *= 1.0 - self.eta;
        }

        self.state = next;

        if self.accumulated_drift > self.drift_budget {
            NodeStatus::Fractured
        } else {
            NodeStatus::Stable
        }
    }
}

/* ============================================================
   LAYER 2 — NETWORK ORCHESTRATION (GLOBAL SYSTEM)
   ============================================================ */

pub struct DVSMNetwork {
    pub nodes: HashMap<usize, DvsmoCoreNode>,
}

impl DVSMNetwork {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, id: usize, node: DvsmoCoreNode) {
        self.nodes.insert(id, node);
    }

    /// One global synchronous tick:
    /// - snapshot states (prevents ordering bias)
    /// - apply local updates
    /// - prune fractured nodes
    pub fn tick(&mut self, sigma: [f32; 4]) {
        let snapshot: HashMap<usize, [f32; 4]> =
            self.nodes.iter().map(|(id, n)| (*id, n.state)).collect();

        let mut fractured = vec![];

        for (id, node) in self.nodes.iter_mut() {
            // pick arbitrary neighbor (minimal coupling model)
            let neighbor_state = snapshot
                .values()
                .find(|&&s| s != node.state)
                .unwrap_or(&node.state);

            let status = node.step(&sigma, neighbor_state);

            if status == NodeStatus::Fractured {
                fractured.push(*id);
            }
        }

        // prune unstable nodes
        for id in fractured {
            self.nodes.remove(&id);
        }
    }

    pub fn global_state(&self) -> Vec<[f32; 4]> {
        self.nodes.values().map(|n| n.state).collect()
    }
}

/* ============================================================
   EXECUTION HARNESS
   ============================================================ */

fn main() {
    let mut net = DVSMNetwork::new();

    net.add_node(1, DvsmoCoreNode::new([1.0, 0.0, 0.0, 0.0], 0.25, 0.01, 10.0));
    net.add_node(2, DvsmoCoreNode::new([0.9, 0.1, 0.0, 0.0], 0.30, 0.01, 10.0));
    net.add_node(3, DvsmoCoreNode::new([0.8, 0.2, 0.0, 0.0], 0.20, 0.01, 10.0));

    let signal_stream = [
        [0.5, 0.5, 0.0, 0.0],
        [0.6, 0.4, 0.1, 0.0],
        [10.0, -5.0, 2.0, 1.0], // perturbation event
    ];

    for (t, sigma) in signal_stream.iter().enumerate() {
        net.tick(*sigma);

        println!("Step {}: {:?}", t, net.global_state());

        if net.nodes.is_empty() {
            println!("All nodes fractured — system collapse.");
            break;
        }
    }
}
/*!
# DVSM CORE ALGEBRAIC PROCESS — FUNDAMENTAL EQUATIONS

This module defines the irreducible algebra of the system.

All higher structures (connections, torsion, holonomy, sheaves)
reduce to a single coupled dynamical system:

------------------------------------------------------------
STATE SPACE
------------------------------------------------------------

S_i(t) ∈ ℝⁿ        // local node state
η_i ∈ (0,1)        // contraction coefficient
H_i ∈ ℝ₊           // accumulated drift

σ(t) ∈ ℝⁿ          // external excitation signal
S_j(t) ∈ ℝⁿ        // neighbor state (interaction partner)

------------------------------------------------------------
1. FUNDAMENTAL STATE EVOLUTION EQUATION
------------------------------------------------------------

S_i(t+1) = S_i(t) + η_i * ( (σ(t) + S_j(t)) - S_i(t) )

Equivalent canonical form:

S_i(t+1) = (1 - η_i) S_i(t) + η_i (σ(t) + S_j(t))

------------------------------------------------------------
2. INTERACTION ERROR (OBSERVABLE DEFECT)
------------------------------------------------------------

Δ_ij(t) = || S_i(t+1) - S_j(t+1) ||₂

This is the ONLY observable coupling signal in the system.

------------------------------------------------------------
3. ADAPTIVE CONTRACTION DYNAMICS
------------------------------------------------------------

If Δ_ij(t) > ε:

    η_i ← η_i (1 - η_i)
    H_i ← H_i + Δ_ij(t)

Interpretation:
- η shrinks nonlinearly under instability
- H accumulates irreversible disagreement

------------------------------------------------------------
4. FAILURE / FRACTURE CONDITION
------------------------------------------------------------

Node fractures iff:

H_i > H_max

------------------------------------------------------------
5. GLOBAL SYSTEM INTERPRETATION (REDUCTION LAW)

All higher constructs reduce to:

- curvature  ≡ Δ (pairwise inconsistency)
- torsion    ≡ η update nonlinearity
- holonomy   ≡ Σ Δ over time (path accumulation)

------------------------------------------------------------
6. SYSTEM FIXED POINT CONDITION (STABILITY)

Stable regime satisfies:

S_i(t+1) ≈ S_j(t+1)
Δ_ij(t) → 0
η_i → η* (constant attractor)
H_i bounded

------------------------------------------------------------
END FUNDAMENTAL SYSTEM
*/
/*!
DVSM CORE — n-DIMENSIONAL STATE DYNAMICS (MINIMAL EXECUTION MODEL)

This file implements the irreducible system:

    S_i(t+1) = (1 - η_i) S_i(t) + η_i (σ(t) + S_j(t))

with:
- Δ_ij = ||S_i - S_j||₂
- adaptive η update
- bounded drift H
*/

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeStatus {
    Stable,
    Fractured,
}

const N: usize = 8; // Target dimension (tunable: 3, 4, 8, 16...)

#[derive(Debug, Clone)]
pub struct Node {
    pub state: [f32; N],
    pub eta: f32,
    pub drift: f32,
    pub drift_budget: f32,
    pub epsilon: f32,
}

impl Node {
    pub fn new(state: [f32; N], eta: f32, epsilon: f32, drift_budget: f32) -> Self {
        Self {
            state,
            eta,
            drift: 0.0,
            drift_budget,
            epsilon,
        }
    }

    /// Core algebraic update step:
    /// S_{t+1} = (1 - η)S_t + η(σ + S_neighbor)
    pub fn step(&mut self, sigma: &[f32; N], neighbor: &[f32; N]) -> NodeStatus {
        let mut next = [0.0f32; N];

        // ---- STATE EVOLUTION ----
        for i in 0..N {
            let excitation = sigma[i] + neighbor[i];
            next[i] = (1.0 - self.eta) * self.state[i] + self.eta * excitation;
        }

        // ---- OBSERVABLE DEFECT (Δ_ij) ----
        let mut defect = 0.0f32;
        for i in 0..N {
            let d = next[i] - neighbor[i];
            defect += d * d;
        }
        defect = defect.sqrt();

        // ---- ADAPTATION RULE ----
        if defect > self.epsilon {
            self.drift += defect;
            self.eta *= 1.0 - self.eta; // nonlinear contraction update
        }

        self.state = next;

        // ---- FAILURE CONDITION ----
        if self.drift > self.drift_budget {
            NodeStatus::Fractured
        } else {
            NodeStatus::Stable
        }
    }
}

/// Euclidean distance helper (explicit, no allocations)
pub fn l2_distance(a: &[f32; N], b: &[f32; N]) -> f32 {
    let mut sum = 0.0;
    for i in 0..N {
        let d = a[i] - b[i];
        sum += d * d;
    }
    sum.sqrt()
}

/// Minimal test harness
fn main() {
    let mut a = Node::new([1.0; N], 0.25, 0.01, 10.0);
    let mut b = Node::new([0.5; N], 0.30, 0.01, 10.0);

    let stream = [
        [0.2; N],
        [0.4; N],
        [5.0; N], // perturbation spike
    ];

    for (t, sigma) in stream.iter().enumerate() {
        let a_snap = a.state;
        let b_snap = b.state;

        let sa = a.step(sigma, &b_snap);
        let sb = b.step(sigma, &a_snap);

        println!(
            "t={} | Δab={:.4} | η_a={:.3} η_b={:.3} | drift_a={:.3} drift_b={:.3}",
            t,
            l2_distance(&a.state, &b.state),
            a.eta,
            b.eta,
            a.drift,
            b.drift
        );

        if sa == NodeStatus::Fractured || sb == NodeStatus::Fractured {
            println!("FRACTURE DETECTED — SYSTEM TERMINATED");
            break;
        }
    }
}
/*!
DVSM CORE 3-IN-1 SYSTEM

LAYER 1: Node algebra (state update)
LAYER 2: Network coupling (neighbor interaction)
LAYER 3: Execution harness (multi-step simulation)

No category theory, no stacks, no abstract topology —
only measurable state + interaction + drift.
*/

use std::f32;

const N: usize = 8;
const MAX_NODES: usize = 8;

/* ============================================================
   LAYER 1 — CORE NODE ALGEBRA
   ============================================================ */

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Stable,
    Fractured,
}

#[derive(Clone)]
pub struct Node {
    pub state: [f32; N],
    pub eta: f32,
    pub drift: f32,
    pub drift_budget: f32,
    pub epsilon: f32,
}

impl Node {
    pub fn new(state: [f32; N], eta: f32, eps: f32, budget: f32) -> Self {
        Self {
            state,
            eta,
            drift: 0.0,
            drift_budget: budget,
            epsilon: eps,
        }
    }

    /// Fundamental equation:
    /// S' = (1 - η)S + η(σ + S_neighbor)
    pub fn step(&mut self, sigma: &[f32; N], neighbor: &[f32; N]) -> Status {
        let mut next = [0.0; N];

        for i in 0..N {
            let excitation = sigma[i] + neighbor[i];
            next[i] = (1.0 - self.eta) * self.state[i] + self.eta * excitation;
        }

        let defect = Self::l2(&next, neighbor);

        if defect > self.epsilon {
            self.drift += defect;
            self.eta *= 1.0 - self.eta;
        }

        self.state = next;

        if self.drift > self.drift_budget {
            Status::Fractured
        } else {
            Status::Stable
        }
    }

    fn l2(a: &[f32; N], b: &[f32; N]) -> f32 {
        let mut s = 0.0;
        for i in 0..N {
            let d = a[i] - b[i];
            s += d * d;
        }
        s.sqrt()
    }
}

/* ============================================================
   LAYER 2 — NETWORK TOPOLOGY (NEIGHBOR COUPLING)
   ============================================================ */

pub struct Network {
    pub nodes: Vec<Node>,
}

impl Network {
    pub fn new(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }

    /// Simple ring topology:
    /// each node interacts with next node
    fn neighbor_index(i: usize) -> usize {
        (i + 1) % MAX_NODES
    }

    pub fn step_all(&mut self, sigma: &[f32; N]) -> bool {
        let snapshots: Vec<[f32; N]> =
            self.nodes.iter().map(|n| n.state).collect();

        let mut all_stable = true;

        for i in 0..self.nodes.len() {
            let j = Self::neighbor_index(i);

            let status = self.nodes[i].step(sigma, &snapshots[j]);

            if status == Status::Fractured {
                all_stable = false;
            }
        }

        all_stable
    }
}

/* ============================================================
   LAYER 3 — EXECUTION HARNESS
   ============================================================ */

fn main() {
    let mut net = Network::new(vec![
        Node::new([1.0; N], 0.25, 0.01, 10.0),
        Node::new([0.8; N], 0.30, 0.01, 10.0),
        Node::new([0.6; N], 0.28, 0.01, 10.0),
    ]);

    let signals = [
        [0.2; N],
        [0.4; N],
        [0.6; N],
        [5.0; N], // instability injection
    ];

    println!("DVSM 3-IN-1 CORE RUNNING");

    for (t, sigma) in signals.iter().enumerate() {
        let stable = net.step_all(sigma);

        println!("t={} | stable={}", t, stable);

        if !stable {
            println!("SYSTEM FRACTURE — HALT");
            break;
        }
    }
}

Game Loop
   ├── Input system
   ├── DVSM update layer   ← your system lives here
   │       ├── entity state update
   │       ├── interaction step
   │       └── drift/stability check
   ├── Physics engine
   ├── Rendering
   └── Network sync (optional DVSM mirror)

/*!
DVSM DISTRIBUTED + SIMD CORE

LAYER 1:
- UDP-based distributed node communication
- each node exchanges state packets

LAYER 2:
- SIMD-accelerated state update kernel
- 8-way parallel vector processing (f32x8)

This is a real execution architecture:
- network = message passing graph
- compute = SIMD contraction dynamics
*/

use std::net::UdpSocket;
use std::time::Duration;

use std::arch::x86_64::*;

/* ============================================================
   CONFIG
   ============================================================ */

const LANES: usize = 8; // SIMD width
const PORT: u16 = 9000;

/* ============================================================
   LAYER 1 — SIMD CORE (8-WAY VECTOR STATE UPDATE)
   ============================================================ */

#[derive(Clone, Copy)]
pub struct SimdNode {
    pub state: [f32; LANES],
    pub eta: f32,
}

impl SimdNode {
    pub fn new(state: [f32; LANES], eta: f32) -> Self {
        Self { state, eta }
    }

    /// SIMD core update:
    /// S' = (1-η)S + η(σ + S_neighbor)
    #[target_feature(enable = "avx2")]
    pub unsafe fn step(
        &mut self,
        sigma: &[f32; LANES],
        neighbor: &[f32; LANES],
    ) {
        let eta = self.eta;
        let one_minus_eta = 1.0 - eta;

        let eta_v = _mm256_set1_ps(eta);
        let one_eta_v = _mm256_set1_ps(one_minus_eta);

        let sigma_v = _mm256_loadu_ps(sigma.as_ptr());
        let neigh_v = _mm256_loadu_ps(neighbor.as_ptr());
        let state_v = _mm256_loadu_ps(self.state.as_ptr());

        // excitation = sigma + neighbor
        let exc_v = _mm256_add_ps(sigma_v, neigh_v);

        // weighted update
        let term1 = _mm256_mul_ps(one_eta_v, state_v);
        let term2 = _mm256_mul_ps(eta_v, exc_v);

        let result = _mm256_add_ps(term1, term2);

        _mm256_storeu_ps(self.state.as_mut_ptr(), result);
    }
}

/* ============================================================
   LAYER 2 — DISTRIBUTED NODE (UDP MESSAGE PASSING)
   ============================================================ */

pub struct DistributedNode {
    pub simd: SimdNode,
    pub id: u32,
    pub peer_addr: String,
    pub socket: UdpSocket,
}

impl DistributedNode {
    pub fn new(id: u32, bind: &str, peer_addr: &str, eta: f32) -> Self {
        let socket = UdpSocket::bind(bind).expect("bind failed");
        socket
            .set_read_timeout(Some(Duration::from_millis(5)))
            .ok();

        Self {
            simd: SimdNode::new([0.0; LANES], eta),
            id,
            peer_addr: peer_addr.to_string(),
            socket,
        }
    }

    /// Serialize state → UDP packet
    fn send_state(&self) {
        let mut buf = [0u8; 32];

        for i in 0..LANES {
            let bytes = self.simd.state[i].to_le_bytes();
            buf[i * 4..i * 4 + 4].copy_from_slice(&bytes);
        }

        let _ = self.socket.send_to(&buf, &self.peer_addr);
    }

    /// Receive neighbor state
    fn recv_state(&self) -> Option<[f32; LANES]> {
        let mut buf = [0u8; 32];

        match self.socket.recv_from(&mut buf) {
            Ok(_) => {
                let mut out = [0.0; LANES];
                for i in 0..LANES {
                    let mut bytes = [0u8; 4];
                    bytes.copy_from_slice(&buf[i * 4..i * 4 + 4]);
                    out[i] = f32::from_le_bytes(bytes);
                }
                Some(out)
            }
            Err(_) => None,
        }
    }

    /// Full distributed + SIMD step
    pub fn tick(&mut self, sigma: &[f32; LANES]) {
        // 1. get neighbor state (or fallback to self)
        let neighbor = self.recv_state().unwrap_or(self.simd.state);

        // 2. SIMD update
        unsafe {
            self.simd.step(sigma, &neighbor);
        }

        // 3. broadcast updated state
        self.send_state();
    }
}

/* ============================================================
   DEMO MAIN (2 NODE SYSTEM)
   ============================================================ */

fn main() {
    let mut node_a = DistributedNode::new(
        1,
        "127.0.0.1:9000",
        "127.0.0.1:9001",
        0.25,
    );

    let mut node_b = DistributedNode::new(
        2,
        "127.0.0.1:9001",
        "127.0.0.1:9000",
        0.30,
    );

    let sigma: [f32; LANES] = [0.5; LANES];

    loop {
        node_a.tick(&sigma);
        node_b.tick(&sigma);

        println!(
            "A: {:?}\nB: {:?}\n---",
            node_a.simd.state,
            node_b.simd.state
        );
    }
}
