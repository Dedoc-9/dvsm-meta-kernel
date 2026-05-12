// ============================================================================
// DVSM RUNTIME KERNEL (FINAL STABILIZED FORM)
// Unified Stochastic State-Space Execution System
// Author: Daniel J. Dillberg
// ============================================================================
//
// REAL-WORLD PURPOSE:
//   This system is a minimal execution kernel for adaptive stochastic control,
//   signal estimation, and feedback stabilization systems (e.g., ANC, DSP,
//   robotics, streaming inference).
//
//   It is designed to:
//     - represent real-time adaptive filters (LMS / SGD family)
//     - unify bounded-memory and multi-agent control systems
//     - provide a strict separation between:
//         (F) dynamics, (O) observation, (L) evaluation
//
//   It is NOT:
//     - a compression engine
//     - a representation learning system
//     - a quotient or ontology system
//
// ============================================================================

use std::marker::PhantomData;

// ============================================================================
// 1. TIME BASE
// ============================================================================

pub type Time = usize;

// ============================================================================
// 2. CORE CONSTRAINT SYSTEM (NOW BOUND TO DVSM)
// ============================================================================

pub trait Constraint {}

pub struct ANC;
pub struct Compression;
pub struct BoundedMemory;
pub struct MultiAgent;

impl Constraint for ANC {}
impl Constraint for Compression {}
impl Constraint for BoundedMemory {}
impl Constraint for MultiAgent {}

// ============================================================================
// 3. GHOST SAFETY (TYPE-ENFORCED INVARIANT)
// ============================================================================

pub trait GhostSafe {
    const VALID: bool;
}

// ============================================================================
// 4. DVSM CORE TRAIT (F, O, L CONTRACT)
// ============================================================================

pub trait DVSMSystem: GhostSafe {
    type State;
    type Input;
    type Observation;
    type Target;

    type Constraint: Constraint;

    fn transition(&mut self, input: Self::Input);     // F
    fn observe(&self) -> Self::Observation;           // O
    fn loss(&self, obs: &Self::Observation, target: &Self::Target) -> f64; // L
}

// ============================================================================
// 5. RUNTIME ENGINE
// ============================================================================

pub struct DVSMRuntime<S: DVSMSystem> {
    pub system: S,
    pub target: S::Target,
    pub t: Time,
}

impl<S: DVSMSystem> DVSMRuntime<S> {

    pub fn new(system: S, target: S::Target) -> Self {
        Self { system, target, t: 0 }
    }

    pub fn assert_valid(&self) {
        assert!(S::VALID, "DVSM Ghost Rule Violation");
    }

    pub fn tick(&mut self, input: S::Input) -> Tick<S::Observation> {

        self.assert_valid();

        // F: state evolution
        self.system.transition(input);

        // O: observation projection
        let obs = self.system.observe();

        // L: evaluation (non-causal)
        let loss = self.system.loss(&obs, &self.target);

        self.t += 1;

        Tick { obs, loss, t: self.t }
    }
}

// ============================================================================
// 6. OUTPUT STRUCT
// ============================================================================

pub struct Tick<O> {
    pub obs: O,
    pub loss: f64,
    pub t: Time,
}

// ============================================================================
// 7. ANC SYSTEM (ADAPTIVE LMS CONTROL)
// ============================================================================

pub struct ANCSystem {
    pub w: f64,
    pub eta: f64,
}

impl GhostSafe for ANCSystem {
    const VALID: bool = true;
}

impl DVSMSystem for ANCSystem {

    type State = f64;
    type Input = f64;
    type Observation = f64;
    type Target = f64;

    type Constraint = ANC;

    fn transition(&mut self, input: f64) {
        let e = input - self.w;
        self.w += self.eta * e;
    }

    fn observe(&self) -> f64 {
        self.w
    }

    fn loss(&self, obs: &f64, target: &f64) -> f64 {
        (obs - target).powi(2)
    }
}

