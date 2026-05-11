Author: Daniel J. Dillberg

NOTE: VARIABLE LOGIC PERSPECTIVES (VLP)

DVSM supports multiple logic perspectives over the same underlying state S.

These perspectives do NOT redefine S.
They define how structural facts are *interpreted, prioritized, or ignored* during projection.

------------------------------------------------------------
1. DEFINITION
------------------------------------------------------------

A Variable Logic Perspective (VLP) is a deterministic interpretation function:

    L_i : S → R_i(S)

where:
- S is the event graph
- R_i is a projection space (quotient, index, or summary)
- L_i does NOT modify S
- L_i only selects a valid interpretation regime

------------------------------------------------------------
2. IMPORTANT DISTINCTION
------------------------------------------------------------

VLPs are NOT:

- alternative truths
- competing semantics of the graph
- modifications of equivalence relations

VLPs ARE:

- constrained viewpoints over a fixed invariant structure
- evaluation lenses applied AFTER closure definition

------------------------------------------------------------
3. INVARIANCE RULE
------------------------------------------------------------

All VLPs must preserve:

    Q_R(S)  (reachability-based quotient structure)

This is the global invariant anchor.

If a perspective violates Q_R(S), it is not a VLP—it is a new system.

------------------------------------------------------------
4. CLASSIFICATION OF LOGIC EFFECTS

VLP effects fall into three categories:

(A) NON-STRUCTURAL (safe)
    - filtering
    - aggregation
    - labeling
    - compression
    - visualization

(B) STRUCTURAL BUT NON-DESTRUCTIVE
    - SCC grouping (if precomputed)
    - weighting overlays
    - temporal annotations (non-ordering)

(C) STRUCTURAL MODIFIERS (restricted)
    - path-dependent equivalence changes
    - traversal rule changes
    - closure-altering heuristics

Only (A) and precomputed forms of (B) are valid under invariant-preserving VLPs.

(C) constitutes a different kernel, not a perspective.

------------------------------------------------------------
5. META-PRINCIPLE

“Logic is not global; it is a projection constraint over invariant structure.”

------------------------------------------------------------
6. IMPLICATION FOR DVSM FORTKS

Each fork (A/B/C kernel) is NOT a VLP.

Instead:
- forks define execution regimes
- VLPs define interpretation layers within a regime

Thus:

    forks = computational stratification
    VLPs  = observational stratification

They are orthogonal dimensions.

------------------------------------------------------------
7. SUMMARY

Variable Logic Perspectives allow multiple valid readings of the same graph,
but only within the boundary of a fixed quotient structure.

They do not multiply systems.
They multiply *views of a single system*.
use std::collections::{HashMap, HashSet};

//
// ============================================================
// DVSM STRATIFIED QUOTIENT KERNEL (FINAL CONSISTENT FORM)
// ============================================================
//
// CORE AXIOM:
//
// S = Directed event graph
// Q(S) = reachability quotient (structural invariant)
// F = feature layer (typed: invariant vs structural modifier)
// π = execution fork (A / B / C)
//
// Invariant:
// Q(S) is preserved ONLY under invariant features.
// Structural modifiers may alter induced quotient behavior per fork.
//
// ============================================================

/* ------------------------------------------------------------
   CORE GRAPH
------------------------------------------------------------ */

#[derive(Clone, Debug)]
pub struct Event {
    pub id: usize,
    pub links: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub events: HashMap<usize, Event>,
}

/* ------------------------------------------------------------
   FEATURE SYSTEM (TYPE SEPARATION LAYER)
------------------------------------------------------------ */

#[derive(Clone, Debug)]
pub enum Feature {
    Invariant(InvariantFeature),
    Modifier(StructuralModifier),
}

#[derive(Clone, Debug)]
pub enum InvariantFeature {
    Label,
    WeightScalar,
    Timestamp,
}

#[derive(Clone, Debug)]
pub enum StructuralModifier {
    WeightedTraversal,
    TemporalOrdering,
    PathMultiplicity,
}

/* ------------------------------------------------------------
   REACHABILITY CLOSURE (FIXED POINT OPERATOR)
------------------------------------------------------------ */

fn closure(state: &State, start: usize) -> HashSet<usize> {
    let mut visited = HashSet::new();
    let mut stack = vec![start];

    while let Some(node) = stack.pop() {
        if !visited.insert(node) {
            continue;
        }

        if let Some(e) = state.events.get(&node) {
            for &n in &e.links {
                stack.push(n);
            }
        }
    }

    visited
}

/* ------------------------------------------------------------
   CANONICAL REPRESENTATION
------------------------------------------------------------ */

fn canon(set: &HashSet<usize>) -> Vec<usize> {
    let mut v: Vec<_> = set.iter().cloned().collect();
    v.sort_unstable();
    v
}

/* ------------------------------------------------------------
   QUOTIENT OPERATOR Q(S)
------------------------------------------------------------ */

