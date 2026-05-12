// ============================================================================
// DVSM — FINAL HARDENED DETERMINISTIC STATE MACHINE (Tightened Addendum Below)
// Single latent state + bounded memory + lossy observation + derived diagnostics
// Author: Daniel J. Dillberg
// ============================================================================

#![allow(dead_code)]

// ============================================================================
// CONSTANTS
// ============================================================================

const PHI: f64 = 1.61803398875;

// ============================================================================
// 1. CORE STATE SPACE (S_t = (v_t, H_t))
// ============================================================================
//
// Invariants:
//   v ∈ [0,1)
//   H is bounded FIFO memory
//   hash is derived (non-state diagnostic only)

#[derive(Clone, Debug)]
pub struct SystemState {
    v: f64,
    h: Vec<f64>,
    hash: u64,
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
    // READ ACCESS
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
    fn normalize(x: f64) -> f64 {
        x.fract()
    }

    #[inline(always)]
    fn set_v(&mut self, value: f64) {
        self.v = Self::normalize(value);
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

    #[inline(always)]
    fn update_hash(&mut self) {
        self.hash = Self::compute_hash(self.v, &self.h);
    }
}

// ============================================================================
// 2. OBSERVATION LAYER (PURE PROJECTION)
// ============================================================================
//
// v¹ = identity projection
// v² = nonlinear lossy projection

pub struct Observation;

impl Observation {
    #[inline(always)]
    pub fn project(v: f64) -> (f64, f64) {
        (v, (v * PHI).fract())
    }
}

// ============================================================================
// 3. CORE DYNAMICS (F: S × U → S)
// ============================================================================

pub struct DVSM;

impl DVSM {
    #[inline(always)]
    pub fn step(state: &mut SystemState, input: f64, memory_limit: usize) {

        // ----------------------------------------
        // STATE UPDATE
        // ----------------------------------------
        let new_v = state.v() + input;
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
// 4. MEMORY POLICY (OPTIONAL EXTERNAL GUARD)
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
// 5. TRACE REGIME MODEL (DIAGNOSTIC ONLY)
// ============================================================================

pub struct TraceRegimeModel;

impl TraceRegimeModel {
    #[inline(always)]
    pub fn regime(len: usize) -> &'static str {
        match len {
            0..=31 => "Low",
            32..=255 => "Medium",
            _ => "High",
        }
    }
}

// ============================================================================
// 6. CONSTRAINT MODEL (SPECIFICATION ONLY)
// ============================================================================

pub struct Constraints;

impl Constraints {
    pub fn deterministic() -> bool { true }

    pub fn bounded_memory(h: &[f64], n: usize) -> bool {
        h.len() <= n
    }

    pub fn observation_isolated() -> bool { true }

    pub fn lossy_projection() -> bool { true }

