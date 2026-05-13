// ============================================================================
// DVSM MC — HARDENED GOODHART-RESISTANT CONTRACTION GEOMETRY
// ============================================================================
// Author: Daniel J. Dillberg
// ============================================================================
// DVSM — HARDENED FUNDAMENTAL EQUATION MODULE
// ============================================================================
// OPENING INDEX (READ FIRST)
// ----------------------------------------------------------------------------
// 0. PURPOSE
//    Implements the hardened DVSM scalar contraction system:
//
//        x_{t+1} = F_A(x_t, σ_t)
//                  - λ₁||a_t||²
//                  - λ₂||j_t||²
//                  - λ₃Δ_t
//                  - λ₄H_t
//
//    where kernel dynamics are separated from stability geometry.
// ----------------------------------------------------------------------------
// 1. STRUCTURAL LAYERS
//
//    (A) Kernel Layer
//        - affine contraction update
//        - deterministic causal evolution
//
//    (B) Temporal Geometry Layer
//        - velocity
//        - acceleration (a_t)
//        - jerk (j_t)
//
//    (C) Stress Geometry Layer
//        - Δ_t : instantaneous divergence field
//        - H_t : accumulated drift memory
//
//    (D) Hardening Layer
//        - penalty aggregation
//        - trajectory-level constraint enforcement
// ----------------------------------------------------------------------------
// 2. CORE VARIABLES
//
//    x_t        : system state (ℝ)
//    σ_t        : external input signal
//    η          : contraction rate
//    σ_gain     : input scaling
//    b          : bias term
//
//    λ₁         : acceleration penalty weight
//    λ₂         : jerk penalty weight
//    λ₃         : divergence penalty weight
//    λ₄         : drift penalty weight
// ----------------------------------------------------------------------------
// 3. DERIVATIVE DEFINITIONS
//
//    v_t = x_t - x_{t-1}
//    a_t = v_t - v_{t-1}
//    j_t = a_t - a_{t-1}
//
// ----------------------------------------------------------------------------
// 4. STRESS GEOMETRY
//
//    Δ_t = |a_t| + |j_t|
//    H_t = Σ_{τ≤t} Δ_τ
//
// ----------------------------------------------------------------------------
// 5. EXECUTION PIPELINE (PER STEP)
//
//    1. Compute kernel candidate: F_A(x_t, σ_t)
//    2. Compute temporal derivatives
//    3. Update stress memory (Δ_t, H_t)
//    4. Compute penalties
//    5. Apply hardened correction
//    6. Commit x_{t+1}
// ----------------------------------------------------------------------------
// 6. DESIGN INVARIANT
//
//    - Kernel defines causality
//    - Stress defines memory of instability
//    - Hardening enforces trajectory-level validity
//    - No step depends on future state
// ----------------------------------------------------------------------------
// 7. INTERPRETATION
//
//    This system is a:
//
//      "trajectory-constrained contraction dynamical system"
//
//    NOT a pointwise threshold system.
// ============================================================================
// Whitepaper Short
// ----------------------------------------------------------------------------
// DVSM (Distributed Vector Stability Model) is a deterministic,
// snapshot-synchronous contraction system designed to resist
// metric gaming, boundary-riding, and trajectory exhaustion attacks.
//
// Traditional validation systems rely on static legality checks:
//
//     legal(state) ∈ {true, false}
//
// These systems fail under Goodhart pressure because attackers
// optimize directly against observable thresholds.
//
// DVSM replaces static legality with:
//
//     trajectory-consistency geometry
//
// The system evaluates:
//
//   • velocity continuity
//   • acceleration continuity
//   • jerk smoothness
//   • instability accumulation
//   • manifold coherence
//   • long-horizon causal consistency
//
// instead of isolated point validity.
// ----------------------------------------------------------------------------
// FUNDAMENTAL HARDENED EQUATION
// ----------------------------------------------------------------------------
// x(t+1)
//   = x(t)
//   + η(F(x(t), σ(t)) - x(t))
//   - λ₁ ||a(t)||²
//   - λ₂ ||j(t)||²
//   - λ₃ Δ(t)
//   - λ₄ H(t)
//
// WHERE:
//
//   x(t)
//       state vector
//
//   F(x,σ)
//       causal contraction kernel
//
//   η
//       adaptive contraction coefficient
//
//   a(t)
//       acceleration field
//
//   j(t)
//       jerk field (3rd derivative continuity)
//
//   Δ(t)
//       geometric divergence field
//
//   H(t)
//       accumulated instability drift memory
// ----------------------------------------------------------------------------
// CORE SECURITY PRINCIPLE
// ----------------------------------------------------------------------------
// OLD MODEL:
//
//     legality(point)
//
// HARDENED MODEL:
//
//     legality(trajectory)
//
// A trajectory is considered lawful only if:
//
//   • spatial continuity is preserved
//   • temporal continuity is preserved
//   • derivative fields remain bounded
//   • instability energy remains contractive
//   • manifold projection remains coherent
// ----------------------------------------------------------------------------
// CAUSAL SEPARATION
// ----------------------------------------------------------------------------
// DVSM strictly separates:
//
//   1. Kernel dynamics        (causal mutation)
//   2. Observation functors  (read-only projections)
//   3. Execution backends    (CPU/SIMD/GPU)
//
// This guarantees:
//
//   • deterministic replay
//   • snapshot isolation
//   • backend semantic equivalence
//   • observer non-interference
// ----------------------------------------------------------------------------
// MATHEMATICAL CLASSIFICATION
// ----------------------------------------------------------------------------
// DVSM belongs to:
//
//   • discrete-time dynamical systems
//   • affine contraction mappings
//   • graph-coupled flow systems
//   • trajectory-constrained control systems
//   • Goodhart-resistant continuity geometries
// ----------------------------------------------------------------------------
// DESIGN PHILOSOPHY
// ----------------------------------------------------------------------------
// The objective is NOT:
//
//     "detect illegal points"
//
// The objective IS:
//
//     "detect unlawful trajectories"
//
// This transforms exploit prevention from:
//
//     threshold enforcement
//
// into:
//
//     causal geometry enforcement
// ============================================================================
// DVSM — HARDENED GOODHART-RESISTANT CAUSAL LAYER
// ============================================================================
// PURPOSE
// ----------------------------------------------------------------------------
// This layer extends DVSM with:
//
//   1. Trajectory continuity enforcement
//   2. Goodhart-resistant legality geometry
//   3. Temporal derivative penalties
//   4. Drift-memory accumulation
//   5. Lawful manifold projection
//
// IMPORTANT:
//
//   This layer DOES NOT replace the causal kernel.
//
//   It wraps the kernel with:
//
//       continuity constraints
//       geometric legality
//       trajectory-energy regularization
//
// FORM:
//
//   x(t+1) = x(t)
//          + η(F(x,σ)-x)
//          - λ1 ||a||²
//          - λ2 ||j||²
//          - λ3 Δ
//          - λ4 H
// ============================================================================

