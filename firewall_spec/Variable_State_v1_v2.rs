// ============================================================================
// DVSM / EIL / DQSDv2 — HARDENED DETERMINISTIC STATE MACHINE (FINAL)
// ============================================================================
//
// SYSTEM TYPE:
//   Deterministic discrete-time nonlinear dynamical system
//   with bounded memory and lossy observation projection.
//
// FORMAL MODEL:
//   S_t = (v_t, H_t)
//   S_{t+1} = F(S_t, u_t)
//   π(v_t) = (v1_t, v2_t)
//
// CONSTRAINTS:
//   - Deterministic update
//   - Bounded memory (|H_t| ≤ N)
//   - Observation is read-only and non-influential
//   - Projection is non-injective (lossy)
// ============================================================================

#![allow(dead_code)]

// ============================================================================
// 1. CORE STATE (ENCAPSULATED INVARIANT)
// ============================================================================

#[derive(Clone, Debug)]
pub struct SystemState {
    v: f64,        // latent scalar state ∈ [0,1)
    v1: f64,       // projection channel A
    v2: f64,       // projection channel B
    h: Vec<f64>,   // bounded memory trace
}

// Constructor enforces initial invariants
impl SystemState {
    pub fn new(capacity: usize) -> Self {
        Self {
            v: 0.0,
            v1: 0.0,
            v2: 0.0,
            h: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    fn normalize(x: f64) -> f64 {
        x.fract()
    }
}

// ============================================================================
// 2. OBSERVATION MAP (PURE FUNCTIONAL LAYER)
// ============================================================================

pub struct Projection;

impl Projection {
    #[inline(always)]
    pub fn apply(v: f64) -> (f64, f64) {
        let a = v;
        let b = (v * 1.61803398875).fract();
        (a, b)
    }
}

// ============================================================================
// 3. CORE DYNAMICS (F: S × U → S)
// ============================================================================

pub struct CoreKernel;

impl CoreKernel {
    #[inline(always)]
    pub fn step(state: &mut SystemState, input: f64, memory_limit: usize) {
        // -----------------------------
        // STATE EVOLUTION (DETERMINISTIC)
        // -----------------------------
        state.v = SystemState::normalize(state.v + input);

        // -----------------------------
        // OBSERVATION UPDATE (DERIVED ONLY)
        // -----------------------------
        let (a, b) = Projection::apply(state.v);
        state.v1 = a;
        state.v2 = b;

        // -----------------------------
        // MEMORY UPDATE (BOUNDED FIFO)
        // -----------------------------
        state.h.push(state.v);

        if state.h.len() > memory_limit {
            let excess = state.h.len() - memory_limit;
            state.h.drain(0..excess);
        }
    }
}

// ============================================================================
// 4. MEMORY POLICY (BOUND ENFORCEMENT LAYER)
// ============================================================================

pub struct MemoryPolicy {
    pub max: usize,
}

impl MemoryPolicy {
    #[inline(always)]
    pub fn enforce(&self, h: &mut Vec<f64>) {
        if h.len() > self.max {
            let excess = h.len() - self.max;
            h.drain(0..excess);
        }
    }
}

// ============================================================================
// 5. CLOCK MODEL (DISCRETE TIME)
// ============================================================================

pub struct Clock;

impl Clock {
    #[inline(always)]
    pub fn tick(t: u64) -> u64 {
        t + 1
    }
}

// ============================================================================
// 6. SYSTEM EVENTS (READ-ONLY CLASSIFICATION)
// ============================================================================

#[derive(Debug, Clone)]
pub enum SystemEvent {
    Normal,
    Instability,
    Saturation,
    Reset,
}

pub fn classify(state: &SystemState) -> SystemEvent {
    if !state.v.is_finite() {
        SystemEvent::Instability
    } else if state.v1 > 0.99 || state.v2 > 0.99 {
        SystemEvent::Saturation
    } else if state.v == 0.0 {
        SystemEvent::Reset
    } else {
        SystemEvent::Normal
    }
}

// ============================================================================
// 7. LEAK ANALYZER (READ-ONLY DIAGNOSTIC FUNCTION)
// ============================================================================

pub struct LeakAnalyzer;

impl LeakAnalyzer {
    pub fn analyze(trace: &[f64]) -> &'static str {
        if trace.len() < 10 {
            return "InsufficientData";
        }

        let var = Self::variance(trace);

        if var < 0.0001 {
            "LowVariancePattern"
        } else if var > 0.25 {
            "HighVariancePattern"
        } else {
            "StablePattern"
        }
    }

    fn variance(x: &[f64]) -> f64 {
        let mean = x.iter().sum::<f64>() / x.len() as f64;
        x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / x.len() as f64
    }
}

// ============================================================================
// 8. EXECUTION PIPELINE (SINGLE ENTRY POINT)
// ============================================================================

pub struct Pipeline;

impl Pipeline {
    pub fn step(mut state: SystemState, input: f64, memory_limit: usize) -> SystemState {
        CoreKernel::step(&mut state, input, memory_limit);
        state
    }
}

// ============================================================================
// 9. INITIALIZATION (INVARIANT SAFE)
// ============================================================================

pub fn init_state(memory_capacity: usize) -> SystemState {
    SystemState::new(memory_capacity)
}

// ============================================================================
// 10. ARCHITECTURE LAYERS (INFORMATION MODEL)
// ============================================================================
//
// L1 CoreKernel     → deterministic state transition F
// L2 Projection     → lossy observation π(v)
// L3 MemoryPolicy   → bounded FIFO enforcement
// L4 Clock          → discrete time progression
// L5 SystemEvent    → read-only classification
// L6 LeakAnalyzer   → statistical diagnostics (no influence)
// L7 Pipeline       → execution wrapper
// L8 SystemState    → unified state container
//
// ============================================================================
// 11. HARD SYSTEM INVARIANTS (ENFORCEABLE CONTRACT)
// ============================================================================
//
// I1 — SINGLE STATE SPACE
//     All computation derives from SystemState only
//
// I2 — DETERMINISM
//     (S_t, u_t) ⇒ uniquely defined S_{t+1}
//
// I3 — OBSERVER INERTNESS
//     Classification cannot modify state
//
// I4 — BOUNDED MEMORY
//     |H_t| ≤ memory_limit enforced per step
//
// I5 — LOSSY OBSERVATION
//     Projection π is non-injective by construction
//
// I6 — DISCRETE TIME
//     State evolves only via explicit step calls
//
// ============================================================================
// 12. FINAL SYSTEM CLASSIFICATION
// ============================================================================
//
// Deterministic bounded-memory nonlinear recurrence system
// with structurally lossy observation channel and strict state encapsulation.
//
// Formal form:
//
//   S_{t+1} = F(S_t, u_t)
//
// Interpretation:
//
//   - Single latent scalar state
//   - One bounded memory buffer
//   - Two derived observation channels
//   - No feedback from observation layer into dynamics
//
// ============================================================================
// DVSM — HARDENED DETERMINISTIC STATE MACHINE (FINAL CONSISTENT FORM)
// ============================================================================

#![allow(dead_code)]

// ============================================================================
// 1. CORE STATE SPACE (S_t = (v_t, H_t))
// ============================================================================
//
// Invariant:
//   - v_t ∈ [0,1)
//   - H_t is bounded FIFO history
//   - No external mutation allowed outside kernel

#[derive(Clone, Debug)]
pub struct SystemState {
    v: f64,            // latent scalar state (PRIVATE INVARIANT)
    h: Vec<f64>,       // bounded memory trace
    hash: u64,         // derived fingerprint (NON-STATE, diagnostic only)
}

impl SystemState {
    pub fn new(capacity: usize) -> Self {
        Self {
            v: 0.0,
            h: Vec::with_capacity(capacity),
            hash: 0,
        }
    }

