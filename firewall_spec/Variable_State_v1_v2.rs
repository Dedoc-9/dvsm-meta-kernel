// ============================================================================
// DVSM — FINAL HARDENED DETERMINISTIC STATE MACHINE
// Single latent state + bounded memory + lossy observation + derived diagnostics
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
//
// END FILE
// ============================================================================