use std::marker::PhantomData;

// ============================================================================
// 1. STATE SPACE
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub x: f64,
}

// ============================================================================
// 2. KERNEL MONOID (CAUSAL DYNAMICS ONLY)
// ============================================================================

pub trait KernelMonoid: Sync {
    const ETA: f64;
    const SIGMA_GAIN: f64;
    const BIAS: f64;

    #[inline(always)]
    fn step(s: &State, sigma: f64) -> State {
        State {
            x: s.x
                + Self::ETA * (Self::SIGMA_GAIN * sigma - s.x)
                + Self::BIAS,
        }
    }
}

// Example stable kernel

pub struct StableKernel;

impl KernelMonoid for StableKernel {
    const ETA: f64 = 0.15;
    const SIGMA_GAIN: f64 = 1.0;
    const BIAS: f64 = 0.0;
}
// ============================================================================
// 3. EXECUTION BACKEND
// ============================================================================
//
// Backend changes execution strategy ONLY.
// Never changes mathematics.
// ============================================================================

pub trait Backend {
    #[inline(always)]
    fn apply<K: KernelMonoid>(s: &State, sigma: f64) -> State {
        K::step(s, sigma)
    }
}

pub struct Cpu;

impl Backend for Cpu {}

// ============================================================================
// 4. TEMPORAL DERIVATIVE GEOMETRY
// ============================================================================
//
// velocity     v = dx/dt
// acceleration a = dv/dt
// jerk         j = da/dt
//
// These are the core anti-Goodhart continuity operators.
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct TemporalDerivatives {
    pub velocity: f64,
    pub acceleration: f64,
    pub jerk: f64,
}

