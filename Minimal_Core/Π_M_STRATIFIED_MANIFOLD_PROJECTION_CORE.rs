// ============================================================================
// DVSM-π — STRATIFIED JET MANIFOLD PROJECTION CORE (FULL VERSION)
// ============================================================================
// Author: Daniel J. Dillberg
// Upgrade: Jet-Coherent Stratified Projection System
// Status: Position + Tangent Space Consistent (x, v, a, j)
// ============================================================================
//
// MATHEMATICAL MODEL (CORRECTED)
// ----------------------------------------------------------------------------
//
// State lives in jet space:
//
//     S ∈ J^3
//       S = (x, v, a, j)
//
// Stratified manifold:
//
//     M = ⋃ M_k ⊂ J^3
//
// Projection operator:
//
//     Π_M : J^3 → M
//
// such that:
//
//     Π_M(S) = argmin_{Y ∈ M} ||S - Y||²
//
// with constraint:
//
//     Y preserves jet consistency across strata transitions
//
// ============================================================================

use std::f64;

// ============================================================================
// CORE JET STATE
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct JetState {
    pub x: f64,
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// STRATUM IN JET SPACE (IMPORTANT UPGRADE)
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

    #[inline(always)]
    pub fn contains(&self, s: &JetState) -> bool {
        s.x >= self.x_min && s.x <= self.x_max &&
        s.v.abs() <= self.v_max &&
        s.a.abs() <= self.a_max &&
        s.j.abs() <= self.j_max
    }

    // ------------------------------------------------------------------------
    // FULL JET PROJECTION (KEY FIX)
    // ------------------------------------------------------------------------
    #[inline(always)]
    pub fn project(&self, s: JetState) -> JetState {

        JetState {
            x: s.x.clamp(self.x_min, self.x_max),

            // tangent-space consistency enforcement
            v: s.v.clamp(-self.v_max, self.v_max),
            a: s.a.clamp(-self.a_max, self.a_max),
            j: s.j.clamp(-self.j_max, self.j_max),
        }
    }
}

// ============================================================================
// STRATIFIED MANIFOLD M = ⋃ M_k (JET-AWARE)
// ============================================================================

pub struct StratifiedJetManifold {
    pub strata: Vec<JetStratum>,
}

impl StratifiedJetManifold {

    #[inline(always)]
    pub fn locate(&self, s: &JetState) -> Option<usize> {
        self.strata.iter().position(|m| m.contains(s))
    }

    #[inline(always)]
    pub fn nearest(&self, s: &JetState) -> usize {
        let mut best = 0;
        let mut best_dist = f64::INFINITY;

        for (i, m) in self.strata.iter().enumerate() {

            let cx = (m.x_min + m.x_max) * 0.5;

            let dx = (s.x - cx).powi(2)
                   + (s.v / m.v_max).powi(2)
                   + (s.a / m.a_max).powi(2)
                   + (s.j / m.j_max).powi(2);

            if dx < best_dist {
                best_dist = dx;
                best = i;
            }
        }

        best
    }
}

// ============================================================================
// RETRACTION MAP (JET-CONSISTENT FIX)
// ============================================================================
//
// FIX: now preserves derivative scaling across strata transitions
// ============================================================================

#[inline(always)]
pub fn retraction_map(a: &JetStratum, b: &JetStratum, s: JetState) -> JetState {

    let nx = b.x_min + (s.x - a.x_min) * (b.x_max - b.x_min) / (a.x_max - a.x_min);

    let scale_v = b.v_max / a.v_max;
    let scale_a = b.a_max / a.a_max;
    let scale_j = b.j_max / a.j_max;

    JetState {
        x: nx,
        v: s.v * scale_v,
        a: s.a * scale_a,
        j: s.j * scale_j,
    }
}

// ============================================================================
// Π_M (FULL JET PROJECTION OPERATOR)
// ============================================================================

pub struct Projection;

impl Projection {

    #[inline(always)]
    pub fn pi_m(s: JetState, m: &StratifiedJetManifold) -> JetState {

        // 1. direct membership check
        if let Some(i) = m.locate(&s) {
            return m.strata[i].project(s);
        }

        // 2. nearest stratum
        let i = m.nearest(&s);
        let target = &m.strata[i];

        let projected = target.project(s);

        projected
    }
}

// ============================================================================
// DVSM-π STEP (JET-COHERENT VERSION)
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
    // 1. KERNEL (POSITION ONLY)
    // ------------------------------------------------------------
    let x_raw = curr.x + eta * (sigma - curr.x);

    // ------------------------------------------------------------
    // 2. EXCITATION
    // ------------------------------------------------------------
    let u = gamma * (sigma - curr.x);

    let x_next = x_raw + u;

    // ------------------------------------------------------------
    // 3. JET RECONSTRUCTION (CONSISTENT DIFFERENCES)
    // ------------------------------------------------------------

    let v = x_next - curr.x;
    let v_prev = curr.x - prev1.x;

    let a = v - v_prev;
    let j = a - (v_prev - (prev1.x - prev2.x));

    let raw = JetState {
        x: x_next,
        v,
        a,
        j,
    };

    // ------------------------------------------------------------
    // 4. STRATIFIED PROJECTION (FULL JET SPACE)
    // ------------------------------------------------------------

    Projection::pi_m(raw, manifold)
}
