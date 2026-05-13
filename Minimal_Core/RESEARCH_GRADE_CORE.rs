// ============================================================================
// DVSM-π+++ — MODE-COMPLETE STRATIFIED JET-MANIFOLD KERNEL
// ============================================================================
// Author: Daniel J. Dillberg
//
// DVSM-π+++ is a deterministic graph-coupled projected dynamical system
// defined on a stratified jet manifold.
//
// CORE EVOLUTION LAW
// ----------------------------------------------------------------------------
//
//     x̃_{t+1} = F(x_t, σ_t, G)
//
//     x_{t+1} = Π_𝓜^{mode}(x̃_{t+1})
//
// where:
//
//     x_t ∈ 𝓙³(ℝⁿ)
//         = (x, v, a, j)
//
//     F
//         : unconstrained graph-coupled evolution operator
//
//     Π_𝓜^{mode}
//         : mode-indexed projection operator onto feasible strata
//
//     G
//         : graph coupling / Laplacian interaction structure
//
//     σ_t
//         : external excitation field
//
// ----------------------------------------------------------------------------
// INTERPRETATION
// ----------------------------------------------------------------------------
//
// 1. F generates a candidate transition in ambient jet space.
//
// 2. Π_𝓜^{mode} enforces:
//
//      • feasibility
//      • bounded jet consistency
//      • stratified closure constraints
//
// 3. Only projected states belong to the admissible trajectory.
//
// Therefore:
//
//     Φ_mode := Π_𝓜^{mode} ∘ F
//
// is the actual system evolution operator.
//
// ----------------------------------------------------------------------------
// IMPORTANT CLARIFICATION
// ----------------------------------------------------------------------------
//
// DVSM-π+++ currently implements:
//
//     bounded jet-feasibility projection
//
// NOT a full nonlinear manifold constraint solve.
//
// Π_𝓜^{+++} presently performs:
//
//     • state feasibility projection
//     • bounded derivative preservation
//     • stratified admissibility enforcement
//
// Future extensions may replace this with:
//
//     • tangent-space constrained optimization
//     • variational manifold projection
//     • nonlinear jet consistency solves
//
// ============================================================================
//
// SYSTEM INTERPRETATION
// ----------------------------------------------------------------------------
//
// DVSM-π+++ is:
//
//   • a hybrid projected graph dynamical system
//   • on a stratified jet bundle
//   • with frozen-frame deterministic semantics
//   • and observational quotient layers
//
// NOT:
//
//   • a reward optimizer
//   • a probabilistic learner
//   • a loss-minimization framework
//
// ============================================================================
//
// PROJECTION HIERARCHY
// ----------------------------------------------------------------------------
//
// π+
//     positional feasibility projection
//
// π++
//     trajectory-consistent jet reconstruction
//
// π+++
//     bounded stratified jet-manifold closure
//
// These are NOT separate systems.
//
// They are different projection resolutions acting on the SAME underlying
// geometric evolution law.
//
// ============================================================================
//
// OBSERVATIONAL SEPARATION AXIOM
// ----------------------------------------------------------------------------
//
// Observers NEVER influence kernel causality.
//
//     kernel      = state evolution
//     projection  = admissibility closure
//     observers   = interpretation only
//
// Observational maps:
//
//     O_k : 𝓜_mode → ℝ^m
//
// are read-only functors over constrained trajectories.
//
// ============================================================================
//
// EXECUTION GUARANTEES
// ----------------------------------------------------------------------------
//
// • deterministic replay
// • frozen-frame updates
// • snapshot-isolated mutation
// • observer-side read-only execution
// • SIMD-friendly topology
// • GPU-mappable layout semantics
// • rollback-compatible trajectory history
//
// ============================================================================

use std::f64;

// ============================================================================
// MODES
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub enum Mode {
    // state-only projection
    PiPlus,

    // jet reconstruction projection
    PiPlusPlus,

    // bounded stratified jet projection
    PiPlusPlusPlus,
}

