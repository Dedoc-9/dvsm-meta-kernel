// ============================================================================
// DVSM — HARDENED EXTENDED KERNEL (v2.0)
// Author: Daniel J. Dillberg
// Includes:
//   - Arithmetic Model A (explicit execution semantics)
//   - Generative vs Epistemic Sigma split
//   - Projection Algebra (O closure, causal isolation)
//   - Contraction kernel F_A
//   - Stability / perturbation semantics
//   - Replay as quotient (epistemic only)
// ============================================================================

use std::marker::PhantomData;

// ============================================================================
// 1. ARITHMETIC EXECUTION MODEL (A)
// ============================================================================

/// A: Arithmetic model (execution semantics, not math abstraction)
#[derive(Clone)]
pub struct ArithmeticModel {
    pub epsilon: f64, // numerical tolerance
}

impl ArithmeticModel {
    #[inline]
    pub fn eq(&self, a: f64, b: f64) -> bool {
        (a - b).abs() <= self.epsilon
    }

    #[inline]
    pub fn norm_diff(&self, a: f64, b: f64) -> f64 {
        (a - b).abs()
    }
}

// ============================================================================
// 2. CORE TYPES
// ============================================================================

pub type Signal = f64;
pub type State = f64;

// ============================================================================
// 3. GENERATIVE SIGMA (CAUSAL)
// ============================================================================

pub trait GenerativeSigma {}

pub trait Sigma {
    fn next(&mut self) -> Option<Signal>;
}

// marker: generative validity
pub trait SigmaLaw: GenerativeSigma {}

// ============================================================================
// 4. EPISTEMIC SIGMA (REPLAY ONLY)
// ============================================================================

pub trait EpistemicSigma {}

pub struct ReplaySigma {
    trace: Vec<Signal>,
    idx: usize,
}

impl ReplaySigma {
    pub fn new(trace: Vec<Signal>) -> Self {
        Self { trace, idx: 0 }
    }

    pub fn next(&mut self) -> Option<Signal> {
        if self.idx >= self.trace.len() {
            return None;
        }
        let v = self.trace[self.idx];
        self.idx += 1;
        Some(v)
    }
}

impl EpistemicSigma for ReplaySigma {}

// ============================================================================
// 5. SIGMA IMPLEMENTATIONS (GENERATIONAL ONLY)
// ============================================================================

pub struct StaticSigma<const N: usize> {
    pub data: [Signal; N],
    pub index: usize,
}

impl<const N: usize> GenerativeSigma for StaticSigma<N> {}
impl<const N: usize> Sigma for StaticSigma<N> {
    fn next(&mut self) -> Option<Signal> {
        if self.index >= N {
            return None;
        }
        let v = self.data[self.index];
        self.index += 1;
        Some(v)
    }
}

impl<const N: usize> SigmaLaw for StaticSigma<N> {}

// ------------------------------------------------------------------------

pub struct IterSigma {
    state: u64,
    seed: u64,
    limit: u64,
}

impl IterSigma {
    pub fn new(seed: u64, limit: u64) -> Self {
        Self { state: seed, seed, limit }
    }
}

impl GenerativeSigma for IterSigma {}
impl Sigma for IterSigma {
    fn next(&mut self) -> Option<Signal> {
        self.state = self.state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);

        Some((self.state % self.limit) as Signal / self.limit as Signal)
    }
}

impl SigmaLaw for IterSigma {}

// ============================================================================
// 6. DVSM KERNEL (F_A)
// ============================================================================

pub struct DVSMKernel {
    pub w: State,
    pub eta: State,
}

impl DVSMKernel {
    pub fn new(w: State, eta: State) -> Self {
        Self { w, eta }
    }

    #[inline]
    pub fn step(&mut self, sigma: Signal, A: &ArithmeticModel) -> State {
        // contraction update (arithmetic-relative)
        self.w = self.w + self.eta * (sigma - self.w);

        // optional stabilization clamp (A-relative semantics)
        if !A.eq(self.w, self.w) {
            self.w = 0.0;
        }

        self.w
    }
}

// ============================================================================
// 7. PROJECTION ALGEBRA (O)
// ============================================================================

#[derive(Clone)]
pub struct Observation {
    pub x_hat: State,
    pub error: State,
}

/// O: projection functor (causally inert)
pub struct ObservationFunctor;

impl ObservationFunctor {
    pub fn project(state: State, sigma: Signal) -> Observation {
        Observation {
            x_hat: state,
            error: sigma - state,
        }
    }
}

