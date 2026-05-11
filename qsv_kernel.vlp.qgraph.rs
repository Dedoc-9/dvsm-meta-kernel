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
// QSV / DVSM FINAL ADDENDUM — QUANTUM-LIKE LIFT LAYER (CLOSED FORM)
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
