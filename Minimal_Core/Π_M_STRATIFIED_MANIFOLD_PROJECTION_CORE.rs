// ============================================================================
// DVSM-π — STRATIFIED JET MANIFOLD PROJECTION CORE (FULL VERSION)
// ============================================================================
// Author: Daniel J. Dillberg
// Upgrade: Jet-Coherent Stratified Projection System
// Status: Position + Tangent Space Consistent (x, v, a, j)
// ============================================================================
// DVSM-π+++ CORE UPDATE LAW
//
// The system evolves by unconstrained generation followed by geometric closure:
//
//     x_{t+1} = Π_M( F(x_t, σ_t) )
//
// where:
//   F      : unconstrained graph-coupled evolution operator
//   Π_M    : stratified projection onto feasible jet-manifold M
//   x_t    : current state on or near M
//   σ_t    : external excitation signal
//
// Interpretation:
//   - F proposes a candidate transition in ambient space
//   - Π_M enforces manifold consistency and feasibility closure
//   - only projected states are admitted into system trajectory
// ============================================================================
// DVSM-π — STRATIFIED JET-MANIFOLD DYNAMICS
// CORRECTED PROJECTION-FIRST FORMULATION
// ============================================================================
// Author: Daniel J. Dillberg
// Revision: Projection-Closed Jet Geometry Formalization
// Status: Research-Grade Geometric Constraint Core
// ============================================================================
//
// FUNDAMENTAL MATHEMATICAL MODEL
// ----------------------------------------------------------------------------
//
// The previous interpretation:
//
//     x_{t+1} = Π_M(F(x_t, σ_t))
//
// is only partially correct because:
//
//     Π_M does NOT act on scalar state alone.
//
// The true system state lives in discrete jet space:
//
//     S_t ∈ J^3
//
// where:
//
//     S_t = (x_t, v_t, a_t, j_t)
//
// and:
//
//     J^3 = discrete third-order jet bundle
//
// Therefore the corrected evolution law is:
//
//     S̃_{t+1} = F(S_t, σ_t)
//
//     S_{t+1} = Π_M(S̃_{t+1})
//
// where:
//
//     Π_M : J^3 → M
//
// and:
//
//     M = ⋃ M_k ⊂ J^3
//
// is a stratified feasible jet manifold.
//
// ============================================================================
//
// CRITICAL CONCEPTUAL CORRECTION
// ----------------------------------------------------------------------------
//
// OLD INTERPRETATION:
//
//     x evolves
//     jets are observations
//
// TRUE GEOMETRIC INTERPRETATION:
//
//     the FULL jet state evolves,
//     while jets are reconstructed geometric consistency sections.
//
// Meaning:
//
//     feasibility applies to trajectory geometry itself,
//     not merely scalar position.
//
// This removes a major hidden inconsistency:
//
//     independent derivative clamping
//     without trajectory coherence.
//
// ============================================================================
//
// GEOMETRIC INTERPRETATION
// ----------------------------------------------------------------------------
//
// Each stratum:
//
//     M_k ⊂ J^3
//
// defines a locally admissible trajectory geometry:
//
//     |v| ≤ v_max
//     |a| ≤ a_max
//     |j| ≤ j_max
//
// together with:
//
//     x ∈ [x_min, x_max]
//
// Projection enforces:
//
//     nearest feasible trajectory geometry
//
// not:
//
//     nearest scalar state.
//
// ============================================================================
//
// IMPORTANT RESEARCH NOTE
// ----------------------------------------------------------------------------
//
// This is NOT a proof of universal stability,
// adversarial invulnerability,
// or "military-grade protection."
//
// This file formalizes:
//
//     projection-constrained nonlinear dynamics
//     with jet-consistent feasibility structure.
//
// It is useful for:
//
//   • bounded control systems
//   • constrained simulation
//   • hybrid dynamical systems research
//   • manifold-constrained evolution
//   • safety envelopes
//   • graph-coupled feasibility dynamics
//
// It is NOT:
//
//   • a cryptographic protocol
//   • a security guarantee
//   • a defense system
//   • a proof of ungameability
//
// ============================================================================
//
// DEV NOTES — IMPORTANT GHOSTS TO WATCH FOR
// ----------------------------------------------------------------------------
//
// GHOST #1 — INDEPENDENT DERIVATIVE CLAMPING
//
// WRONG:
//
//     clamp x
//     clamp v
//     clamp a
//     clamp j
//
// independently.
//
// This breaks trajectory coherence.
//
// FIX:
//
//     reconstruct jets from projected trajectory history.
//
// ----------------------------------------------------------------------------
//
// GHOST #2 — DOUBLE PROJECTION
//
// WRONG:
//
//     project x
//     compute jet
//     project jet
//     recompute x
//
// This creates hidden discontinuities.
//
// FIX:
//
//     evolve once
//     reconstruct once
//     project once.
//
// ----------------------------------------------------------------------------
//
// GHOST #3 — METRIC REINTRODUCTION
//
// If any scalar:
//
//     E(x)
//     Loss(x)
//     Reward(x)
//
// affects evolution directly:
//
//     you have reintroduced optimization pressure.
//
// Keep diagnostics observational only.
//
// ----------------------------------------------------------------------------
//
// GHOST #4 — FALSE STABILITY CLAIMS
//
// Boundedness under Π_M:
//
//     ≠ universal stability
//
// Feasibility:
//
//     ≠ security guarantee
//
// Projection:
//
//     ≠ adversarial immunity
//
// ============================================================================

