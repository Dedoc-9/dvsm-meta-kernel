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