    pub fn no_feedback_from_observation() -> bool { true }
}

// ============================================================================
// 7. SYSTEM CLASSIFICATION
// ============================================================================

pub struct System;

impl System {
    pub fn classify() -> &'static str {
        "Deterministic bounded-memory nonlinear recurrence system with dual lossy observation channels and derived diagnostic fingerprint"
    }
}

// ============================================================================
// 8. FORMAL MATHEMATICAL MODEL
// ============================================================================
//
// STATE:
//   S_t = (v_t, H_t)
//
// CONSTANT:
//   α = PHI
//
// DYNAMICS:
//   v_{t+1} = fract(v_t + u_t)
//   H_{t+1} = truncate(H_t ∪ {v_{t+1}})
//
// OBSERVATION:
//   O(v_t) = (v_t, fract(α v_t))
//
// DERIVED DIAGNOSTIC:
//   Φ_t = hash(v_t, H_t[0:16])
//
// NOTE:
//   Φ_t is NOT part of system evolution
//   Observation is causally inert by design
//
// ============================================================================
// 9. FORMAL MATHEMATICAL MODEL (CLOSED INTERPRETATION LAYER)
// ============================================================================
//
// This section defines the mathematical interpretation of the implemented system.
// It MUST NOT introduce additional state beyond SystemState.
// It is a declarative mapping of code → formal structure.
//
// ============================================================================
//
// STATE (ONLY DYNAMICAL OBJECT)
// ---------------------------------------------------------------------------
//
// S_t = (v_t, H_t)
//
// where:
//   v_t ∈ [0,1)
//   H_t is a FIFO-bounded history buffer
//
// No other variables participate in system evolution.
//
// ============================================================================
//
// DYNAMICS (ONLY TRANSITION FUNCTION)
// ---------------------------------------------------------------------------
//
// Let u_t be external input.
//
// v_{t+1} = fract(v_t + u_t)
//
// H_{t+1} = FIFO_cap(H_t, v_{t+1})
//
// where:
//
// FIFO_cap(H, x):
//   1. append x to H
//   2. if |H| > cap, remove oldest elements until |H| = cap
//
// NOTE:
// - No set operations are used (this is NOT a set union system)
// - History is ordered and destructive (not persistent)
//
// ============================================================================
//
// OBSERVATION (NON-CAUSAL PROJECTION)
// ---------------------------------------------------------------------------
//
// O(v_t) = (v_t, fract(α · v_t)),  where α = PHI
//
// Properties:
//   - O is non-injective
//   - O does NOT influence S_{t+1}
//   - O is a pure function of current state only
//
// ============================================================================
//
// DERIVED DIAGNOSTIC (NON-STATE FUNCTIONAL)
// ---------------------------------------------------------------------------
//
// Φ_t = F(S_t)
//
// where F is the hash function:
//
// Φ_t = hash(v_t, H_t[0:min(16, |H_t|)])
//
// IMPORTANT:
//   - Φ_t is NOT part of S_t
//   - Φ_t does NOT affect evolution
//   - Φ_t is computed AFTER state transition
//
// ============================================================================
//
// CAUSAL STRUCTURE
// ---------------------------------------------------------------------------
//
// S_t  →  S_{t+1}   (only valid transition relation)
// S_t  →  O(S_t)    (observation only)
// S_t  →  Φ_t       (diagnostic only)
//
// No reverse edges exist.
//
// ============================================================================
//
// SYSTEM PROPERTY SUMMARY
// ---------------------------------------------------------------------------
//
// - Deterministic evolution
// - Single scalar dynamical variable
// - Bounded FIFO memory trace
// - Non-injective observation mapping
// - Purely derived diagnostics
// - No hidden or auxiliary state

// STATE:
//  S_t = (v_t, H_t)

// DYNAMICS:
//  v_{t+1} = fract(v_t + u_t)
//  H_{t+1} = FIFO_cap(H_t, v_{t+1})

// OBSERVATION:
//  O(v_t) = (v_t, fract(α v_t))

// DIAGNOSTIC (NON-STATE):
//  Φ_t = F(S_t)

// PROPERTIES:
//  - O is non-injective
//  - Φ_t does not influence S_{t+1}
//   - system is causally closed over S_t only
//
// ============================================================================
// END FORMAL MODEL
// ============================================================================
// ============================================================================
// DVSM — FINAL HARDENED DETERMINISTIC STATE MACHINE
// Hardened Addendum Integrated
// ============================================================================

#![allow(dead_code)]

// ============================================================================
// CONSTANTS
// ============================================================================

const PHI: f64 = 1.61803398875;

// ============================================================================
// 1. CORE STATE SPACE (S_t = (v_t, H_t))
// ============================================================================
//
// Invariants:
//   v ∈ [0,1)
//   H is bounded FIFO memory
//   hash is derived (non-state diagnostic only)

#[derive(Clone, Debug)]
pub struct SystemState {
    v: f64,
    h: Vec<f64>,
    hash: u64,
}

impl SystemState {
    pub fn new(capacity: usize) -> Self {
        Self {
            v: 0.0,
            h: Vec::with_capacity(capacity),
            hash: 0,
        }
    }

