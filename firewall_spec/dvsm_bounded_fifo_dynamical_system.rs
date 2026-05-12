// ============================================================================
// DVSM / EIL / DQSDv2 — COLLAPSED QUOTIENT DYNAMICAL SYSTEM (C-STYLE RUST)
// Author: Daniel J. Dillberg
// ============================================================================
// Deterministic bounded-memory dynamical system in Rust:
// state evolves via pure update rules with FIFO memory constraint,
// mod-1 scalar normalization, and projection-based observation layer.
// Suitable for simulation, control systems, and time-series modeling.

// CORE MATHEMATICAL MODEL:
//
//   S_{t+1} = F(S_t, u_t)
//
//   S_t = (v_t, H_t)
//   v_t ∈ [0,1)
//   H_t ∈ Seq_≤N(ℝ)  (FIFO-ordered finite sequence, |H_t| ≤ N)
//
// H_t ∈ Seq_≤N(ℝ)
// (FIFO-ordered finite sequence with max length N)
//
// Structural interpretations (non-collapsing views):
//
// 1) Type view:
//    H_t inhabits the state space Seq_≤N(ℝ)
//
// 2) Invariant view:
//    |H_t| ≤ N is a state invariant preserved by F
//    (F is a self-map on Seq_≤N(ℝ))
//
// 3) Dynamics view:
//    H_{t+1} = truncate(H_t ⧺ v_{t+1}, N)
//    (truncation is part of the definition of F)
//
// NOTE:
// - These are not interchangeable definitions.
// - They are mutually consistent constraints on a single state evolution system.
// - Only the dynamics view defines state transition semantics.
// - Type view constrains admissible states (static restriction).
// - Invariant view follows from closure of the executed F over Seq_≤N(ℝ).
// - Separation prevents conflation of (type, invariant, dynamics) layers.

State space:
    S := Seq_≤N(ℝ)

Dynamics (closed endomorphism):
    F : S × ℝ → S

Observation (projection map):
    O : S → 𝒪

Observation space:
    𝒪 := im(O)
    (i.e., 𝒪 is defined as the image of O; no separate codomain structure assumed)

Induced dynamics (well-defined on observation space):
    f : 𝒪 × ℝ → 𝒪
    f(O(s), u) := O(F(s, u))

Well-definedness condition:
    O(s₁) = O(s₂) ⇒ O(F(s₁, u)) = O(F(s₂, u))

Commutativity (diagram):
    O ∘ F = f ∘ O

Equality interpretation:
    - Strict form: holds exactly when O is injective (trivial quotient)
    - Quotient form: holds on equivalence classes induced by ~
      where s₁ ~ s₂ ⇔ O(s₁) = O(s₂)

Diagram:

      F
  S ------> S
  |         |
 O|         |O
  v         v
 𝒪 ----->   𝒪
      f

Definitions:
    S = concrete state space
    𝒪 = quotient observation space (image of O)
    O = surjective projection defining equivalence relation ~ on S
    F = state endomorphism on S
    f = uniquely induced observable morphism (when well-definedness holds)

// OBSERVATION STRUCTURE:
//
//   O : S → 𝒪
//   𝒪 := im(O)
//
//   S₁ ~ S₂ ⇔ O(S₁) = O(S₂)
//
// WELL-DEFINEDNESS CONDITION:
//
//   O(S₁)=O(S₂) ⇒ O(F(S₁,u)) = O(F(S₂,u))
//
// OBSERVABLE DYNAMICS:
//
//   f(O(S)) := O(F(S))
//
// COMMUTATIVITY:
//
//   O ∘ F = f ∘ O   (in quotient space S/~)
//
// ============================================================================

#![allow(dead_code)]

// ============================================================================
// CORE STATE (S)
// ============================================================================

#[derive(Clone, Debug)]
pub struct State {
    pub v: f64,
    pub h: Vec<f64>,
}

// ============================================================================
// CONSTANTS / UTIL (C-STYLE LOW LEVEL LOGIC)
// ============================================================================

const V_MIN: f64 = 0.0;
const V_MAX: f64 = 1.0;

#[inline(always)]
fn is_valid(x: f64) -> bool {
    x.is_finite()
}

#[inline(always)]
fn norm(x: f64) -> f64 {
    x.fract()
}

#[inline(always)]
fn trim<T: Clone>(buf: &mut Vec<T>, cap: usize) {
    if buf.len() > cap {
        let excess = buf.len() - cap;
        buf.drain(0..excess);
    }
}