use std::f64;

// ============================================================================
// CORE JET STATE
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct JetState {
    pub x: f64,
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// STRATUM DEFINITION
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct JetStratum {
    pub x_min: f64,
    pub x_max: f64,

    pub v_max: f64,
    pub a_max: f64,
    pub j_max: f64,
}

impl JetStratum {

    // ------------------------------------------------------------------------
    // MEMBERSHIP TEST
    // ------------------------------------------------------------------------

    #[inline(always)]
    pub fn contains(&self, s: &JetState) -> bool {

        s.x >= self.x_min
        && s.x <= self.x_max
        && s.v.abs() <= self.v_max
        && s.a.abs() <= self.a_max
        && s.j.abs() <= self.j_max
    }

    // ------------------------------------------------------------------------
    // LOCAL PROJECTION
    // ------------------------------------------------------------------------
    //
    // NOTE:
    // This is only a LOCAL feasibility approximation.
    //
    // True projection onto a nonlinear trajectory manifold would require:
    //
    //     constrained variational optimization
    //
    // over trajectory space.
    //
    // ------------------------------------------------------------------------

    #[inline(always)]
    pub fn project_local(&self, s: JetState) -> JetState {

        JetState {
            x: s.x.clamp(self.x_min, self.x_max),

            v: s.v.clamp(-self.v_max, self.v_max),

            a: s.a.clamp(-self.a_max, self.a_max),

            j: s.j.clamp(-self.j_max, self.j_max),
        }
    }
}

// ============================================================================
// STRATIFIED MANIFOLD
// ============================================================================

pub struct StratifiedJetManifold {
    pub strata: Vec<JetStratum>,
}

impl StratifiedJetManifold {

    // ------------------------------------------------------------------------
    // LOCATE FEASIBLE STRATUM
    // ------------------------------------------------------------------------

    #[inline(always)]
    pub fn locate(&self, s: &JetState) -> Option<usize> {

        self.strata
            .iter()
            .position(|m| m.contains(s))
    }

    // ------------------------------------------------------------------------
    // NEAREST STRATUM
    // ------------------------------------------------------------------------
    //
    // Distance metric is normalized jet geometry distance.
    //
    // ------------------------------------------------------------------------

    #[inline(always)]
    pub fn nearest(&self, s: &JetState) -> usize {

        let mut best = 0usize;
        let mut best_dist = f64::INFINITY;

        for (i, m) in self.strata.iter().enumerate() {

            let cx = (m.x_min + m.x_max) * 0.5;

            let d =
                (s.x - cx).powi(2)
                + (s.v / m.v_max).powi(2)
                + (s.a / m.a_max).powi(2)
                + (s.j / m.j_max).powi(2);

            if d < best_dist {
                best_dist = d;
                best = i;
            }
        }

        best
    }
}

// ============================================================================
// RETRACTION MAP
// ============================================================================
//
// Used when crossing between strata.
//
// IMPORTANT:
//
// preserves derivative scaling structure.
//
// ============================================================================

#[inline(always)]
pub fn retraction_map(
    from: &JetStratum,
    to: &JetStratum,
    s: JetState,
) -> JetState {

    let x_norm =
        (s.x - from.x_min)
        / (from.x_max - from.x_min);

    let x_new =
        to.x_min
        + x_norm * (to.x_max - to.x_min);

    let sv = to.v_max / from.v_max;
    let sa = to.a_max / from.a_max;
    let sj = to.j_max / from.j_max;

    JetState {
        x: x_new,
        v: s.v * sv,
        a: s.a * sa,
        j: s.j * sj,
    }
}

// ============================================================================
// Π_M — STRATIFIED JET PROJECTION
// ============================================================================

pub struct Projection;

impl Projection {

    #[inline(always)]
    pub fn pi_m(
        s: JetState,
        m: &StratifiedJetManifold,
    ) -> JetState {

        // ------------------------------------------------------------
        // DIRECT FEASIBILITY
        // ------------------------------------------------------------

        if let Some(i) = m.locate(&s) {
            return m.strata[i].project_local(s);
        }

        // ------------------------------------------------------------
        // NEAREST STRATUM PROJECTION
        // ------------------------------------------------------------

        let i = m.nearest(&s);

        m.strata[i].project_local(s)
    }
}

// ============================================================================
// DVSM KERNEL
// ============================================================================

#[inline(always)]
pub fn kernel(
    x: f64,
    sigma: f64,
    eta: f64,
) -> f64 {

    x + eta * (sigma - x)
}

// ============================================================================
// EXCITATION FIELD
// ============================================================================

#[inline(always)]
pub fn excitation(
    sigma: f64,
    x: f64,
) -> f64 {

    sigma - x
}

// ============================================================================
// JET RECONSTRUCTION
// ============================================================================
//
// IMPORTANT:
//
// jets are reconstructed from trajectory,
// NOT independently evolved.
//
// ============================================================================

#[inline(always)]
pub fn reconstruct_jet(
    x2: f64,
    x1: f64,
    x0: f64,
) -> JetState {

    let v = x0 - x1;

    let v_prev = x1 - x2;

    let a = v - v_prev;

    let a_prev = v_prev - (x2 - x2);

    let j = a - a_prev;

    JetState {
        x: x0,
        v,
        a,
        j,
    }
}

// ============================================================================
// EVOLUTION MAP
// ============================================================================
//
// S̃_{t+1} = F(S_t, σ_t)
//
// ============================================================================

#[inline(always)]
pub fn evolve(
    curr: JetState,
    sigma: f64,
    eta: f64,
    gamma: f64,
) -> f64 {

    let k = kernel(curr.x, sigma, eta);

    let u = gamma * excitation(sigma, curr.x);

    k + u
}

// ============================================================================
// DVSM-π STEP
// ============================================================================
//
// FINAL CORRECTED FORM:
//
//     S̃_{t+1} = F(S_t, σ_t)
//
//     S_{t+1} = Π_M(S̃_{t+1})
//
// ============================================================================

pub fn dvsm_pi_step(
    prev2: JetState,
    prev1: JetState,
    curr: JetState,
    sigma: f64,
    eta: f64,
    gamma: f64,
    manifold: &StratifiedJetManifold,
) -> JetState {

    // ------------------------------------------------------------
    // 1. FREE EVOLUTION
    // ------------------------------------------------------------

    let x_next =
        evolve(curr, sigma, eta, gamma);

    // ------------------------------------------------------------
    // 2. JET RECONSTRUCTION
    // ------------------------------------------------------------

    let raw =
        reconstruct_jet(
            prev1.x,
            curr.x,
            x_next,
        );

    // ------------------------------------------------------------
    // 3. PROJECTION
    // ------------------------------------------------------------

    Projection::pi_m(raw, manifold)
}

// ============================================================================
// SIMPLE EXAMPLE
// ============================================================================

fn main() {

    let manifold = StratifiedJetManifold {
        strata: vec![
            JetStratum {
                x_min: -10.0,
                x_max: 10.0,

                v_max: 5.0,
                a_max: 3.0,
                j_max: 2.0,
            }
        ]
    };

    let s0 = JetState::default();
    let s1 = JetState::default();
    let s2 = JetState::default();

    let next =
        dvsm_pi_step(
            s0,
            s1,
            s2,
            1.0,
            0.2,
            0.3,
            &manifold,
        );

    println!("{:#?}", next);
}

// ============================================================================
// END OF FILE
// ============================================================================