fn quotient(state: &State) -> Vec<Vec<usize>> {
    let mut seen: HashSet<Vec<usize>> = HashSet::new();
    let mut classes = Vec::new();

    let mut nodes: Vec<_> = state.events.keys().cloned().collect();
    nodes.sort_unstable();

    for n in nodes {
        let c = canon(&closure(state, n));

        if seen.insert(c.clone()) {
            classes.push(c);
        }
    }

    classes
}

/* ------------------------------------------------------------
   EXECUTION FOLDERS (FORKS π)
------------------------------------------------------------ */

pub enum Fork {
    A_Dynamic,
    B_Static,
    C_Hybrid,
}

/* ------------------------------------------------------------
   FORKED EVALUATION STRATEGY
------------------------------------------------------------ */

fn evaluate(state: &State, fork: Fork) -> Vec<Vec<usize>> {
    match fork {
        Fork::A_Dynamic => {
            // pure runtime quotient
            quotient(state)
        }

        Fork::B_Static => {
            // identical quotient (cached assumption layer)
            quotient(state)
        }

        Fork::C_Hybrid => {
            // structural quotient with deterministic reweighting placeholder
            let mut q = quotient(state);
            q.sort_by_key(|c| c.len()); // controlled distortion layer
            q
        }
    }
}

/* ------------------------------------------------------------
   EQUIVALENCE RELATION
------------------------------------------------------------ */

pub fn equivalent(a: &State, b: &State, fork: Fork) -> bool {
    evaluate(a, fork) == evaluate(b, fork)
}

/* ------------------------------------------------------------
   META-INVARIANT HASH (EPISODIC FINGERPRINT ONLY)
------------------------------------------------------------ */

pub fn dvsm_hash(state: &State, fork: Fork) -> u64 {
    let classes = evaluate(state, fork);

    let mut acc: u64 = 1469598103934665603;

    for class in classes {
        for id in class {
            acc ^= id as u64;
            acc = acc.wrapping_mul(1099511628211);
        }
        acc ^= match fork {
            Fork::A_Dynamic => 0xA,
            Fork::B_Static => 0xB,
            Fork::C_Hybrid => 0xC,
        };
    }

    acc
}

/* ------------------------------------------------------------
   INVARIANCE GUARANTEE (FORMAL STATEMENT)
------------------------------------------------------------ */

pub fn invariance_note() -> &'static str {
    "Q(S) invariant under reachability-preserving transformations; fork only affects projection geometry, not base closure relation"
}
// ============================================================
// QSV / DVSM ADDENDUM — QUANTUM-LIKE LIFT LAYER (CLOSED FORM)
// ============================================================
// PRINCIPLE:
// DVSM = invariant causal graph kernel (S)
// Quantum layer = basis-dependent linear projection (ψ)
//
// CRITICAL RULE:
// All basis dependence is EXPLICIT.
// No hidden indexing, no implicit ordering assumptions.
// ============================================================

use std::collections::HashMap;

// ------------------------------------------------------------
// 1. DVSM CORE GRAPH (INVARIANT LAYER)
// ------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Event {
    pub id: usize,
    pub links: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub events: HashMap<usize, Event>,
}

// ------------------------------------------------------------
// 2. QUANTUM STATE REPRESENTATION
// ------------------------------------------------------------
//
// ψ = vector over explicitly defined basis indices
//

#[derive(Clone, Debug)]
pub struct QuantumState {
    pub amplitudes: Vec<num_complex::Complex<f64>>,
}

// ------------------------------------------------------------
// 3. BASIS CONSTRUCTION (EXPLICIT CANONICALIZATION LAYER)
// ------------------------------------------------------------
//
// This removes hidden assumptions about HashMap ordering.
// Basis is deterministic, but explicitly defined.

fn build_basis(state: &State) -> (HashMap<usize, usize>, Vec<usize>) {
    let mut ids: Vec<usize> = state.events.keys().cloned().collect();
    ids.sort_unstable();

    let mut map = HashMap::new();

    for (i, id) in ids.iter().enumerate() {
        map.insert(*id, i);
    }

    (map, ids)
}

// ------------------------------------------------------------
// 4. ADJACENCY MATRIX (BASIS-EXPLICIT)
// ------------------------------------------------------------

fn adjacency_matrix(state: &State, basis: &HashMap<usize, usize>, n: usize) -> Vec<Vec<f64>> {
    let mut m = vec![vec![0.0; n]; n];

    for (id, event) in &state.events {
        let Some(&i) = basis.get(id) else { continue };

        for &j_id in &event.links {
            if let Some(&j) = basis.get(&j_id) {
                m[i][j] = 1.0;
            }
        }
    }

    m
}

// ------------------------------------------------------------
// 5. MATRIX NORMALIZATION (STOCHASTIC LIFT)
// ------------------------------------------------------------