// ============================================================================
// 8. BOUNDED MEMORY SYSTEM
// ============================================================================

pub struct BoundedMemorySystem {
    pub state: Vec<f64>,
    pub cap: usize,
}

impl GhostSafe for BoundedMemorySystem {
    const VALID: bool = true;
}

impl DVSMSystem for BoundedMemorySystem {

    type State = Vec<f64>;
    type Input = f64;
    type Observation = f64;
    type Target = f64;

    type Constraint = BoundedMemory;

    fn transition(&mut self, input: f64) {
        self.state.push(input);
        if self.state.len() > self.cap {
            self.state.remove(0);
        }
    }

    fn observe(&self) -> f64 {
        self.state.iter().sum::<f64>() / self.state.len().max(1) as f64
    }

    fn loss(&self, obs: &f64, target: &f64) -> f64 {
        (obs - target).powi(2)
    }
}

// ============================================================================
// 9. COMPRESSION SYSTEM (CORRECTED: VALID STATE-SPACE MODEL)
// ============================================================================
//
// Compression is NOT a system type.
// It is a constraint on observation structure.
// But must still respect state evolution.
//
// ============================================================================

pub struct CompressionSystem {
    pub state: f64,
}

impl GhostSafe for CompressionSystem {
    const VALID: bool = true;
}

impl DVSMSystem for CompressionSystem {

    type State = f64;
    type Input = f64;
    type Observation = f64;
    type Target = f64;

    type Constraint = Compression;

    fn transition(&mut self, input: f64) {
        // valid dynamics (not degenerate)
        self.state = 0.85 * self.state + 0.15 * input;
    }

    fn observe(&self) -> f64 {
        // lossy projection (compression behavior)
        self.state.tanh()
    }

    fn loss(&self, obs: &f64, target: &f64) -> f64 {
        (obs - target).abs()
    }
}

// ============================================================================
// 10. MULTI-AGENT SYSTEM
// ============================================================================

pub struct MultiAgentSystem {
    pub a: f64,
    pub b: f64,
}

impl GhostSafe for MultiAgentSystem {
    const VALID: bool = true;
}

impl DVSMSystem for MultiAgentSystem {

    type State = (f64, f64);
    type Input = (f64, f64);
    type Observation = (f64, f64);
    type Target = (f64, f64);

    type Constraint = MultiAgent;

    fn transition(&mut self, input: (f64, f64)) {
        self.a += 0.1 * (input.0 - self.b);
        self.b += 0.1 * (input.1 - self.a);
    }

    fn observe(&self) -> (f64, f64) {
        (self.a, self.b)
    }

    fn loss(&self, obs: &(f64, f64), target: &(f64, f64)) -> f64 {
        (obs.0 - target.0).powi(2) + (obs.1 - target.1).powi(2)
    }
}

// ============================================================================
// 11. COMPILER INTERFACE (META SYSTEM ONLY)
// ============================================================================

pub trait DVSMCompiler {
    type Spec;
    type System: DVSMSystem;

    fn compile(spec: Self::Spec) -> Self::System;
}

// ============================================================================
// 12. CORE INVARIANT STATEMENT
// ============================================================================
//
// SYSTEM AXIOM:
//   Only (S, F, O, L) are causal.
//
// CONSTRAINTS:
//   Only define valid instantiations of DVSMSystem.
//
// GHOST RULE:
//   Anything not expressed in S, F, O, L has no runtime effect.
//
// ============================================================================

// ============================================================================
// 13. DEMO EXECUTION
// ============================================================================