// ============================================================================
// BUNDLE STATE
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Bundle {
    pub x: f64,
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// GRAPH STRUCTURE
// ============================================================================

#[derive(Clone, Debug)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

// ============================================================================
// PARAMETERS
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Params {
    pub eta: f64,
    pub gamma: f64,
    pub coupling: f64,
}

// ============================================================================
// FEASIBILITY STRATUM
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Bounds {
    pub x_min: f64,
    pub x_max: f64,

    pub v_max: f64,
    pub a_max: f64,
    pub j_max: f64,
}

// ============================================================================
// FEASIBILITY CHECK
// ============================================================================

impl Bounds {
    #[inline(always)]
    pub fn contains(&self, b: &Bundle) -> bool {
        b.x >= self.x_min
            && b.x <= self.x_max
            && b.v.abs() <= self.v_max
            && b.a.abs() <= self.a_max
            && b.j.abs() <= self.j_max
    }
}

// ============================================================================
// KERNEL FLOW
// ============================================================================
//
// x + η(σ - x)
//
// ============================================================================

#[inline(always)]
fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// EXTERNAL EXCITATION
// ============================================================================

#[inline(always)]
fn excitation(sigma: f64, x: f64) -> f64 {
    sigma - x
}

// ============================================================================
// GRAPH LAPLACIAN
// ============================================================================

fn laplacian(graph: &Graph, x: &[f64], i: usize) -> f64 {
    let mut sum = 0.0;
    let mut deg = 0.0;

    for &(a, b) in &graph.edges {
        if a == i {
            sum += x[b] - x[a];
            deg += 1.0;
        }

        if b == i {
            sum += x[a] - x[b];
            deg += 1.0;
        }
    }

    if deg > 0.0 {
        sum / deg
    } else {
        0.0
    }
}

// ============================================================================
// UNCONSTRAINED EVOLUTION OPERATOR F
// ============================================================================
//
// x̃_{t+1} = F(x_t, σ_t, G)
//
// NOTE:
//   This stage is unconstrained ambient-space evolution.
//
// ============================================================================

fn evolve(
    current: Bundle,
    sigma: f64,
    lap: f64,
    p: &Params,
) -> Bundle {

    // ------------------------------------------------------------
    // unconstrained proposal
    // ------------------------------------------------------------

    let x_next =
        kernel(
            current.x,
            sigma + p.coupling * lap,
            p.eta,
        )
        + p.gamma * excitation(sigma, current.x);

    // ------------------------------------------------------------
    // jet reconstruction
    // ------------------------------------------------------------

    let v = x_next - current.x;

    let a = v - current.v;

    let j = a - current.a;

    Bundle {
        x: x_next,
        v,
        a,
        j,
    }
}

// ============================================================================
// PROJECTION OPERATOR Π_𝓜
// ============================================================================
//
// Current implementation:
//
//     bounded jet-feasibility projection
//
// NOT:
//
//     full nonlinear manifold optimization
//
// ============================================================================

fn project_bundle(
    b: Bundle,
    bounds: &Bounds,
) -> Bundle {

    Bundle {
        x: b.x.clamp(bounds.x_min, bounds.x_max),

        v: b.v.clamp(-bounds.v_max, bounds.v_max),

        a: b.a.clamp(-bounds.a_max, bounds.a_max),

        j: b.j.clamp(-bounds.j_max, bounds.j_max),
    }
}

// ============================================================================
// MODE-DEPENDENT PROJECTION Π_𝓜^{mode}
// ============================================================================

fn project_mode(
    b: Bundle,
    mode: Mode,
    bounds: &Bounds,
) -> Bundle {

    match mode {

        // --------------------------------------------------------
        // π+
        // positional feasibility only
        // --------------------------------------------------------

        Mode::PiPlus => {

            let x =
                b.x.clamp(bounds.x_min, bounds.x_max);

            Bundle {
                x,
                v: 0.0,
                a: 0.0,
                j: 0.0,
            }
        }

        // --------------------------------------------------------
        // π++
        // preserve reconstructed jets
        // --------------------------------------------------------

        Mode::PiPlusPlus => {

            let x =
                b.x.clamp(bounds.x_min, bounds.x_max);

            Bundle {
                x,
                v: b.v,
                a: b.a,
                j: b.j,
            }
        }

        // --------------------------------------------------------
        // π+++
        // bounded stratified jet projection
        // --------------------------------------------------------

        Mode::PiPlusPlusPlus => {
            project_bundle(b, bounds)
        }
    }
}

// ============================================================================
// ACTIVE SET SYMBOLS
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub enum ActiveSet {
    Interior,
    Upper,
    Lower,
}

// ============================================================================
// ACTIVE SET CLASSIFICATION
// ============================================================================

fn classify_active_set(
    x: f64,
    bounds: &Bounds,
) -> ActiveSet {

    if x >= bounds.x_max {
        ActiveSet::Upper
    } else if x <= bounds.x_min {
        ActiveSet::Lower
    } else {
        ActiveSet::Interior
    }
}

// ============================================================================
// CONTACT EVENT
// ============================================================================

#[derive(Clone, Debug)]
pub struct ContactEvent {
    pub step: usize,
    pub node: usize,

    pub x_raw: f64,
    pub x_projected: f64,

    pub clip_magnitude: f64,

    pub active: ActiveSet,
}

// ============================================================================
// DVSM SYSTEM
// ============================================================================

pub struct DVSM {
    pub nodes: Vec<Bundle>,

    pub history: Vec<Vec<Bundle>>,

    pub events: Vec<ContactEvent>,

    pub graph: Graph,

    pub params: Params,

    pub bounds: Bounds,

    pub mode: Mode,

    pub frame: usize,
}

// ============================================================================
// STEP FUNCTION
// ============================================================================
//
// Snapshot-synchronous deterministic update:
//
//     x̃_{t+1} = F(x_t)
//
//     x_{t+1} = Π_𝓜(x̃_{t+1})
//
// ============================================================================

impl DVSM {

    pub fn step(&mut self, sigma: f64) {

        // --------------------------------------------------------
        // frozen frame snapshot
        // --------------------------------------------------------

        let snapshot = self.nodes.clone();

        let x: Vec<f64> =
            snapshot.iter().map(|b| b.x).collect();

        let mut next = snapshot.clone();

        // --------------------------------------------------------
        // unconstrained evolution
        // --------------------------------------------------------

        for i in 0..snapshot.len() {

            let lap =
                laplacian(&self.graph, &x, i);

            let raw =
                evolve(
                    snapshot[i],
                    sigma,
                    lap,
                    &self.params,
                );

            // ----------------------------------------------------
            // projection closure
            // ----------------------------------------------------

            let projected =
                project_mode(
                    raw,
                    self.mode,
                    &self.bounds,
                );

            // ----------------------------------------------------
            // active-set diagnostics
            // ----------------------------------------------------

            let clip =
                (raw.x - projected.x).abs();

            if clip > 0.0 {

                self.events.push(ContactEvent {

                    step: self.frame,

                    node: i,

                    x_raw: raw.x,

                    x_projected: projected.x,

                    clip_magnitude: clip,

                    active: classify_active_set(
                        projected.x,
                        &self.bounds,
                    ),
                });
            }

            next[i] = projected;
        }

        // --------------------------------------------------------
        // atomic commit
        // --------------------------------------------------------

        self.history.push(snapshot);

        self.nodes = next;

        self.frame += 1;
    }
}

// ============================================================================
// EXAMPLE INITIALIZATION
// ============================================================================

fn build_system() -> DVSM {

    let graph = Graph {
        edges: vec![
            (0, 1),
            (1, 2),
            (2, 0),
        ],
    };

    let nodes = vec![
        Bundle::default(),
        Bundle::default(),
        Bundle::default(),
    ];

    DVSM {

        nodes,

        history: vec![],

        events: vec![],

        graph,

        params: Params {
            eta: 0.15,
            gamma: 0.65,
            coupling: 0.2,
        },

        bounds: Bounds {

            x_min: -1.0,
            x_max:  1.0,

            v_max: 2.0,
            a_max: 4.0,
            j_max: 8.0,
        },

        mode: Mode::PiPlusPlusPlus,

        frame: 0,
    }
}

// ============================================================================
// MAIN
// ============================================================================

fn main() {

    let mut system = build_system();

    for t in 0..100 {

        let sigma =
            (t as f64 * 0.05).sin() * 1.2;

        system.step(sigma);
    }

    println!("frames: {}", system.frame);

    println!("events: {}", system.events.len());

    for e in system.events.iter().take(10) {

        println!(
            "step={} node={} raw={:.3} proj={:.3} clip={:.3}",
            e.step,
            e.node,
            e.x_raw,
            e.x_projected,
            e.clip_magnitude,
        );
    }
}

// ============================================================================
// FINAL INTERPRETATION
// ============================================================================
//
// DVSM-π+++ defines:
//
//     Φ_mode := Π_𝓜^{mode} ∘ F
//
// acting on:
//
//     𝓙³(ℝⁿ)
//
// with:
//
// • graph-coupled evolution
// • projection-defined admissibility
// • hybrid active-set geometry
// • deterministic frozen-frame execution
// • observational separation
//
// ============================================================================
//
// KEY STRUCTURAL RESULT
// ----------------------------------------------------------------------------
//
//     π+, π++, π+++ are NOT separate systems.
//
// They are:
//
//     projection resolutions over the SAME underlying
//     stratified jet-bundle dynamics.
//
// ============================================================================