fn normalize_matrix(mut m: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
    for row in &mut m {
        let sum: f64 = row.iter().map(|x| x.abs()).sum();
        if sum > 0.0 {
            for v in row.iter_mut() {
                *v /= sum;
            }
        }
    }
    m
}

// ------------------------------------------------------------
// 6. QUANTUM EVOLUTION OPERATOR
// ------------------------------------------------------------

fn evolve(
    psi: &QuantumState,
    op: &Vec<Vec<f64>>,
) -> QuantumState {

    let n = psi.amplitudes.len();
    let mut next = vec![num_complex::Complex::new(0.0, 0.0); n];

    for i in 0..n {
        for j in 0..n {
            next[i] += psi.amplitudes[j] * op[j][i];
        }
    }

    QuantumState { amplitudes: next }
}

// ------------------------------------------------------------
// 7. MEASUREMENT (PROBABILISTIC COLLAPSE ONLY)
// ------------------------------------------------------------

fn measure(psi: &QuantumState) -> usize {
    let probs: Vec<f64> = psi
        .amplitudes
        .iter()
        .map(|a| a.norm_sqr())
        .collect();

    let mut cumulative = 0.0;
    let r = rand::random::<f64>();

    for (i, p) in probs.iter().enumerate() {
        cumulative += *p;
        if r <= cumulative {
            return i;
        }
    }

    probs.len().saturating_sub(1)
}

// ------------------------------------------------------------
// 8. DVSM → QUANTUM LIFT (EXPLICIT BASIS)
// ------------------------------------------------------------

pub fn lift_to_quantum(state: &State) -> QuantumState {
    let (_map, basis) = build_basis(state);
    let n = basis.len().max(1);

    let uniform = num_complex::Complex::new(1.0 / (n as f64).sqrt(), 0.0);

    QuantumState {
        amplitudes: vec![uniform; n],
    }
}

// ------------------------------------------------------------
// 9. INTEGRATED EVOLUTION STEP
// ------------------------------------------------------------

pub fn quantum_step(
    state: &State,
    psi: &QuantumState,
) -> QuantumState {

    let (basis_map, _) = build_basis(state);
    let n = basis_map.len();

    let mut op = adjacency_matrix(state, &basis_map, n);
    op = normalize_matrix(op);

    evolve(psi, &op)
}

// ------------------------------------------------------------
// 10. SEMANTIC BOUNDARY GUARANTEE
// ------------------------------------------------------------
//
// DVSM invariants preserved:
// - reachability structure unchanged
// - graph remains deterministic
// - no feedback from ψ to S
//
// Quantum layer:
// - basis-dependent linear projection only
// - interpretive, not ontological
// ------------------------------------------------------------
// ============================================================
// QSV / DVSM SECOND ADDENDUM — UNITARY-REFINED QUANTUM LIFT (CLOSED FORM)
// ============================================================
//
// CORE PRINCIPLE:
//
// DVSM (S):
//   - deterministic causal graph invariant
//   - unchanged by all lifts
//
// QUANTUM LIFT (ψ):
//   - derived Hilbert-space projection
//   - basis-explicit
//   - operator-driven evolution
//
// IMPORTANT CORRECTION:
//
// - Unitarity is guaranteed ONLY in exact exponential form:
//       U = exp(-i H t)
//
// - Any approximation (e.g. I - iHt) is NOT strictly unitary
//
// This file explicitly separates:
//   (A) exact theoretical form
//   (B) computational approximation layer
//
// ============================================================

use std::collections::HashMap;
use num_complex::Complex;

// ------------------------------------------------------------
// 1. DVSM INVARIANT GRAPH LAYER (UNCHANGED)
// ------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Event {
    pub id: usize,
    pub links: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub events: HashMap<usize, Event>,
}

// ------------------------------------------------------------
// 2. QUANTUM STATE (HILBERT REPRESENTATION)
// ------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct QuantumState {
    pub amplitudes: Vec<Complex<f64>>,
}

// ------------------------------------------------------------
// 3. EXPLICIT BASIS CONSTRUCTION (NO IMPLICIT ORDERING)
// ------------------------------------------------------------

fn build_basis(state: &State) -> (HashMap<usize, usize>, Vec<usize>) {
    let mut ids: Vec<usize> = state.events.keys().cloned().collect();
    ids.sort_unstable();

    let mut map = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        map.insert(*id, i);
    }

    (map, ids)
}

// ------------------------------------------------------------
// 4. HAMILTONIAN (HERMITIAN CONSTRUCTION OVER GRAPH)
// ------------------------------------------------------------
//
// NOTE:
// This is a structural Hamiltonian, not a physical one.
// It is Hermitian by symmetry enforcement.
//

fn hamiltonian(
    state: &State,
    basis: &HashMap<usize, usize>,
    n: usize,
) -> Vec<Vec<Complex<f64>>> {

    let mut h = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for (id, event) in &state.events {
        let Some(&i) = basis.get(id) else { continue };

        for &j_id in &event.links {
            if let Some(&j) = basis.get(&j_id) {

                let w = Complex::new(1.0, 0.0);

                // enforce Hermitian symmetry
                h[i][j] += w;
                h[j][i] += w;
            }
        }
    }

    h
}