    // ------------------------------------------------------------------------
    // READ ACCESS
    // ------------------------------------------------------------------------

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

    // ------------------------------------------------------------------------
    // INVARIANT ENFORCEMENT
    // ------------------------------------------------------------------------
    //
    // HARDENED:
    // Rust fract() preserves sign for negatives.
    //
    // We therefore use true modular normalization:
    //
    // normalize(x) = x - floor(x)
    //
    // guaranteeing:
    //   normalize(x) ∈ [0,1)
    //
    // ------------------------------------------------------------------------

    #[inline(always)]
    fn normalize(x: f64) -> f64 {
        x - x.floor()
    }

    #[inline(always)]
    fn set_v(&mut self, value: f64) {
        self.v = Self::normalize(value);

        debug_assert!(self.v >= 0.0);
        debug_assert!(self.v < 1.0);
    }

    #[inline(always)]
    fn push_history(&mut self, value: f64, limit: usize) {
        self.h.push(value);

        if self.h.len() > limit {
            let excess = self.h.len() - limit;
            self.h.drain(0..excess);
        }

        debug_assert!(self.h.len() <= limit);

        #[cfg(debug_assertions)]
        for &x in &self.h {
            debug_assert!(x >= 0.0);
            debug_assert!(x < 1.0);
        }
    }

    // ------------------------------------------------------------------------
    // DERIVED FINGERPRINT (NON-STATE)
    // ------------------------------------------------------------------------
    //
    // Φ_t ∈ {0,1}^64
    //
    // Non-invertible diagnostic collapse.
    // Explicitly excluded from state evolution.
    //
    // ------------------------------------------------------------------------

    #[inline(always)]
    fn compute_hash(v: f64, h: &[f64]) -> u64 {
        let mut acc = v.to_bits().wrapping_mul(0x9E3779B97F4A7C15);

        for x in h.iter().take(16) {
            acc ^= x.to_bits().wrapping_mul(0xBF58476D1CE4E5B9);
            acc = acc.rotate_left(5);
        }

        acc
    }

    #[inline(always)]
    fn update_hash(&mut self) {
        self.hash = Self::compute_hash(self.v, &self.h);
    }
}

// ============================================================================
// 2. OBSERVATION LAYER (PURE PROJECTION)
// ============================================================================
//
// Observation manifold:
//
// O(v_t) = (v_t, mod(φv_t, 1))
//
// Properties:
//   - non-injective
//   - causally inert
//   - pure projection
//
// ============================================================================

pub struct Observation;

impl Observation {
    #[inline(always)]
    pub fn project(v: f64) -> (f64, f64) {
        (v, SystemState::normalize(v * PHI))
    }
}

// ============================================================================
// 3. CORE DYNAMICS (T : S × ℝ → S)
// ============================================================================
//
// T(v_t, H_t, u_t) =
//   ( mod(v_t + u_t, 1),
//     push(mod(v_t + u_t, 1), H_t) )
//
// Non-autonomous discrete-time map.
//
// ============================================================================

pub struct DVSM;

impl DVSM {
    #[inline(always)]
    pub fn step(
        state: &mut SystemState,
        input: f64,
        memory_limit: usize,
    ) {
        // --------------------------------------------------------------------
        // TRANSITION OPERATOR
        // --------------------------------------------------------------------

        let new_v = state.v() + input;

        state.set_v(new_v);

        // --------------------------------------------------------------------
        // MEMORY UPDATE (FIFO CAP)
        // --------------------------------------------------------------------

        state.push_history(state.v(), memory_limit);

        // --------------------------------------------------------------------
        // DERIVED DIAGNOSTIC UPDATE
        // --------------------------------------------------------------------

        state.update_hash();

        // --------------------------------------------------------------------
        // GLOBAL INVARIANT CHECKS
        // --------------------------------------------------------------------

        debug_assert!(Constraints::bounded_memory(
            state.history(),
            memory_limit
        ));

        debug_assert!(state.v() >= 0.0);
        debug_assert!(state.v() < 1.0);
    }
}

// ============================================================================
// 4. MEMORY POLICY (OPTIONAL EXTERNAL GUARD)
// ============================================================================

pub struct MemoryPolicy;

impl MemoryPolicy {
    #[inline(always)]
    pub fn enforce(h: &mut Vec<f64>, limit: usize) {
        if h.len() > limit {
            let excess = h.len() - limit;
            h.drain(0..excess);
        }

        debug_assert!(h.len() <= limit);
    }
}

// ============================================================================
// 5. TRACE REGIME MODEL (DIAGNOSTIC ONLY)
// ============================================================================

pub struct TraceRegimeModel;

impl TraceRegimeModel {
    #[inline(always)]
    pub fn regime(len: usize) -> &'static str {
        match len {
            0..=31 => "Low",
            32..=255 => "Medium",
            _ => "High",
        }
    }
}