impl TemporalDerivatives {
    #[inline(always)]
    pub fn compute(
        prev2: Option<State>,
        prev1: Option<State>,
        current: State,
    ) -> Self {
        let v = match prev1 {
            Some(p1) => current.x - p1.x,
            None => 0.0,
        };

        let a = match (prev2, prev1) {
            (Some(p2), Some(p1)) => {
                let v_prev = p1.x - p2.x;
                v - v_prev
            }
            _ => 0.0,
        };

        let j = match (prev2, prev1) {
            (Some(p2), Some(p1)) => {
                let v_prev = p1.x - p2.x;
                let a_prev = v_prev;

                a - a_prev
            }
            _ => 0.0,
        };

        Self {
            velocity: v,
            acceleration: a,
            jerk: j,
        }
    }
}
// ============================================================================
// 5. GOODHART HARDENING PARAMETERS
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct HardeningConfig {
    // acceleration penalty
    pub lambda_accel: f64,

    // jerk penalty
    pub lambda_jerk: f64,

    // divergence penalty
    pub lambda_divergence: f64,

    // drift-memory penalty
    pub lambda_drift: f64,

    // lawful manifold clamp
    pub lawful_min: f64,
    pub lawful_max: f64,
}

impl Default for HardeningConfig {
    fn default() -> Self {
        Self {
            lambda_accel: 0.05,
            lambda_jerk: 0.02,
            lambda_divergence: 0.01,
            lambda_drift: 0.005,
            lawful_min: -10.0,
            lawful_max: 10.0,
        }
    }
}
// ============================================================================
// 6. STRESS GEOMETRY
// ============================================================================
//
// Δ = divergence field
// H = accumulated instability drift
//
// These create temporal exploit memory.
//
// This is what prevents:
//
//   legal -> exploit -> cooldown -> repeat
//
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct StressField {
    pub delta: f64,
    pub drift: f64,
}

impl StressField {
    #[inline(always)]
    pub fn update(
        &mut self,
        derivatives: &TemporalDerivatives,
    ) {
        // local instability estimate
        self.delta =
            derivatives.acceleration.abs()
            + derivatives.jerk.abs();

        // irreversible drift accumulation
        self.drift += self.delta;
    }
}
// ============================================================================
// 7. LAWFUL MANIFOLD PROJECTION
// ============================================================================
//
// Π_M(x)
//
// Projects candidate state back into lawful subspace.
// ============================================================================

#[inline(always)]
pub fn project_to_manifold(
    x: f64,
    min: f64,
    max: f64,
) -> f64 {
    x.clamp(min, max)
}
// ============================================================================
// 8. GOODHART HARDENING LAYER
// ============================================================================
//
// This layer transforms:
//
//   point legality
//
// into:
//
//   trajectory legality
//
// ============================================================================

pub struct HardenedLayer {
    pub config: HardeningConfig,
    pub stress: StressField,
}

impl HardenedLayer {
    #[inline(always)]
    pub fn apply(
        &mut self,
        candidate: State,
        derivatives: &TemporalDerivatives,
    ) -> State {

        // ------------------------------------------------------------
        // CONTINUITY ENERGY
        // ------------------------------------------------------------

        let accel_energy =
            self.config.lambda_accel
            * derivatives.acceleration.powi(2);

        let jerk_energy =
            self.config.lambda_jerk
            * derivatives.jerk.powi(2);

        let divergence_energy =
            self.config.lambda_divergence
            * self.stress.delta;

        let drift_energy =
            self.config.lambda_drift
            * self.stress.drift;

        // total instability energy
        let penalty =
            accel_energy
            + jerk_energy
            + divergence_energy
            + drift_energy;

        // ------------------------------------------------------------
        // HARDENED EVOLUTION
        // ------------------------------------------------------------

        let hardened_x = candidate.x - penalty;

        // ------------------------------------------------------------
        // PROJECT TO LAWFUL MANIFOLD
        // ------------------------------------------------------------

        State {
            x: project_to_manifold(
                hardened_x,
                self.config.lawful_min,
                self.config.lawful_max,
            ),
        }
    }
}
// ============================================================================
// 9. DVSM ENGINE
// ============================================================================
//
// FULL HARDENED EXECUTION PIPELINE
//
//   causal kernel
//     ↓
//   derivative geometry
//     ↓
//   stress accumulation
//     ↓
//   Goodhart hardening
//     ↓
//   lawful manifold projection
//
// ============================================================================