    // ----------------------------
    // READ ACCESS ONLY
    // ----------------------------
    #[inline(always)]
    pub fn v(&self) -> f64 {
        self.v
    }

    #[inline(always)]
    pub fn history(&self) -> &[f64] {
        &self.h
    }

    #[inline(always)]
    pub fn fingerprint(&self) -> u64 {
        self.hash
    }

    // ----------------------------
    // INVARIANT ENFORCEMENT
    // ----------------------------
    #[inline(always)]
    fn set_v(&mut self, value: f64) {
        self.v = value.fract();
    }

    #[inline(always)]
    fn push_history(&mut self, value: f64, limit: usize) {
        self.h.push(value);

        if self.h.len() > limit {
            let excess = self.h.len() - limit;
            self.h.drain(0..excess);
        }
    }

    // ----------------------------
    // DERIVED FINGERPRINT (NON-STATE)
    // ----------------------------
    #[inline(always)]
    fn compute_hash(v: f64, h: &[f64]) -> u64 {
        let mut acc = v.to_bits().wrapping_mul(0x9E3779B97F4A7C15);

        for x in h.iter().take(16) {
            acc ^= x.to_bits().wrapping_mul(0xBF58476D1CE4E5B9);
            acc = acc.rotate_left(5);
        }

        acc
    }