// ============================================================================
// 6. CONSTRAINT MODEL (SPECIFICATION ONLY)
// ============================================================================
//
// Non-Interference Property:
//
// ∂v_{t+1} / ∂O(v_t) = 0
//
// Observation is causally isolated.
//
// ============================================================================

pub struct Constraints;

impl Constraints {
    #[inline(always)]
    pub fn deterministic() -> bool {
        true
    }

    #[inline(always)]
    pub fn bounded_memory(h: &[f64], n: usize) -> bool {
        h.len() <= n
    }

    #[inline(always)]
    pub fn observation_isolated() -> bool {
        true
    }

    #[inline(always)]
    pub fn lossy_projection() -> bool {
        true
    }

    #[inline(always)]
    pub fn no_feedback_from_observation() -> bool {
        true
    }

    #[inline(always)]
    pub fn latent_interval(v: f64) -> bool {
        v >= 0.0 && v < 1.0
    }
}

// ============================================================================
// 7. SYSTEM CLASSIFICATION
// ============================================================================

pub struct System;

impl System {
    pub fn classify() -> &'static str {
        "Deterministic bounded-memory nonlinear recurrence system \
with dual lossy observation channels and derived diagnostic fingerprint"
    }
}

// ============================================================================
// 8. INTERPRETATION LAYER
// ============================================================================
//
// Declarative mathematical interpretation only.
// Introduces NO additional state.
//
// ============================================================================

pub struct Interpretation;

impl Interpretation {

    #[inline(always)]
    pub fn state_topology() -> &'static str {
        "Toroidal [0, 1) latent space with FIFO history manifold"
    }

    #[inline(always)]
    pub fn dynamical_class() -> &'static str {
        "Non-autonomous discrete-time map"
    }

    #[inline(always)]
    pub fn transition_operator() -> &'static str {
        "T : S × ℝ → S"
    }

    #[inline(always)]
    pub fn observation_manifold() -> &'static str {
        "2D nonlinear lossy projection manifold"
    }

    #[inline(always)]
    pub fn entropy_model() -> &'static str {
        "Finite-memory bounded deterministic recurrence with 64-bit diagnostic collapse"
    }
}

// ============================================================================
// 9. FORMAL MATHEMATICAL MODEL
// ============================================================================
//
// STATE SPACE:
//   S = [0,1) × [0,1)^N
//
// LATENT STATE:
//   S_t = (v_t, H_t)
//
// TRANSITION OPERATOR:
//   T(v_t, H_t, u_t) =
//      ( (v_t + u_t) mod 1,
//        push((v_t + u_t) mod 1, H_t) )
//
// OBSERVATION:
//   O(v_t) = (v_t, mod(φv_t, 1))
//
// NON-INTERFERENCE:
//   ∂v_{t+1} / ∂O(v_t) = 0
//
// DIAGNOSTIC:
//   Φ_t = hash(v_t, H_t[0:16])
//
// INVARIANT:
//   ∀t:
//      v_t ∈ [0,1)
//      |H_t| ≤ memory_limit
//
// ============================================================================
//
// END FILE
// ============================================================================