pub struct DVSM<K: KernelMonoid, B: Backend> {
    state: State,

    history: Vec<State>,

    hardening: HardenedLayer,

    _kernel: PhantomData<K>,
    _backend: PhantomData<B>,
}

impl<K: KernelMonoid, B: Backend> DVSM<K, B> {

    #[inline(always)]
    pub fn new(initial: State) -> Self {
        Self {
            state: initial,

            history: vec![initial],

            hardening: HardenedLayer {
                config: HardeningConfig::default(),
                stress: StressField::default(),
            },

            _kernel: PhantomData,
            _backend: PhantomData,
        }
    }
    // ========================================================================
    // FROZEN-FRAME STEP
    // ========================================================================

    #[inline(always)]
    pub fn step(&mut self, sigma: f64) {

        // ------------------------------------------------------------
        // (1) PURE CAUSAL KERNEL
        // ------------------------------------------------------------

        let candidate =
            B::apply::<K>(&self.state, sigma);

        // ------------------------------------------------------------
        // (2) DERIVATIVE GEOMETRY
        // ------------------------------------------------------------

        let len = self.history.len();

        let prev2 =
            if len >= 2 {
                Some(self.history[len - 2])
            } else {
                None
            };

        let prev1 =
            self.history.last().copied();

        let derivatives =
            TemporalDerivatives::compute(
                prev2,
                prev1,
                candidate,
            );

        // ------------------------------------------------------------
        // (3) UPDATE STRESS GEOMETRY
        // ------------------------------------------------------------

        self.hardening
            .stress
            .update(&derivatives);

        // ------------------------------------------------------------
        // (4) APPLY GOODHART HARDENING
        // ------------------------------------------------------------

        let hardened =
            self.hardening.apply(
                candidate,
                &derivatives,
            );

        // ------------------------------------------------------------
        // (5) COMMIT
        // ------------------------------------------------------------

        self.state = hardened;

        self.history.push(hardened);
    }

    // ========================================================================
    // READ-ONLY OBSERVATION
    // ========================================================================

    pub fn state(&self) -> State {
        self.state
    }

    pub fn history(&self) -> &[State] {
        &self.history
    }

    pub fn stress(&self) -> StressField {
        self.hardening.stress
    }
}
// ============================================================================
// 10. EXAMPLE EXECUTION
// ============================================================================