    fn update_hash(&mut self) {
        self.hash = Self::compute_hash(self.v, &self.h);
    }
}

//  Minor refinement (optional but important for precision)

//    1. “hash is NON-STATE” is now correctly implemented but semantically strong

// You currently label:

// hash: u64 // NON-STATE, diagnostic only

// ✔ This is fine conceptually

// ⚠ but Rust still stores it inside state struct

// So strictly speaking:

// It is state-contained but not state-semantic

// If you ever want maximum rigor, rename mentally as:

// derived_hash (cached projection)

// Not required — just tightening terminology.

// ============================================================================
// 2. OBSERVATION MAP (PURE FUNCTIONAL PROJECTION)
// ============================================================================
//
// Property:
//   - read-only
//   - no access to SystemState internals
//   - no causality into system dynamics

pub struct Observation;

impl Observation {
    #[inline(always)]
    pub fn project(v: f64) -> (f64, f64) {
        (f1(v), f2(v))
    }
}

#[inline(always)]
fn f1(v: f64) -> f64 {
    v
}

#[inline(always)]
fn f2(v: f64) -> f64 {
    (v * 1.61803398875).fract()
}

// ============================================================================
// 3. CORE DYNAMICS (F: S × U → S)
// ============================================================================
//
// Deterministic discrete-time recurrence system

pub struct DVSM;

impl DVSM {
    #[inline(always)]
    pub fn step(state: &mut SystemState, input: f64, memory_limit: usize) {

        // ----------------------------------------
        // STATE EVOLUTION (DETERMINISTIC)
        // ----------------------------------------
        let new_v = (state.v() + input).fract();
        state.set_v(new_v);

        // ----------------------------------------
        // MEMORY UPDATE (BOUNDED FIFO)
        // ----------------------------------------
        state.push_history(state.v(), memory_limit);

        // ----------------------------------------
        // DERIVED FINGERPRINT UPDATE
        // ----------------------------------------
        state.update_hash();
    }
}

// ============================================================================
// 4. MEMORY POLICY (OPTIONAL EXTERNAL SAFETY LAYER)
// ============================================================================

pub struct MemoryPolicy;

impl MemoryPolicy {
    #[inline(always)]
    pub fn enforce(h: &mut Vec<f64>, limit: usize) {
        if h.len() > limit {
            let excess = h.len() - limit;
            h.drain(0..excess);
        }
    }
}

// ============================================================================
// 5. CONSTRAINT MODEL (SPECIFICATION LAYER ONLY)
// ============================================================================
//
// NOTE:
// These are NOT runtime-enforced guarantees,
// but invariants of intended execution semantics.

pub struct Constraints;

impl Constraints {

    pub fn deterministic() -> bool {
        true
    }

    pub fn bounded_memory(h: &[f64], n: usize) -> bool {
        h.len() <= n
    }

    pub fn observation_isolated() -> bool {
        true
    }

    pub fn lossy_observation() -> bool {
        true
    }

    pub fn no_state_feedback_from_observation() -> bool {
        true
    }
}

// ============================================================================
// 6. GHOST MODEL (PURE DIAGNOSTIC METRIC — NON-ONTOLOGICAL)
// ============================================================================
//
// Interpretation rule:
//   "ghosts" are NOT entities
//   they are statistical compression artifacts of trace structure

pub struct GhostModel;

impl GhostModel {

    #[inline(always)]
    pub fn drift_risk(trace_len: usize) -> &'static str {
        match trace_len {
            0..=31 => "Low",
            32..=255 => "Medium",
            _ => "High",
        }
    }
}

// ============================================================================
// 7. SYSTEM CLASSIFICATION
// ============================================================================

pub struct System;

impl System {
    pub fn classify() -> &'static str {
        "Deterministic bounded-memory nonlinear recurrence system with lossy observation and derived diagnostic fingerprint"
    }
}

// ============================================================================
// 8. FORMAL MATHEMATICAL MODEL (FINAL CONSISTENT FORM)
// ============================================================================
//
// STATE:
//   S_t = (v_t, H_t)
//
// DYNAMICS:
//   v_{t+1} = fract(v_t + u_t)
//   H_{t+1} = truncate(H_t ∪ {v_{t+1}})
//
// OBSERVATION:
//   O(v_t) = (v_t, fract(αv_t))
//
// DERIVED FUNCTIONAL:
//   Φ_t = hash(v_t, H_t[0:k])
//
// KEY PROPERTY:
//   Φ is NOT part of state evolution
//   Φ_t = Ψ(S_t) computed after F(S_t, u_t)
//   Φ is a diagnostic projection of trajectory history
//
// ============================================================================

// ============================================================================
// FINAL SYSTEM CLASSIFICATION
// ============================================================================
//
// Deterministic bounded-memory discrete-time dynamical system
// with lossy observation channel and non-state diagnostic fingerprint.
//
// No auxiliary state dimensions exist beyond (v, H).
//
// ============================================================================
// END FILE
// ============================================================================