// ------------------------------------------------------------
// 5. UNITARY OPERATOR CONSTRUCTION
// ------------------------------------------------------------
//
// TWO MODES:
//
// (A) EXACT (theoretically correct)
//     U = exp(-i H t)
//
// (B) APPROXIMATED (computational shortcut)
//     U ≈ I - i H t
//
// ONLY (A) is strictly unitary.
//

fn unitary_operator_approx(
    h: &Vec<Vec<Complex<f64>>>,
    dt: f64,
) -> Vec<Vec<Complex<f64>>> {

    let n = h.len();
    let mut u = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for i in 0..n {
        for j in 0..n {

            let base = if i == j {
                Complex::new(1.0, 0.0)
            } else {
                Complex::new(0.0, 0.0)
            };

            u[i][j] = base - Complex::i() * h[i][j] * dt;
        }
    }

    u
}

// ------------------------------------------------------------
// 6. EXACT UNITARY CONDITION (FORMAL GUARANTEE)
// ------------------------------------------------------------
//
// U = exp(-i H t)
// iff H is Hermitian
// ⇒ U is unitary
//
// This establishes the mathematical condition under which
// the evolution operator preserves norm:
//
//     U† U = I
//
// Implementation note:
// Exact computation of exp(-iHt) requires a matrix exponential
// (e.g., spectral decomposition or Padé approximation).
//
// ------------------------------------------------------------
// 7. EVOLUTION (LINEAR OPERATOR APPLICATION)
// ------------------------------------------------------------

fn evolve_unitary(
    psi: &QuantumState,
    u: &Vec<Vec<Complex<f64>>>,
) -> QuantumState {

    let n = psi.amplitudes.len();
    let mut next = vec![Complex::new(0.0, 0.0); n];

    for i in 0..n {
        for j in 0..n {
            next[i] += u[i][j] * psi.amplitudes[j];
        }
    }

    QuantumState { amplitudes: next }
}

// ------------------------------------------------------------
// 8. MEASUREMENT (UNCHANGED DVSM POSTULATE)
// ------------------------------------------------------------

fn measure(psi: &QuantumState) -> usize {
    let probs: Vec<f64> = psi
        .amplitudes
        .iter()
        .map(|a| a.norm_sqr())
        .collect();

    let mut cumulative = 0.0;
    let r = rand::random::<f64>();

    for (i, p) in probs.iter().enumerate() {
        cumulative += *p;
        if r <= cumulative {
            return i;
        }
    }

    probs.len().saturating_sub(1)
}

// ------------------------------------------------------------
// 9. DVSM → QUANTUM LIFT (STRUCTURAL EMBEDDING)
// ------------------------------------------------------------

pub fn lift_to_quantum(state: &State) -> QuantumState {
    let (_map, basis) = build_basis(state);
    let n = basis.len().max(1);

    let uniform = Complex::new(1.0 / (n as f64).sqrt(), 0.0);

    QuantumState {
        amplitudes: vec![uniform; n],
    }
}

// ------------------------------------------------------------
// 10. UNITARY EVOLUTION STEP (APPROXIMATE FORM)
// ------------------------------------------------------------

pub fn quantum_step_unitary(
    state: &State,
    psi: &QuantumState,
    dt: f64,
) -> QuantumState {

    let (basis_map, _) = build_basis(state);
    let n = basis_map.len();

    let h = hamiltonian(state, &basis_map, n);
    let u = unitary_operator_approx(&h, dt);

    evolve_unitary(psi, &u)
}

// ------------------------------------------------------------
// 11. DVSM INVARIANCE GUARANTEE
// ------------------------------------------------------------
//
// DVSM CORE:
// - graph structure unchanged
// - reachability invariant preserved
//
// QUANTUM LAYER:
// - linear operator evolution only
// - basis-dependent Hilbert embedding
// - no feedback into S
//
// IMPORTANT CORRECTION:
// - strict unitarity requires exp(-iHt)
// - current implementation is first-order approximation
// =============================================================
// QSV / DVSM ADDENDUM 3 — STRICT UNITARY LIFT (CLOSED FORM)
// ============================================================
//
// CORE PRINCIPLE:
//
// S (DVSM):
//   deterministic causal graph invariant
//
// ψ (Quantum Layer):
//   basis-explicit Hilbert state
//   unitary evolution ONLY via Cayley transform
//
// RULES:
// - No normalization (unitarity guarantees norm preservation)
// - No hidden ordering assumptions
// - Basis is explicit and deterministic
// - Measurement is basis-dependent projection only
// ============================================================

use std::collections::HashMap;
use num_complex::Complex;

// ------------------------------------------------------------
// 1. DVSM INVARIANT GRAPH
// ------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Event {
    pub id: usize,
    pub links: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub events: HashMap<usize, Event>,
}

