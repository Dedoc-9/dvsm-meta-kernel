//==============================================================================
// DVSM — CLOSED DETERMINISTIC BOUNDED STATE MACHINE (CANONICAL FORM)
// Author: Daniel J. Dillberg
//==============================================================================
//
// Axiom Summary:
//
// State:
//   S_t = (v_t, H_t, cap)
//
//   v_t ∈ [0,1)
//   H_t = FIFO trace of last ≤ cap values of v
//
// Dynamics:
//   v_{t+1} = fract(v_t + u_t)
//   truncate_append(H, x, cap)
  = let H' = H ⧺ [x]
    in { H'[i] | i ∈ ℕ, |H'| - cap ≤ i < |H'| }
//
// Observation (causally inert):
//   O(v) = (v, fract(PHI * v))
//
// Properties:
//   - deterministic
//   - bounded memory (logical FIFO horizon)
//   - no hidden state
//   - no feedback from observation
//==============================================================================

#![allow(dead_code)]

const PHI: f64 = 1.61803398875;

//==============================================================================
// 1. STATE SPACE (SINGLE SOURCE OF TRUTH)
//==============================================================================

#[derive(Clone, Debug)]
pub struct State {
    v: f64,        // canonical scalar state ∈ [0,1)
    h: Vec<f64>,   // FIFO history (bounded logical trace)
    cap: usize,    // immutable memory horizon (set at construction)
}

impl State {

    //--------------------------------------------------------------------------
    // CONSTRUCTION (ONLY ENTRY POINT)
    //--------------------------------------------------------------------------

    #[inline(always)]
    pub fn new(capacity: usize) -> Self {
        Self {
            v: 0.0,
            h: Vec::with_capacity(capacity),
            cap: capacity,
        }
    }

    //--------------------------------------------------------------------------
    // READ INTERFACE (IMMUTABLE SURFACE)
    //--------------------------------------------------------------------------

    #[inline(always)]
    pub fn v(&self) -> f64 {
        self.v
    }

    #[inline(always)]
    pub fn history(&self) -> &[f64] {
        &self.h
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    //--------------------------------------------------------------------------
    // SINGLE NORMALIZATION OPERATOR (CANONICAL)
    //--------------------------------------------------------------------------

    #[inline(always)]
    fn normalize(x: f64) -> f64 {
        x.fract()
    }

    //--------------------------------------------------------------------------
    // STATE WRITE (ONLY AUTHORIZED MUTATION PATH)
    //--------------------------------------------------------------------------

    #[inline(always)]
    fn set_v(&mut self, v: f64) {
        self.v = Self::normalize(v);
    }

    //--------------------------------------------------------------------------
    // MEMORY OPERATOR (STRICT FIFO SEMANTICS)
    //--------------------------------------------------------------------------

    #[inline(always)]
    fn push_fifo(&mut self, value: f64) {
        self.h.push(value);

        // enforce logical horizon constraint
        if self.h.len() > self.cap {
            let excess = self.h.len() - self.cap;
            self.h.drain(0..excess);
        }
    }
}

//==============================================================================
// 2. INPUT DOMAIN (REACHABILITY FILTER)
//==============================================================================

#[inline(always)]
fn validate_input(u: f64) -> bool {
    u.is_finite()
}

//==============================================================================
// 3. OBSERVATION (PURE PROJECTION, NO STATE ACCESS)
//==============================================================================

#[inline(always)]
pub fn observe(v: f64) -> (f64, f64) {
    (v, (v * PHI).fract())
}

//==============================================================================
// 4. DYNAMICS (SINGLE TRANSITION FUNCTION F)
//==============================================================================
//
// S_{t+1} = F(S_t, u_t)
//==============================================================================

#[inline(always)]
pub fn step(state: &mut State, input: f64) {

    //--------------------------------------------------------------------------
    // INPUT GATE
    //--------------------------------------------------------------------------

    if !validate_input(input) {
        return;
    }

    //--------------------------------------------------------------------------
    // STATE EVOLUTION (SINGLE SCALAR UPDATE)
    //--------------------------------------------------------------------------

    let next_v = State::normalize(state.v() + input);
    state.set_v(next_v);

    //--------------------------------------------------------------------------
    // MEMORY UPDATE (FIFO TRACE)
    //--------------------------------------------------------------------------

    state.push_fifo(next_v);
}

//==============================================================================
// 5. SYSTEM DESCRIPTION (CONTRACTUAL ONLY)
//==============================================================================

#[inline(always)]
pub fn classify() -> &'static str {
    "Closed deterministic bounded-memory scalar recurrence system with FIFO history and lossy observation projection"
}

//==============================================================================
// 6. FORMAL INVARIANTS (IMPLEMENTATION-ALIGNED)
//==============================================================================
//
// 1. State closure:
//      S = (v, H, cap)
//
// 2. Scalar invariant:
//      v ∈ [0,1)
//
// 3. Memory invariant:
//      len(H) ≤ cap (enforced via FIFO truncation)
//
// 4. Determinism:
//      identical (S_t, u_t) → identical S_{t+1}
//
// 5. Observation inertness:
//      observe(v) does not affect S
//
// 6. Representation completeness:
//      no state exists outside (v, H, cap)
//
//==============================================================================
//
// END FILE
//==============================================================================