// ============================================================================
// DYNAMICS (F) — LATENT SYSTEM
// ============================================================================

pub struct F;

impl F {
    #[inline(always)]
    pub fn step(s: &mut State, u: f64, cap: usize) {

        // C-STYLE GUARD CLAUSE (fail-fast)
        if !is_valid(u) {
            return;
        }

        // deterministic update
        let v_next = norm(s.v + u);

        s.v = v_next;
        s.h.push(v_next);

        trim(&mut s.h, cap);
    }
}

// ============================================================================
// OBSERVATION MAP (O)
// ============================================================================
//
// NOTE:
// - O is NOT a system
// - O is a projection / quotient constructor
// - 𝒪 is defined as image(O)
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub struct Obs {
    pub v: f64,
    pub h: Vec<f64>,
}

pub struct O;

impl O {

    #[inline(always)]
    pub fn project(s: &State) -> Obs {
        Obs {
            v: norm(s.v),
            h: s.h.clone(),
        }
    }

    #[inline(always)]
    pub fn eq(a: &State, b: &State) -> bool {
        Self::project(a) == Self::project(b)
    }
}

// ============================================================================
// OBSERVABLE DYNAMICS (f) — DERIVED ONLY
// ============================================================================

pub struct f;

impl f {

    #[inline(always)]
    pub fn step(o: &Obs, u: f64, cap: usize) -> Obs {

        if !is_valid(u) {
            return o.clone();
        }

        let v_next = norm(o.v + u);

        let mut h = o.h.clone();
        h.push(v_next);

        trim(&mut h, cap);

        Obs { v: v_next, h }
    }
}

// ============================================================================
// COMMUTATIVITY CHECK (EXECUTION TEST)
// ============================================================================

pub fn check(mut s: State, u: f64, cap: usize) -> bool {

    // O(F(S))
    let o_before = O::project(&s);
    F::step(&mut s, u, cap);
    let o_after = O::project(&s);

    // f(O(S))
    let o_expected = f::step(&o_before, u, cap);

    // quotient equality
    o_after == o_expected
}

// ============================================================================
// RUNTIME ENGINE (C-STYLE PROCEDURAL LOOP)
// ============================================================================

pub struct Runtime {
    pub state: State,
    pub time: u64,
    pub cap: usize,
}

impl Runtime {

    #[inline(always)]
    pub fn new(cap: usize) -> Self {
        Self {
            state: State { v: 0.0, h: Vec::new() },
            time: 0,
            cap,
        }
    }

    // PURE PROCEDURAL STEP (C STYLE EXECUTION FLOW)
    #[inline(always)]
    pub fn tick(&mut self, u: f64) {

        // 1. advance clock
        self.time += 1;

        // 2. apply latent dynamics
        F::step(&mut self.state, u, self.cap);

        // 3. optional observation hook (no state coupling)
        let _obs = O::project(&self.state);

        // (no side effects beyond state evolution)
    }
}