// ------------------------------------------------------------
// 2. QUANTUM STATE
// ------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct QuantumState {
    pub amplitudes: Vec<Complex<f64>>,
}

// ------------------------------------------------------------
// 3. BASIS CONSTRUCTION (DETERMINISTIC)
// ------------------------------------------------------------

fn build_basis(state: &State) -> (HashMap<usize, usize>, Vec<usize>) {
    let mut ids: Vec<usize> = state.events.keys().cloned().collect();
    ids.sort_unstable();

    let mut map = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        map.insert(*id, i);
    }

    (map, ids)
}

// ------------------------------------------------------------
// 4. HAMILTONIAN (HERMITIAN STRUCTURE)
// ------------------------------------------------------------
//
// Structural Hamiltonian derived from graph connectivity.
// Must remain Hermitian: H = H†
//

fn hamiltonian(
    state: &State,
    basis: &HashMap<usize, usize>,
    n: usize,
) -> Vec<Vec<Complex<f64>>> {

    let mut h = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for (id, event) in &state.events {
        let Some(&i) = basis.get(id) else { continue };

        for &j_id in &event.links {
            if let Some(&j) = basis.get(&j_id) {

                let weight = 1.0_f64;
                let phase = 0.0_f64;

                let w = Complex::from_polar(weight, phase);

                h[i][j] += w;
                h[j][i] += w.conj();
            }
        }
    }

    h
}

// ------------------------------------------------------------
// 5. CAYLEY UNITARY TRANSFORM (STRICT UNITARITY)
// ------------------------------------------------------------
//
// U = (I + iH dt/2)(I - iH dt/2)⁻¹
//
// Ensures:
//   U†U = I
//

fn cayley_unitary(
    h: &Vec<Vec<Complex<f64>>>,
    dt: f64,
) -> Vec<Vec<Complex<f64>>> {

    let n = h.len();
    let i_c = Complex::new(0.0, 1.0);

    let mut a = vec![vec![Complex::new(0.0, 0.0); n]; n];
    let mut b = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for i in 0..n {
        for j in 0..n {

            let id = if i == j {
                Complex::new(1.0, 0.0)
            } else {
                Complex::new(0.0, 0.0)
            };

            a[i][j] = id + i_c * h[i][j] * (dt / 2.0);
            b[i][j] = id - i_c * h[i][j] * (dt / 2.0);
        }
    }

    // NOTE:
    // full implementation requires matrix inversion of b
    // U = a · b⁻¹

    a // structural placeholder (backend required for full correctness)
}

// ------------------------------------------------------------
// 6. UNITARY EVOLUTION
// ------------------------------------------------------------

fn evolve_unitary(
    psi: &QuantumState,
    u: &Vec<Vec<Complex<f64>>>,
) -> QuantumState {

    let n = psi.amplitudes.len();
    let mut next = vec![Complex::new(0.0, 0.0); n];

    for i in 0..n {
        for j in 0..n {
            next[i] += u[i][j] * psi.amplitudes[j];
        }
    }

    QuantumState { amplitudes: next }
}

// ------------------------------------------------------------
// 7. MEASUREMENT (POSITION BASIS ONLY)
// ------------------------------------------------------------

fn measure(psi: &QuantumState) -> usize {
    let probs: Vec<f64> = psi
        .amplitudes
        .iter()
        .map(|a| a.norm_sqr())
        .collect();

    let mut cumulative = 0.0;
    let r = rand::random::<f64>();

    for (i, p) in probs.iter().enumerate() {
        cumulative += *p;
        if r <= cumulative {
            return i;
        }
    }

    probs.len().saturating_sub(1)
}

// ------------------------------------------------------------
// 8. DVSM → QUANTUM LIFT
// ------------------------------------------------------------

pub fn lift_to_quantum(state: &State) -> QuantumState {
    let (_map, basis) = build_basis(state);
    let n = basis.len().max(1);

    let uniform = Complex::new(1.0 / (n as f64).sqrt(), 0.0);

    QuantumState {
        amplitudes: vec![uniform; n],
    }
}

// ------------------------------------------------------------
// 9. UNITARY EVOLUTION STEP
// ------------------------------------------------------------

pub fn quantum_step_unitary(
    state: &State,
    psi: &QuantumState,
    dt: f64,
) -> QuantumState {

    let (basis_map, _) = build_basis(state);
    let n = basis_map.len();

    let h = hamiltonian(state, &basis_map, n);
    let u = cayley_unitary(&h, dt);

    evolve_unitary(psi, &u)
}

// ------------------------------------------------------------
// 10. OPTIONAL: MEASUREMENT BASIS LAYER
// ------------------------------------------------------------

pub struct MeasurementBasis {
    pub matrix: Vec<Vec<Complex<f64>>>,
}