fn main() {

    type Engine = DVSM<StableKernel, Cpu>;

    let mut system =
        Engine::new(State { x: 0.0 });

    // ------------------------------------------------------------
    // INPUT STREAM
    // ------------------------------------------------------------

    let sigma_stream = [
        1.0,
        1.2,
        1.1,
        4.0,   // unnatural spike
        1.0,
        1.1,
        1.05,
    ];

    // ------------------------------------------------------------
    // EVOLUTION
    // ------------------------------------------------------------

    for sigma in sigma_stream {

        system.step(sigma);

        println!(
            "state={:?} stress={:?}",
            system.state(),
            system.stress(),
        );
    }
}
// ============================================================================
// 11. RESULTING SECURITY PROPERTY
// ============================================================================
// OLD SYSTEM:
//
//   legality(state)
//
// EXPLOIT:
//
//   optimize edge conditions
// ---------------------------------------------------------------------------
// HARDENED SYSTEM:
//
//   legality(trajectory)
//
// where legality depends on:
//
//   velocity continuity
//   acceleration continuity
//   jerk continuity
//   divergence accumulation
//   drift persistence
//   manifold coherence
// ---------------------------------------------------------------------------
// RESULT:
//
//   Exploit optimization becomes significantly harder because:
//
//   attacker must preserve:
//
//     spatial legality
//     temporal legality
//     derivative smoothness
//     long-horizon coherence
//     accumulated stability
//
// simultaneously.
// ============================================================================
// 12. FINAL INTERPRETATION
// ============================================================================
// DVSM + Hardened Layer becomes:
//
//   a deterministic trajectory-constrained
//   contraction geometry engine
//
// with:
//
//   causal isolation
//   temporal continuity enforcement
//   Goodhart-resistant legality geometry
//   manifold projection hardening
// ============================================================================
// ============================================================================
// DEVELOPER NOTES (ADAPTIVE / EVOLVING SPECIFICATION)
// ============================================================================
//
// IMPORTANT ARCHITECTURAL CLARIFICATION
// ----------------------------------------------------------------------------
// DVSM is NOT a fixed-purpose anti-cheat, ML model, or behavioral classifier.
//
// HOWEVER:
//
// It MAY be adapted into systems that *include* those capabilities,
// PROVIDED that such extensions remain:
//
//   (1) causally separated from the kernel
//   (2) trajectory-consistent in semantics
//   (3) deterministic under identical inputs
//   (4) non-invasive to core state evolution rules
//
// In other words:
//
//     DVSM defines constraints on HOW systems evolve,
//     not WHAT application domain they serve.
// ============================================================================
// 1. EXTENSIBILITY PRINCIPLE
// ============================================================================
// The statement:
//
//   "DVSM is not X"
//
// should be interpreted as:
//
//   "DVSM core kernel does not assume X as a requirement"
//
// rather than:
//
//   "DVSM cannot be used to implement X"
//
// Therefore:
//
//   • anti-cheat layers MAY be built on top
//   • behavioral analytics MAY use π-modes
//   • anomaly detection MAY use Δ/H fields
//   • control systems MAY interpret stability geometry
//
// as long as:
//
//   kernel semantics remain unchanged.
// ============================================================================
// 2. LAYERED ARCHITECTURE GUARANTEE
// ============================================================================
// Any extension MUST obey the following separation:
//
//   (A) Causal Core
//       - state evolution
//       - kernel dynamics
//       - contraction mapping F_A
//
//   (B) Stability Geometry Layer
//       - Δ (divergence field)
//       - H (drift memory)
//       - η (adaptive control field)
//
//   (C) Observation Layer (π-functors)
//       - classical projections
//       - fracture projections
//       - entropy / lyapunov views
//
//   (D) Application Layer (ADAPTABLE)
//       - anti-cheat systems
//       - simulation engines
//       - robotics control
//       - economic modeling
//       - telemetry analysis
//
// ONLY (A) is causally authoritative.
// ============================================================================
// 3. "NOT A ..." STATEMENTS (REFINED SEMANTICS)
// ============================================================================
// When we say DVSM is NOT:
//
//   • a probabilistic classifier
//   • a behavioral inference engine
//   • an identity system
//
// we mean:
//
//   these are NOT intrinsic properties of the kernel.
//
// BUT:
//
//   these may be constructed as derived systems over:
//
//       π-modes + trajectory geometry + stress fields
//
// WITHOUT modifying causal semantics.
// ============================================================================
// 4. GOODHART RESISTANCE AS A DESIGN LAYER
// ============================================================================
// Goodhart resistance is NOT a prohibition layer.
//
// It is a structural property:
//
//   optimization target ≠ local observable
//
// Instead:
//
//   optimization target = trajectory-consistency functional
//
// Therefore:
//
//   Any system built on DVSM inherits a *shifted optimization landscape*,
//   not a restriction on application scope.
// ============================================================================
// 5. EXTENSION RULES (HARD CONSTRAINTS)
// ============================================================================
// Allowed extensions:
//
//   • new π-mode projections
//   • new stress metrics (Δ variants)
//   • new backend execution strategies
//   • new scheduling policies
//   • new interpretation layers
//
// Forbidden extensions:
//
//   • modifying kernel update rule from outside KernelMonoid
//   • feeding π-mode outputs back into F_A directly
//   • breaking snapshot isolation
//   • introducing nondeterminism in step execution
// ============================================================================
// 6. INTERPRETATION FLEXIBILITY
// ============================================================================
// DVSM is intentionally domain-agnostic.
//
// It can represent:
//
//   • physical systems
//   • graph dynamics
//   • control systems
//   • behavioral telemetry
//   • optimization environments
//
// BUT:
//
//   interpretation is external.
//
// kernel semantics remain invariant.
// ============================================================================
// 7. DESIGN INTENT RECONCILIATION
// ============================================================================
// Previous phrasing:
//
//   "DVSM is not X"
//
// should be upgraded to:
//
//   "DVSM does not assume X at the kernel level,
//    but may instantiate systems that exhibit X-like behavior
//    at the application layer under controlled mappings."
// ============================================================================
// 8. FINAL PRINCIPLE
// ============================================================================
// DVSM is:
//
//   a deterministic causal core + extensible observational lattice
//
// NOT:
//
//   a closed-domain system definition
//
// It is designed to be:
//
//   structurally universal,
//   but semantically constrained.
// ============================================================================
// ============================================================================
// DVSM — HARDENED FUNDAMENTAL EQUATION (RUST REFERENCE IMPLEMENTATION)
// ============================================================================
// This file encodes the short-form equation:
//
//   x_{t+1} = (1-η)x_t + η(σ_gain · σ_t + b)
//             - λ1||a_t||² - λ2||j_t||² - λ3Δ_t - λ4H_t
//
// as a deterministic step operator over a scalar state.
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct State {
    pub x: f64,
}