// ============================================================================
// 8. DVSM SYSTEM (CAUSAL CORE)
// ============================================================================

pub struct DVSMRuntime<S: Sigma> {
    sigma: S,
    kernel: DVSMKernel,
    arith: ArithmeticModel,
    _phantom: PhantomData<S>,
}

impl<S: Sigma> DVSMRuntime<S> {
    pub fn new(sigma: S, kernel: DVSMKernel, arith: ArithmeticModel) -> Self {
        Self {
            sigma,
            kernel,
            arith,
            _phantom: PhantomData,
        }
    }

    // single causal step
    pub fn step(&mut self) -> Option<(State, Observation)> {
        let sigma_t = self.sigma.next()?;

        // projection (epistemic, inert)
        let obs = ObservationFunctor::project(self.kernel.w, sigma_t);

        // causal update
        let s_next = self.kernel.step(sigma_t, &self.arith);

        Some((s_next, obs))
    }

    pub fn run(&mut self, steps: usize) -> Vec<State> {
        let mut out = Vec::with_capacity(steps);

        for _ in 0..steps {
            match self.step() {
                Some((s, _)) => out.push(s),
                None => break,
            }
        }

        out
    }
}

// ============================================================================
// 9. STABILITY LAW (A-RELATIVE)
// ============================================================================

pub fn perturbation_bound(
    a: &ArithmeticModel,
    s1: &[State],
    s2: &[State],
) -> State {
    let mut acc = 0.0;

    for (x, y) in s1.iter().zip(s2.iter()) {
        acc += a.norm_diff(*x, *y);
    }

    acc / (s1.len() as f64)
}

// ============================================================================
// 10. REPLAY QUOTIENT CHECK (EPISTEMIC ONLY)
// ============================================================================

pub fn replay_equivalent(
    a: &ArithmeticModel,
    t1: &[State],
    t2: &[State],
) -> bool {
    if t1.len() != t2.len() {
        return false;
    }

    for (x, y) in t1.iter().zip(t2.iter()) {
        if !a.eq(*x, *y) {
            return false;
        }
    }

    true
}

// ============================================================================
// 11. EXAMPLE MAIN
// ============================================================================