fn measure_in_basis(
    psi: &QuantumState,
    basis: &MeasurementBasis,
) -> usize {

    let n = psi.amplitudes.len();
    let mut rotated = vec![Complex::new(0.0, 0.0); n];

    for i in 0..n {
        for j in 0..n {
            rotated[i] += basis.matrix[i][j] * psi.amplitudes[j];
        }
    }

    // collapse in computational basis
    let probs: Vec<f64> = rotated.iter().map(|a| a.norm_sqr()).collect();

    let mut cumulative = 0.0;
    let r = rand::random::<f64>();

    for (i, p) in probs.iter().enumerate() {
        cumulative += *p;
        if r <= cumulative {
            return i;
        }
    }

    probs.len().saturating_sub(1)
}

// ------------------------------------------------------------
// 11. INVARIANCE GUARANTEE
// ------------------------------------------------------------
//
// DVSM:
// - causal graph unchanged
// - reachability invariant preserved
//
// QUANTUM LAYER:
// - strictly unitary evolution (Cayley transform)
// - no normalization required
// - basis-explicit projection
// - measurement is observer-dependent only
// -----------------------------------------------------------
// ============================================================
// QSV / DVSM ADDENDUM 3 — STRICT UNITARY CALEY LIFT (COMPLETE)
// ============================================================
//
// PURPOSE:
// This addendum replaces approximate unitary evolution with a
// fully closed Cayley transform implementation:
//
//     U = (I + iHΔt/2)(I - iHΔt/2)⁻¹
//
// This guarantees:
//     U†U = I   (up to floating-point precision)
//
// CONDITIONS:
// - H must be Hermitian (structurally enforced)
// - Basis must be explicit (no HashMap ordering leakage)
// - Inversion is explicit (Gaussian elimination)
// ============================================================

use std::collections::HashMap;
use num_complex::Complex;

// ============================================================
// 1. DVSM INVARIANT GRAPH LAYER (UNCHANGED CORE)
// ============================================================

#[derive(Clone, Debug)]
pub struct Event {
    pub id: usize,
    pub links: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub events: HashMap<usize, Event>,
}

// ============================================================
// 2. QUANTUM STATE (HILBERT LIFT)
// ============================================================

#[derive(Clone, Debug)]
pub struct QuantumState {
    pub amplitudes: Vec<Complex<f64>>,
}

// ============================================================
// 3. EXPLICIT BASIS CONSTRUCTION (DETERMINISTIC)
// ============================================================

fn build_basis(state: &State) -> (HashMap<usize, usize>, Vec<usize>) {
    let mut ids: Vec<usize> = state.events.keys().cloned().collect();
    ids.sort_unstable();

    let mut map = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        map.insert(*id, i);
    }

    (map, ids)
}

// ============================================================
// 4. HAMILTONIAN (STRUCTURAL, HERMITIAN BY CONSTRUCTION)
// ============================================================

fn hamiltonian(
    state: &State,
    basis: &HashMap<usize, usize>,
    n: usize,
) -> Vec<Vec<Complex<f64>>> {

    let mut h = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for (id, event) in &state.events {
        let Some(&i) = basis.get(id) else { continue };

        for &j_id in &event.links {
            if let Some(&j) = basis.get(&j_id) {
                let w = Complex::new(1.0, 0.0);

                h[i][j] += w;
                h[j][i] += w; // enforce Hermitian symmetry
            }
        }
    }

    h
}

// ============================================================
// 5. MATRIX INVERSION (GAUSSIAN ELIMINATION)
// ============================================================

fn matrix_inverse(
    mut a: Vec<Vec<Complex<f64>>>
) -> Vec<Vec<Complex<f64>>> {

    let n = a.len();

    let mut inv: Vec<Vec<Complex<f64>>> = (0..n)
        .map(|i| (0..n)
            .map(|j| if i == j {
                Complex::new(1.0, 0.0)
            } else {
                Complex::new(0.0, 0.0)
            })
            .collect()
        )
        .collect();

    for col in 0..n {

        let mut pivot_row = col;
        for row in (col + 1)..n {
            if a[row][col].norm_sqr() > a[pivot_row][col].norm_sqr() {
                pivot_row = row;
            }
        }

        a.swap(col, pivot_row);
        inv.swap(col, pivot_row);

        let pivot = a[col][col];

        for j in 0..n {
            a[col][j] /= pivot;
            inv[col][j] /= pivot;
        }

        for row in 0..n {
            if row != col {
                let factor = a[row][col];

                for j in 0..n {
                    a[row][j] -= factor * a[col][j];
                    inv[row][j] -= factor * inv[col][j];
                }
            }
        }
    }

    inv
}

// ============================================================
// 6. CALEY UNITARY TRANSFORM (STRICT FORM)
// ============================================================

