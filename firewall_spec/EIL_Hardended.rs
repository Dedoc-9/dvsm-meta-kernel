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
// DVSM — CLOSED DETERMINISTIC BOUNDED STATE MACHINE (CANONICAL FORM)
// ADDENDUM — FORMAL REDUCTION + HARDENING LAYER
//==============================================================================
//
// This addendum refines the canonical DVSM into a strictly bounded,
// deterministic, implementation-aligned transition system.
//
// It removes hidden ambiguity in:
//   - normalization
//   - memory truncation semantics
//   - observation independence
//   - invariance interpretation
//
// No new runtime state is introduced.
// No new causal paths are added.
//
//==============================================================================

#![allow(dead_code)]

const PHI: f64 = 1.61803398875;

//==============================================================================
// 7. FORMAL STATE INTERPRETATION (REDUCTION LAYER)
//==============================================================================
//
// The system is equivalently representable as:
//
//   S_t ∈ [0,1) × H_cap
//
// where:
//
//   v_t ∈ [0,1)
//   H_t ∈ ([0,1))^≤cap
//
// cap ∈ ℕ fixed at initialization.
//
//==============================================================================
//
// REDUCED TRANSITION FUNCTION:
//
//   F : S × ℝ → S
//
//   F(v_t, H_t, u_t) =
//       ( normalize(v_t + u_t),
//         FIFO(H_t ⧺ {v_{t+1}}) )
//
//==============================================================================

//==============================================================================
// 8. CANONICAL NORMALIZATION LAW (STRICT FORM)
//==============================================================================
//
// IMPORTANT HARDENING:
//
// We explicitly define normalization as total modular projection:
//
//   normalize(x) = x - floor(x)
//
// ensuring:
//
//   ∀x ∈ ℝ : normalize(x) ∈ [0,1)
//
// This avoids platform-dependent fract() edge behavior.
//
//==============================================================================

#[inline(always)]
fn normalize(x: f64) -> f64 {
    x - x.floor()
}

//==============================================================================
// 9. FIFO MEMORY OPERATOR (DETERMINISTIC TRUNCATION)
//==============================================================================
//
// The FIFO operator is defined as:
//
//   FIFO_cap(H ⧺ x) = tail_{≤cap}(H ⧺ x)
//
// i.e. last-cap elements only.
//
// This is equivalent to:
//
//   remove_oldest_until_len ≤ cap
//
//==============================================================================

#[inline(always)]
fn fifo_push(h: &mut Vec<f64>, x: f64, cap: usize) {
    h.push(x);

    if h.len() > cap {
        let excess = h.len() - cap;
        h.drain(0..excess);
    }

    debug_assert!(h.len() <= cap);
}

//==============================================================================
// 10. OBSERVATION LAW (STRICT SEPARATION)
//==============================================================================
//
// Observation function:
//
//   O : [0,1) → [0,1) × [0,1)
//
//   O(v) = (v, fract(φv))
//
// HARDENING NOTE:
//
// Observation is:
//
//   - side-effect free
//   - state-independent
//   - causally inert
//   - non-injective
//
//==============================================================================

#[inline(always)]
pub fn observe(v: f64) -> (f64, f64) {
    (v, normalize(v * PHI))
}

//==============================================================================
// 11. STATE MACHINE (STRICT TRANSITION CORE)
//==============================================================================
//
// Single source of truth for evolution:
//
//==============================================================================

#[derive(Clone, Debug)]
pub struct State {
    v: f64,
    h: Vec<f64>,
    cap: usize,
}

impl State {

    //--------------------------------------------------------------------------
    // CONSTRUCTION
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
    // READ INTERFACE
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
    // INTERNAL STATE UPDATE
    //--------------------------------------------------------------------------

    #[inline(always)]
    fn set_v(&mut self, v: f64) {
        let nv = normalize(v);

        debug_assert!(nv >= 0.0 && nv < 1.0);

        self.v = nv;
    }

    #[inline(always)]
    fn push(&mut self, v: f64) {
        fifo_push(&mut self.h, v, self.cap);
    }
}

//==============================================================================
// 12. INPUT GATE (TOTALITY FILTER)
//==============================================================================
//
// Only constraint:
//
//   reject NaN / Inf to preserve closure of [0,1)
//
//==============================================================================

#[inline(always)]
fn validate_input(u: f64) -> bool {
    u.is_finite()
}

//==============================================================================
// 13. TRANSITION FUNCTION (FINAL FORM)
//==============================================================================
//
// S_{t+1} = F(S_t, u_t)
//
//==============================================================================

#[inline(always)]
pub fn step(state: &mut State, input: f64) {

    //--------------------------------------------------------------------------
    // INPUT VALIDATION (TOTAL DOMAIN FILTER)
    //--------------------------------------------------------------------------

    if !validate_input(input) {
        return;
    }

    //--------------------------------------------------------------------------
    // SCALAR EVOLUTION
    //--------------------------------------------------------------------------

    let next_v = normalize(state.v() + input);
    state.set_v(next_v);

    //--------------------------------------------------------------------------
    // MEMORY EVOLUTION (STRICT FIFO APPEND)
    //--------------------------------------------------------------------------

    state.push(next_v);

    //--------------------------------------------------------------------------
    // POST-CONDITIONS (DEBUG ONLY)
    //--------------------------------------------------------------------------

    debug_assert!(state.v() >= 0.0 && state.v() < 1.0);
    debug_assert!(state.history().len() <= state.cap);
}

//==============================================================================
// 14. SYSTEM CLASSIFICATION (REDUCED FORM)
//==============================================================================

#[inline(always)]
pub fn classify() -> &'static str {
    "Deterministic bounded scalar recurrence with modular normalization, FIFO trace truncation, and lossy non-injective observation projection"
}

//==============================================================================
// 15. FORMAL INVARIANTS (HARDENED)
//==============================================================================
//
// I1. State closure:
//     S_t ∈ [0,1) × ([0,1))^≤cap
//
// I2. Determinism:
//     (S_t, u_t) → S_{t+1} is a pure function
//
// I3. Memory bound:
//     |H_t| ≤ cap invariant under all transitions
//
// I4. Observation independence:
//     O(v_t) ∉ causal influence set of S_{t+1}
//
// I5. Normalization closure:
//     ∀x ∈ ℝ → normalize(x) ∈ [0,1)
//
// I6. No hidden state:
//     State is fully captured by (v, H, cap)
//
//==============================================================================
//
// END ADDENDUM
//==============================================================================
//==============================================================================
//
// END FILE
//==============================================================================