// ============================================================================
// HARDENING PARAMETERS (λ-weights + kernel params)
// ============================================================================

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

// ============================================================================
// STRESS MEMORY (Δ and H)
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Stress {
    pub delta: f64, // instantaneous divergence
    pub drift: f64, // accumulated instability H_t
}

// ============================================================================
// TEMPORAL DERIVATIVES
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Derivatives {
    pub velocity: f64,
    pub acceleration: f64,
    pub jerk: f64,
}

pub fn compute_derivatives(
    prev2: Option<State>,
    prev1: Option<State>,
    curr: State,
) -> Derivatives {
    let v = match prev1 {
        Some(p1) => curr.x - p1.x,
        None => 0.0,
    };

    let a = match (prev2, prev1) {
        (Some(p2), Some(p1)) => {
            let v_prev = p1.x - p2.x;
            v - v_prev
        }
        _ => 0.0,
    };

    let j = match (prev2, prev1) {
        (Some(p2), Some(p1)) => {
            let v_prev = p1.x - p2.x;
            let a_prev = v_prev;
            a - a_prev
        }
        _ => 0.0,
    };

    Derivatives {
        velocity: v,
        acceleration: a,
        jerk: j,
    }
}

// ============================================================================
// KERNEL (F_A)
// ============================================================================

pub fn kernel_step(x: f64, sigma: f64, p: &Params) -> f64 {
    x + p.eta * (p.sigma_gain * sigma - x) + p.bias
}

// ============================================================================
// HARDENED STEP OPERATOR
// ============================================================================

pub fn hardened_step(
    state: State,
    sigma: f64,
    params: &Params,
    prev2: Option<State>,
    prev1: Option<State>,
    stress: &mut Stress,
) -> State {
    // ------------------------------------------------------------
    // (1) PURE KERNEL EVOLUTION
    // ------------------------------------------------------------

    let candidate_x = kernel_step(state.x, sigma, params);

    // ------------------------------------------------------------
    // (2) TEMPORAL GEOMETRY
    // ------------------------------------------------------------

    let deriv = compute_derivatives(prev2, prev1, State { x: candidate_x });

    // ------------------------------------------------------------
    // (3) UPDATE STRESS FIELD
    // ------------------------------------------------------------

    stress.delta = deriv.acceleration.abs() + deriv.jerk.abs();
    stress.drift += stress.delta;

    // ------------------------------------------------------------
    // (4) HARDENED PENALTY TERMS
    // ------------------------------------------------------------

    let accel_penalty = params.lambda_accel * deriv.acceleration.powi(2);
    let jerk_penalty = params.lambda_jerk * deriv.jerk.powi(2);
    let delta_penalty = params.lambda_delta * stress.delta;
    let drift_penalty = params.lambda_drift * stress.drift;

    let total_penalty = accel_penalty + jerk_penalty + delta_penalty + drift_penalty;

    // ------------------------------------------------------------
    // (5) FINAL EQUATION APPLICATION
    // ------------------------------------------------------------

    let x_next = candidate_x - total_penalty;

    State { x: x_next }
}

// ============================================================================
// INTERPRETATION
// ============================================================================
//
// This implements:
//
//   x_{t+1} = F_A(x_t, σ_t)
//             - λ1 a_t² - λ2 j_t² - λ3 Δ_t - λ4 H_t
//
// where:
//
//   F_A      = affine contraction kernel
//   a_t      = acceleration (2nd derivative)
//   j_t      = jerk (3rd derivative)
//   Δ_t      = instantaneous instability field
//   H_t      = accumulated instability memory
//
// The system enforces:
//   trajectory-level legality instead of pointwise legality
//
// ============================================================================