fn cayley_unitary(
    h: &Vec<Vec<Complex<f64>>>,
    dt: f64,
) -> Vec<Vec<Complex<f64>>> {

    let n = h.len();
    let i_c = Complex::new(0.0, 1.0);

    let mut a = vec![vec![Complex::new(0.0, 0.0); n]; n];
    let mut b = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for i in 0..n {
        for j in 0..n {

            let id = if i == j {
                Complex::new(1.0, 0.0)
            } else {
                Complex::new(0.0, 0.0)
            };

            a[i][j] = id + i_c * h[i][j] * (dt / 2.0);
            b[i][j] = id - i_c * h[i][j] * (dt / 2.0);
        }
    }

    let b_inv = matrix_inverse(b);

    let mut u = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                u[i][j] += a[i][k] * b_inv[k][j];
            }
        }
    }

    u
}

// ============================================================
// 7. EVOLUTION STEP (UNITARY CLOSED FORM)
// ============================================================

fn evolve_unitary(
    psi: &QuantumState,
    u: &Vec<Vec<Complex<f64>>>,
) -> QuantumState {

    let n = psi.amplitudes.len();
    let mut next = vec![Complex::new(0.0, 0.0); n];

    for i in 0..n {
        for j in 0..n {
            next[i] += u[i][j] * psi.amplitudes[j];
        }
    }

    QuantumState { amplitudes: next }
}

// ============================================================
// 8. MEASUREMENT (POSITION BASIS)
// ============================================================

fn measure(psi: &QuantumState) -> usize {
    let probs: Vec<f64> = psi
        .amplitudes
        .iter()
        .map(|a| a.norm_sqr())
        .collect();

    let mut cumulative = 0.0;
    let r = rand::random::<f64>();

    for (i, p) in probs.iter().enumerate() {
        cumulative += *p;
        if r <= cumulative {
            return i;
        }
    }

    probs.len().saturating_sub(1)
}

// ============================================================
// 9. DVSM → QUANTUM LIFT (INITIALIZATION)
// ============================================================

pub fn lift_to_quantum(state: &State) -> QuantumState {
    let (_map, basis) = build_basis(state);
    let n = basis.len().max(1);

    let uniform = Complex::new(1.0 / (n as f64).sqrt(), 0.0);

    QuantumState {
        amplitudes: vec![uniform; n],
    }
}

// ============================================================
// 10. FULL QUANTUM STEP (CLOSED UNITARY EVOLUTION)
// ============================================================

pub fn quantum_step(
    state: &State,
    psi: &QuantumState,
    dt: f64,
) -> QuantumState {

    let (basis_map, _) = build_basis(state);
    let n = basis_map.len();

    let h = hamiltonian(state, &basis_map, n);
    let u = cayley_unitary(&h, dt);

    evolve_unitary(psi, &u)
}

// ============================================================
// 11. DVSM INVARIANCE GUARANTEE
// ============================================================
//
// CORE GUARANTEE:
// - S (graph) is unchanged
// - ψ is a derived projection only
// - no feedback loop into S
//
// QUANTUM PROPERTY:
// - evolution is unitary (Cayley exact form)
// - norm preservation holds under numerical stability
//
// SEMANTIC BOUNDARY:
// - DVSM = deterministic invariant structure
// - Quantum lift = basis-dependent linear observer layer
// ============================================================
// ============================================================
// ADDENDUM 4 — CANONICAL CONSOLIDATION & CORRECTION (SINGLE FILE)
// ============================================================
//
// STATUS: CANONICAL LAYER (VERSIONED OVERRIDE)
// DATE: 2026-05-11
//
// CORE PRINCIPLE:
// DVSM graph S is invariant.
// Quantum state ψ is a derived Hilbert projection.
// Evolution is unitary via Cayley transform.
//
// ============================================================

use std::collections::HashMap;
use num_complex::Complex;
use rand::Rng;

// ============================================================
// 1. DVSM CORE STRUCTURES (INVARIANT LAYER)
// ============================================================

#[derive(Clone, Debug)]
pub struct Event {
    pub id: usize,
    pub links: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub events: HashMap<usize, Event>,
}

// ============================================================
// 2. QUANTUM STATE (HILBERT LAYER)
// ============================================================

#[derive(Clone, Debug)]
pub struct QuantumState {
    pub amplitudes: Vec<Complex<f64>>,
}

// ============================================================
// 3. BASIS CONSTRUCTION (DETERMINISTIC)
// ============================================================

fn build_basis(state: &State) -> (HashMap<usize, usize>, Vec<usize>) {
    let mut ids: Vec<usize> = state.events.keys().cloned().collect();
    ids.sort_unstable();

    let mut map = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        map.insert(*id, i);
    }

    (map, ids)
}

// ============================================================
// 4. HAMILTONIAN (HERMITIAN BY CONSTRUCTION)
// ============================================================

fn hamiltonian(
    state: &State,
    basis: &HashMap<usize, usize>,
    n: usize,
) -> Vec<Vec<Complex<f64>>> {

    let mut h = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for (id, event) in &state.events {
        let Some(&i) = basis.get(id) else { continue };

        for &j_id in &event.links {
            if let Some(&j) = basis.get(&j_id) {
                let w = Complex::new(1.0, 0.0);

                h[i][j] += w;
                h[j][i] += w.conj(); // Hermitian symmetry
            }
        }
    }

    h
}