fn main() {
    let sigma = IterSigma::new(42, 100);

    let kernel = DVSMKernel::new(0.0, 0.1);

    let arith = ArithmeticModel { epsilon: 1e-9 };

    let mut system = DVSMRuntime::new(sigma, kernel, arith.clone());

    let trajectory = system.run(10);

    println!("trajectory = {:?}", trajectory);
}
// ============================================================================
// DVSM — DEVELOPER NOTE BLOCK (EVOLUTION TRACE v1 → v2 HARDENED)
// ============================================================================
//
// This document describes how the DVSM architecture evolved from:
//
//   (1) informal contraction system
//   (2) layered epistemic architecture
//   (3) partitioned causal/epistemic system
//   (4) arithmetic-relative deterministic kernel
//   (5) projection-algebra closed system (current form)
//
// It is NOT executable and does NOT affect runtime semantics.
//
// ============================================================================
//
// 1. INITIAL STATE: PURE CONTRACTION KERNEL
// ============================================================================
//
// Original model:
//
//     w_{t+1} = w_t + η(σ_t - w_t)
//
// Characteristics:
//   - single-state dynamical system
//   - σ treated implicitly (external but undefined)
//   - no formal separation of epistemics
//
// Failure mode:
//   - σ ambiguity (generator vs observation conflation)
//   - replay interpreted as causal reverse process
//
// Key limitation:
//   No explicit boundary between:
//     - generation
//     - execution
//     - observation
//
// ============================================================================
//
// 2. LAYERED EPISTEMIC EXPANSION (ARCHITECTURAL OVERLOAD PHASE)
// ============================================================================
//
// Introduced concepts:
//
//   - O (observation layer)
//   - L (loss / evaluation layer)
//   - Replay systems
//   - Diagnostics / hashing / traces
//
// Resulting structure:
//
//   T (dynamics)
//   O (observation)
//   L (evaluation)
//
// Issue:
//   Epistemic components were implicitly treated as semi-causal.
//
// Failure mode:
//   - accidental feedback interpretation
//   - “observer contamination” of system semantics
//   - unclear ontology of traces vs states
//
// Core problem:
//   Epistemic objects behaved like parallel system, not projection.
//
// ============================================================================
//
// 3. CAUSAL / EPISTEMIC PARTITION (FIRST STABLE FORM)
// ============================================================================
//
// Introduced strict separation:
//
//   Σ → σ_t → F → S_t
//   E = projection only
//
// Additions:
//
//   - Σ declared exogenous
//   - Replay declared non-causal
//   - Hashing removed from causal interpretation
//
// Improvement:
//   - prevented backward coupling
//
// Remaining issue:
//   - arithmetic execution model still implicit
//   - determinism treated as abstract property
//
// ============================================================================
//
// 4. ARITHMETIC RELATIVITY LAYER (MAJOR CORRECTION)
// ============================================================================
//
// Key realization:
//
//   Determinism is not absolute.
//   It is relative to execution semantics.
//
// Introduced:
//
//   A = (precision, rounding, evaluation order)
//
// Revised axiom:
//
//   (S₀, Σ, F, A) → trajectory
//
// Impact:
//   - removed false assumption of symbolic determinism
//   - aligned model with real hardware execution
//
// Critical correction:
//   Floating-point + compiler behavior becomes part of system identity.
//
// ============================================================================
//
// 5. Σ DUALITY SPLIT (GENERATION vs RECONSTRUCTION)
// ============================================================================
//
// Introduced distinction:
//
//   Σ_gen  = causal signal generator
//   Σ_epi  = replay / reconstruction operator
//
// Constraint:
//
//   Σ_gen ∩ Σ_epi = ∅
//
// Meaning:
//   - replay is not inversion
//   - reconstruction is epistemic approximation only
//
// Effect:
//   - eliminated “trace-as-cause” interpretation
//
// ============================================================================
//
// 6. PROJECTION ALGEBRA FORMALIZATION
// ============================================================================
//
// Observation reframed as:
//
//   O : S → E
//   E = ℱ(S₀:T)
//
// Key shift:
//   - O is not a layer
//   - O is a morphism
//   - E is a functional space over trajectories
//
// Closure rule:
//
//   O_i ∘ O_j ∈ 𝒪*
//
// BUT:
//
//   𝒪* ∩ causal domain = ∅
//
// Meaning:
//   Observations compose freely but remain causally inert.
//
// ============================================================================
//
// 7. STABILITY + PERTURBATION MODEL INTRODUCTION
// ============================================================================
//
// Added missing engineering constraint:
//
//   DVSM is not exact-state invariant.
//   It is bounded-stability invariant.
//
// Introduced:
//
//   ||S_t - S'_t|| ≤ K(ε, δ, η)
//
// Meaning:
//   - determinism is trajectory equivalence under bounded error
//
// Effect:
//   - replaced brittle equality notion with robust equivalence
//
// ============================================================================
//
// 8. REPLAY AS QUOTIENT SPACE (FINAL CORRECTION)
// ============================================================================
//
// Final correction:
//
//   Replay is not identity checking.
//   Replay is equivalence class membership.
//
// Defined:
//
//   Traj / ~
//
// where:
//
//   Σ₁ ~ Σ₂ ⇔ trajectory equivalence under A
//
// Effect:
//   - removes ontological interpretation of replay
//   - formalizes observational equivalence
//
// ============================================================================
//
// 9. CURRENT STABLE FORM (HARDENED DVSM)
// ============================================================================
//
// DVSM := (S, Σ_gen, F_A, 𝒪, A, 𝒱)
//
// Where:
//
//   S       = state space
//   Σ_gen   = generative signal functor
//   F_A     = arithmetic-relative contraction operator
//   𝒪       = projection algebra (epistemic only)
//   A       = execution semantics (hardware reality layer)
//   𝒱       = causal firewall (no feedback constraints)
//
// ============================================================================
//
// 10. CORE ARCHITECTURAL INSIGHT (WHAT ACTUALLY CHANGED)
// ============================================================================
//
// The evolution was NOT about adding components.
//
// It was about progressively removing incorrect implicit assumptions:
//
//   (1) removed implicit epistemic causality
//   (2) removed assumption of absolute determinism
//   (3) removed inversion interpretation of replay
//   (4) removed abstraction of arithmetic execution
//   (5) replaced “layers” with morphisms and constraints
//
// ============================================================================
//
// FINAL RESULT:
//
// DVSM is now:
//
//   a causally-closed, arithmetic-relative,
//   projection-stable contraction system
//
// NOT:
//
//   a learning system
//   a probabilistic model
//   a bidirectional inference engine
//   a layered cognitive architecture
//
// ============================================================================
// END DEVELOPER EVOLUTION NOTE
// ============================================================================//