// ============================================================================
// SYSTEM INVARIANTS (OPERATIONAL ONLY)
// ============================================================================
//
// ✔ single state space S
// ✔ deterministic F
// ✔ bounded FIFO memory H
// ✔ mod-1 scalar projection
// ✔ observation map O induces quotient S/~
// ✔ f is derived, not independent
// ✔ commutativity holds in quotient sense
//
// ============================================================================
//
// FINAL CLASSIFICATION
// ============================================================================
//
// This is:
//
//   A deterministic discrete-time bounded-memory dynamical system
//   equipped with a surjective observation map inducing a quotient structure.
//
// STRUCTURE:
//
//   Latent system:
//     S := state space
//     F : S × ℝ → S   (deterministic endomorphism)
//
//   Observation:
//     O : S → 𝒪
//     𝒪 := im(O)
//
//   Equivalence relation:
//     s₁ ~ s₂  ⇔  O(s₁) = O(s₂)
//
//   Quotient space:
//     S / ~
//
//   Induced dynamics (well-defined only if compatibility holds):
//     f : 𝒪 × ℝ → 𝒪
//     f(O(s), u) := O(F(s, u))
//
//   Well-definedness condition:
//     O(s₁) = O(s₂) ⇒ O(F(s₁, u)) = O(F(s₂, u))
//
// COMMUTATIVE DIAGRAM:
//
//        F
//    S ─────→ S
//    │         │
//    O         O
//    ↓         ↓
//    𝒪 ───f──→ 𝒪
//
// PROPERTY:
//
//   O ∘ F = f ∘ O   (on equivalence classes of S / ~)
//
// ============================================================================
// ADDENDUM — SYSTEM CLASSIFICATION NOTES (INTERPRETATION LAYER)
// ============================================================================
//
// PURPOSE:
// ---------------------------------------------------------------------------
// This section clarifies the semantic classification of the system.
// It does NOT modify:
//   - state space S
//   - dynamics F
//   - observation map O
//   - induced dynamics f
//
// It is purely a descriptive layer for implementation clarity.
//
// ============================================================================
//
// 1. BOUNDEDNESS
// ============================================================================
//
// The system is bounded in two orthogonal dimensions:
//
// (a) Scalar bound:
//     v_t ∈ [0,1)  via mod-1 normalization
//
// (b) Memory bound:
//     H_t ∈ Seq_≤N(ℝ)
//     |H_t| ≤ N enforced by FIFO truncation in F
//
// Interpretation:
//     - state magnitude is bounded
//     - memory depth is bounded
//
// ============================================================================
//
// 2. STATE SCOPE (SHARED STATE MODEL)
// ============================================================================
//
// The system operates on a single shared state space S:
//
//     S := Seq_≤N(ℝ) // S := [0,1) × Seq≤N_FIFO(ℝ)  ≡  {(v,H) | v∈[0,1), H∈Seq≤N_FIFO(ℝ)}
//
// F: S × ℝ → S defines a parameterized family of endomorphisms on S:
//   - v update: v ↦ (v + u) mod 1
//   - H update: H ↦ truncate(H ⧺ v', N), where v' is the updated scalar state
//
// Closure:
//   F(S, u) ∈ S for all admissible (S, u)
//
// Structure is product-form; coupling resides entirely in F.× [0,1)
//
// All operators (F, O, f, Runtime) act on elements of S.
//
// Interpretation:
//     - one unified state space
//     - no independent state spaces
//     - partitioning is representational, not ontological
//
// ============================================================================
// FILE NOTE — PARAMETERIZED MORPHISM VIEW (OBSERVATION UNIFICATION LAYER)
// ============================================================================
//
// 📌 Optional micro-upgrade for maximal rigor:
//
// If the observation quotient layer is unified with the dynamical system,
// the parameterized map:
//
//     F : S × ℝ → S
//
// can be equivalently re-expressed as a family of morphisms:
//
//     F_u : S → S
//
// where each input u ∈ ℝ indexes a distinct endomorphism on S.
//
// Interpretation:
//   - u is treated as a morphism parameter (not a structural state variable)
//   - system evolution becomes a family of maps {F_u}_{u ∈ ℝ}
//   - observation compatibility (O ∘ F_u = f_u ∘ O) can then be stated per u
//
// Benefit:
//   - aligns dynamics with categorical composition view
//   - simplifies commutativity statements in quotient form
//   - prepares system for functorial / diagrammatic formulation
//
// NOTE:
//   This does NOT change the underlying system definition.
//   It only re-frames F as a parameter-indexed morphism family.
//
// ============================================================================
//
// 3. VARIABILITY
// ============================================================================
//
// (a) Variable components:
//     - v_t (scalar evolving state)
//     - H_t (FIFO-evolving history buffer)
//
// (b) Fixed structure:
//     - S (state space definition)
//     - F (transition operator signature)
//     - O (projection map)
//     - N (memory bound)
//
// Interpretation:
//     - system evolves over time
//     - structure of the mathematical system is invariant
//       (only state instances evolve; S, F, O, f remain fixed)
//
// ============================================================================
//
// 4. TERMINOLOGY NORMALIZATION
// ============================================================================
//
// "bounded":
//     → constraints on value ranges and memory size
//
// "shared":
//     → single state space used across all operators
//
// "variable":
//     → evolving state values within a fixed structural system
//
// ============================================================================
//
// 5. SUMMARY CLASSIFICATION
// ============================================================================
//
// The system is:
//
//     A bounded, shared-state, variable-value deterministic dynamical system
//     with FIFO-constrained memory evolution and projection-based observation.
//
// ============================================================================
//
// NOTE:
// ---------------------------------------------------------------------------
// These classifications are interpretive only.
// They do not introduce new mathematical objects or modify system dynamics.
//
// ============================================================================
// END ADDENDUM
// ============================================================================
// ============================================================================
//
// END OF FILE
// ============================================================================