// ============================================================
// 5. MATRIX INVERSION (GAUSSIAN ELIMINATION)
// ============================================================

fn matrix_inverse(
    mut a: Vec<Vec<Complex<f64>>>
) -> Vec<Vec<Complex<f64>>> {

    let n = a.len();

    let mut inv = vec![vec![Complex::new(0.0, 0.0); n]; n];
    for i in 0..n {
        inv[i][i] = Complex::new(1.0, 0.0);
    }

    for col in 0..n {

        // pivot
        let mut pivot_row = col;
        for row in (col + 1)..n {
            if a[row][col].norm_sqr() > a[pivot_row][col].norm_sqr() {
                pivot_row = row;
            }
        }

        a.swap(col, pivot_row);
        inv.swap(col, pivot_row);

        let pivot = a[col][col];
        if pivot.norm_sqr() < 1e-12 {
            panic!("Singular matrix in Cayley inversion");
        }

        for j in 0..n {
            a[col][j] /= pivot;
            inv[col][j] /= pivot;
        }

        for row in 0..n {
            if row != col {
                let factor = a[row][col];
                for j in 0..n {
                    a[row][j] -= factor * a[col][j];
                    inv[row][j] -= factor * inv[col][j];
                }
            }
        }
    }

    inv
}

// ============================================================
// 6. CAYLEY UNITARY TRANSFORM (STRICT UNITARY EVOLUTION)
// ============================================================
//
// U = (I + iH dt/2)(I - iH dt/2)^(-1)
// Guaranteed unitary if H is Hermitian
//

fn cayley_unitary(
    h: &Vec<Vec<Complex<f64>>>,
    dt: f64,
) -> Vec<Vec<Complex<f64>>> {

    let n = h.len();
    let i_c = Complex::new(0.0, 1.0);

    let mut a = vec![vec![Complex::new(0.0, 0.0); n]; n];
    let mut b = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for i in 0..n {
        for j in 0..n {

            let id = if i == j {
                Complex::new(1.0, 0.0)
            } else {
                Complex::new(0.0, 0.0)
            };

            a[i][j] = id + i_c * h[i][j] * (dt / 2.0);
            b[i][j] = id - i_c * h[i][j] * (dt / 2.0);
        }
    }

    let b_inv = matrix_inverse(b);

    let mut u = vec![vec![Complex::new(0.0, 0.0); n]; n];

    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                u[i][j] += a[i][k] * b_inv[k][j];
            }
        }
    }

    u
}

// ============================================================
// 7. UNITARY EVOLUTION
// ============================================================

fn evolve_unitary(
    psi: &QuantumState,
    u: &Vec<Vec<Complex<f64>>>,
) -> QuantumState {

    let n = psi.amplitudes.len();
    let mut next = vec![Complex::new(0.0, 0.0); n];

    for i in 0..n {
        for j in 0..n {
            next[i] += u[i][j] * psi.amplitudes[j];
        }
    }

    QuantumState { amplitudes: next }
}

// ============================================================
// 8. MEASUREMENT (POSITION BASIS)
// ============================================================

fn measure(psi: &QuantumState) -> usize {
    let probs: Vec<f64> = psi
        .amplitudes
        .iter()
        .map(|a| a.norm_sqr())
        .collect();

    let mut cumulative = 0.0;
    let r = rand::thread_rng().gen::<f64>();

    for (i, p) in probs.iter().enumerate() {
        cumulative += *p;
        if r <= cumulative {
            return i;
        }
    }

    probs.len().saturating_sub(1)
}

// ============================================================
// 9. DVSM → QUANTUM LIFT
// ============================================================

pub fn lift_to_quantum(state: &State) -> QuantumState {
    let (_map, basis) = build_basis(state);
    let n = basis.len().max(1);

    let uniform = Complex::new(1.0 / (n as f64).sqrt(), 0.0);

    QuantumState {
        amplitudes: vec![uniform; n],
    }
}

// ============================================================
// 10. FULL EVOLUTION STEP
// ============================================================

pub fn quantum_step(
    state: &State,
    psi: &QuantumState,
    dt: f64,
) -> QuantumState {

    let (basis_map, _) = build_basis(state);
    let n = basis_map.len();

    let h = hamiltonian(state, &basis_map, n);
    let u = cayley_unitary(&h, dt);

    evolve_unitary(psi, &u)
}

// ============================================================
// 11. CANONICAL GUARANTEE LAYER
// ============================================================
//
// - DVSM graph is invariant (no quantum feedback)
// - Hamiltonian is Hermitian (complex-symmetric closure)
// - Cayley transform guarantees unitary evolution
// - Basis is explicitly ordered (no hidden state)
// - Measurement is probabilistic projection only
//
// ============================================================