fn main() {

    let mut anc = ANCSystem { w: 0.0, eta: 0.1 };
    let mut runtime = DVSMRuntime::new(anc, 1.0);

    for _ in 0..5 {
        let tick = runtime.tick(1.0);
        println!("ANC -> t:{} loss:{:.4}", tick.t, tick.loss);
    }

    let mut mem = BoundedMemorySystem {
        state: vec![],
        cap: 3,
    };

    let mut runtime2 = DVSMRuntime::new(mem, 1.0);

    for i in 0..5 {
        let tick = runtime2.tick(i as f64);
        println!("MEM -> t:{} loss:{:.4}", tick.t, tick.loss);
    }
}
// ============================================================================
// DVSM BOUNDED STATE MACHINE (v1.1)
// Constraint-Restricted Stochastic Contraction Runtime Layer
// ============================================================================
//
// ROLE:
//   This module defines a bounded execution regime over DVSM Operator T.
//
//   It does NOT define a new system type.
//   It constrains execution geometry of an existing contraction operator.
//
//   Formal interpretation:
//     w_{t+1} = Π_B( T(w_t, x_t) )
//
//   where Π_B is projection onto a compact convex set B.
//
// ============================================================================

use crate::Operator;
use std::collections::VecDeque;

// ============================================================================
// 1. BOUNDED EXECUTION WRAPPER
// ============================================================================

pub struct BoundedDVSM<T>
where
    T: Operator<State = f64, Input = f64>,
{
    /// underlying contraction operator (T)
    pub system: T,

    /// compact state bound (defines convex projection set B = [-bound, bound])
    pub bound: f64,

    /// diagnostic trace space (NOT part of system state)
    pub history: VecDeque<f64>,

    /// finite memory horizon for diagnostics only
    pub cap: usize,
}

impl<T> BoundedDVSM<T>
where
    T: Operator<State = f64, Input = f64>,
{
    pub fn new(system: T, bound: f64, cap: usize) -> Self {
        Self {
            system,
            bound,
            history: VecDeque::with_capacity(cap),
            cap,
        }
    }

    // ========================================================================
    // 2. PROJECTION OPERATOR Π_B (GEOMETRIC CONSTRAINT)
    // ========================================================================
    //
    // Interpretation:
    //   Π_B(x) = projection onto convex compact interval [-bound, bound]
    //
    // This preserves contraction structure under bounded perturbation.
    // ========================================================================

    #[inline]
    fn project(&self, state: f64) -> f64 {
        if state > self.bound {
            self.bound
        } else if state < -self.bound {
            -self.bound
        } else {
            state
        }
    }

    // ========================================================================
    // 3. EXECUTION STEP (CONSTRAINED OPERATOR APPLICATION)
    // ========================================================================

    pub fn step(&mut self, input: f64) -> f64 {
        // raw contraction update
        let mut state = self.system.step(input);

        // geometric projection (bounded manifold constraint)
        state = self.project(state);

        // diagnostic trace update (non-causal layer)
        self.history.push_back(state);
        if self.history.len() > self.cap {
            self.history.pop_front();
        }

        state
    }

    // ========================================================================
    // 4. DIAGNOSTIC STABILITY METRIC (NON-CAUSAL)
    // ========================================================================

    pub fn variance_estimate(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }

        let mean = self.history.iter().sum::<f64>() / self.history.len() as f64;

        self.history
            .iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>()
            / self.history.len() as f64
    }
}

// ============================================================================
// 5. BOUNDED OPERATOR CONTRACT (TYPE DISCIPLINE ONLY)
// ============================================================================
//
// NOTE:
// This is NOT a runtime constraint.
// It is a compile-time semantic marker for systems compatible with Π_B.
//
// ============================================================================

pub trait BoundedOperator {
    fn is_within_bounds(&self, bound: f64) -> bool;
}

// ============================================================================
// 6. INTERPRETATION LAYER (STRICTLY NON-CAUSAL)
// ============================================================================
//
// This module implements:
//
//   bounded stochastic contraction systems under convex projection.
//
// It does NOT modify:
//   - operator dynamics T
//   - observation O
//   - loss L
//
// It ONLY modifies:
//   - admissible state manifold geometry
//
// ============================================================================
