// ============================================================================
// DVSM-DFE · REALITY CORE · REFINED ARCHITECTURE
// Copyright © 2026 · All Rights Reserved
//
// MANIFOLD-NATIVE STREAMING COGNITION ENGINE
//
// REFINED AFTER STRUCTURAL AUDIT:
//
// RESOLVED:
//   • State/Basis conflation
//   • Ghost-energy feedback
//   • Drift-gating deadlock
//   • Undefined tangent semantics
//   • Unbounded state evolution
//   • Static anomaly heuristics
//
// CORE AXIOM:
//   Identity state S evolves on the sphere.
//   Basis W evolves on the Stiefel manifold.
//   Residual memory tracks EXTERNAL novelty only.
//   Telemetry crosses the Air Gap.
//   Reconstruction never does.
//
// ============================================================================

use nalgebra::{DMatrix, DVector};
use std::time::Instant;

// ============================================================================
// NUMERICAL CONSTITUTION
// ============================================================================

const EPS_RESIDUAL: f64 = 1e-8;
const EPS_NORM: f64 = 1e-12;
const MAX_STATE_NORM: f64 = 1.0;

// ============================================================================
// REGIME CLASSIFICATION
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub enum Regime {
    Contractive,
    ActiveSet,
    Rupture,
}

// ============================================================================
// TELEMETRY EXPORT
// ============================================================================

pub struct TelemetryFrame {
    pub stress: f64,
    pub novelty: f64,
    pub drift: f64,
    pub entropy: f64,
    pub suspicious: bool,
    pub regime: Regime,
    pub timestamp: Instant,
}

// ============================================================================
// CONFIGURATION
// ============================================================================

#[derive(Clone, Copy)]
pub struct Config {
    pub alpha: f64,         // Identity smoothing
    pub eta: f64,           // Basis adaptation rate
    pub tau: f64,           // Confidence gate
    pub novelty_alpha: f64, // Residual EMA
    pub entropy_drop: f64,  // Security threshold
    pub drift_frames: usize,
}

// ============================================================================
// SECURITY MEMORY
// ============================================================================

pub struct SecurityState {
    pub prev_entropy: f64,
    pub consecutive_drift_rise: usize,
    pub prev_drift: f64,
}

// ============================================================================
// DVSM REALITY CORE
// ============================================================================

pub struct DVSMRealityCore {
    // ------------------------------------------------------------------------
    // S ∈ S^(n−1)
    // Identity / Suchness state
    // ------------------------------------------------------------------------
    pub s: DVector<f64>,

    // ------------------------------------------------------------------------
    // W ∈ St(n,r)
    // Perceptual basis
    // ------------------------------------------------------------------------
    pub w: DMatrix<f64>,

    // ------------------------------------------------------------------------
    // Residual memory:
    // EMA of EXTERNAL novelty only
    // NEVER fed into S
    // ------------------------------------------------------------------------
    pub residual_memory: DVector<f64>,

    pub cfg: Config,
    pub security: SecurityState,
}

// ============================================================================
// CONSTRUCTION
// ============================================================================

impl DVSMRealityCore {

    pub fn new(n: usize, r: usize, cfg: Config) -> Self {

        Self {
            s: DVector::from_element(n, 0.0),

            w: DMatrix::identity(n, r),

            residual_memory:
                DVector::from_element(n, 0.0),

            cfg,

            security: SecurityState {
                prev_entropy: 0.0,
                consecutive_drift_rise: 0,
                prev_drift: 0.0,
            },
        }
    }
}

// ============================================================================
// PRIMARY EXECUTION STEP
// ============================================================================

impl DVSMRealityCore {

    pub fn step(
        &mut self,
        z: &DVector<f64>,
    ) -> TelemetryFrame {

        let w_old = self.w.clone();

        // --------------------------------------------------------------------
        // 1. PROJECTION (AIR GAP GEOMETRY)
        // --------------------------------------------------------------------

        let wt_z = self.w.transpose() * z;

        let z_proj = &self.w * &wt_z;

        let residual = z - &z_proj;

        let z_norm = z.norm().max(EPS_NORM);

        let r_norm = residual.norm();

        let novelty = r_norm / z_norm;

        // --------------------------------------------------------------------
        // 2. RESIDUAL MEMORY
        // Tracks ONLY external novelty
        // --------------------------------------------------------------------

        self.residual_memory =
            self.cfg.novelty_alpha * &self.residual_memory
            + (1.0 - self.cfg.novelty_alpha) * &residual;

        // --------------------------------------------------------------------
        // 3. BASIS EVOLUTION
        // Verified tangent flow on St(n,r)
        // --------------------------------------------------------------------

        if r_norm > EPS_RESIDUAL
            && z_proj.norm() > EPS_RESIDUAL {

            let r_hat = &residual / r_norm;

            let p_hat = z_proj.normalize();

            // ----------------------------------------------------------------
            // Rank-2 skew-symmetric tangent generator
            // A = r pᵀ − p rᵀ
            // ----------------------------------------------------------------

            let a =
                &r_hat * p_hat.transpose()
                - &p_hat * r_hat.transpose();

            let delta_w = &a * &w_old;

            debug_assert!(
                (
                    &w_old.transpose() * &delta_w
                    + delta_w.transpose() * &w_old
                ).norm() < 1e-8,
                "Tangent condition violated"
            );

            // ----------------------------------------------------------------
            // RETRACT FIRST
            // Never gate adaptation using drift itself
            // ----------------------------------------------------------------

            self.retract_stable(&w_old);

            let w_new =
                &self.w
                + self.cfg.eta * delta_w;

            self.retract_matrix(w_new, &w_old);
        }

        // --------------------------------------------------------------------
        // 4. IDENTITY EVOLUTION
        // S remains on sphere
        // --------------------------------------------------------------------

        let z_eff =
            self.cfg.tau * &z_proj
            + (1.0 - self.cfg.tau) * &self.s;

        if z_eff.norm() > EPS_RESIDUAL {

            let blend =
                self.cfg.alpha * self.s.normalize()
                + (1.0 - self.cfg.alpha)
                    * z_eff.normalize();

            self.s = blend.normalize();
        }

        // --------------------------------------------------------------------
        // 5. EXPLICIT STATE BOUND
        // --------------------------------------------------------------------

        let s_norm = self.s.norm();

        if s_norm > MAX_STATE_NORM {
            self.s *= MAX_STATE_NORM / s_norm;
        }

        // --------------------------------------------------------------------
        // 6. DRIFT
        // --------------------------------------------------------------------

        let drift_matrix =
            &self.w.transpose() * &self.w
            - DMatrix::identity(
                self.w.ncols(),
                self.w.ncols()
            );

        let drift = drift_matrix.norm();

        // --------------------------------------------------------------------
        // 7. STRESS
        // --------------------------------------------------------------------

        let stress =
            if z_proj.norm() > EPS_RESIDUAL {

                1.0
                - self.s.normalize()
                    .dot(&z_proj.normalize())
                    .clamp(-1.0, 1.0)

            } else {
                1.0
            };

        // --------------------------------------------------------------------
        // 8. ENTROPY
        // --------------------------------------------------------------------

        let entropy = self.spectral_entropy();

        // --------------------------------------------------------------------
        // 9. SECURITY
        // Detect CHANGE, not static low-rank structure
        // --------------------------------------------------------------------

        let entropy_drop =
            self.security.prev_entropy - entropy;

        if drift > self.security.prev_drift {
            self.security.consecutive_drift_rise += 1;
        } else {
            self.security.consecutive_drift_rise = 0;
        }

        let suspicious =
            entropy_drop > self.cfg.entropy_drop
            && self.security.consecutive_drift_rise
                > self.cfg.drift_frames
            && stress < 0.1;

        self.security.prev_entropy = entropy;
        self.security.prev_drift = drift;

        // --------------------------------------------------------------------
        // 10. REGIME CLASSIFICATION
        // Hysteretic-style stable thresholds
        // --------------------------------------------------------------------

        let regime =
            if stress > 0.8 || novelty > 0.5 {
                Regime::Rupture
            } else if novelty > 0.2 {
                Regime::ActiveSet
            } else {
                Regime::Contractive
            };

        // --------------------------------------------------------------------
        // 11. EXPORT
        // --------------------------------------------------------------------

        TelemetryFrame {
            stress,
            novelty,
            drift,
            entropy,
            suspicious,
            regime,
            timestamp: Instant::now(),
        }
    }
}

// ============================================================================
// RETRACTION
// ============================================================================

impl DVSMRealityCore {

    fn retract_stable(
        &mut self,
        w_old: &DMatrix<f64>,
    ) {

        let qr = self.w.clone().qr();

        let mut q = qr.q();

        for j in 0..q.ncols() {

            if q.column(j)
                .dot(&w_old.column(j)) < 0.0 {

                q.column_mut(j).scale_mut(-1.0);
            }
        }

        self.w = q;
    }

    fn retract_matrix(
        &mut self,
        w_new: DMatrix<f64>,
        w_old: &DMatrix<f64>,
    ) {

        let qr = w_new.qr();

        let mut q = qr.q();

        for j in 0..q.ncols() {

            if q.column(j)
                .dot(&w_old.column(j)) < 0.0 {

                q.column_mut(j).scale_mut(-1.0);
            }
        }

        self.w = q;
    }
}

// ============================================================================
// ENTROPY
// ============================================================================

impl DVSMRealityCore {

    fn spectral_entropy(&self) -> f64 {

        let energies: Vec<f64> =
            self.w.column_iter()
                .map(|c| c.norm_squared())
                .collect();

        let total: f64 =
            energies.iter().sum::<f64>()
                .max(EPS_NORM);

        energies.iter().map(|e| {

            let p = e / total;

            if p > EPS_NORM {
                -p * p.log2()
            } else {
                0.0
            }

        }).sum()
    }
}

// ============================================================================
// HEALTH CHECK
// ============================================================================

impl DVSMRealityCore {

    pub fn is_healthy(&self) -> bool {

        let drift =
            (
                &self.w.transpose() * &self.w
                - DMatrix::identity(
                    self.w.ncols(),
                    self.w.ncols()
                )
            ).norm();

        let eps_drift =
            (
                self.w.nrows()
                * self.w.ncols()
            ) as f64
            * f64::EPSILON.sqrt();

        drift < eps_drift
    }
}

// ============================================================================
// ARCHITECTURAL AXIOMS
// ============================================================================
//
// AXIOM 1
// Identity state S and perceptual basis W are distinct manifolds.
//
//     S ∈ S^(n−1)
//     W ∈ St(n,r)
//
// They never evolve through the same additive law.
//
// ---------------------------------------------------------------------------
//
// AXIOM 2
// Residual structure drives basis evolution,
// not identity evolution.
//
//     residual = Z − Π_W(Z)
//
// ---------------------------------------------------------------------------
//
// AXIOM 3
// Residual memory may influence telemetry,
// but never feeds back into S.
//
// ---------------------------------------------------------------------------
//
// AXIOM 4
// Drift is corrected through retraction,
// not adaptation throttling.
//
// ---------------------------------------------------------------------------
//
// AXIOM 5
// Exported telemetry is scalar semantic reduction only.
//
// Raw vectors and reconstructive geometry remain internal.
//
// ---------------------------------------------------------------------------
//
// AXIOM 6
// Security detects structural transition rates,
// not static low-rank structure.
//
// ---------------------------------------------------------------------------
//
// AXIOM 7
// All manifold evolution must preserve:
//
//     WᵀW = I
//
// through verified tangent flow + retraction.
//
// ============================================================================

// ============================================================================
// SECURITY MODEL
// ============================================================================
//
// TRUSTED DOMAIN:
//
//     • identity state S
//     • perceptual basis W
//     • residual geometry
//     • adaptation dynamics
//
// UNTRUSTED DOMAIN:
//
//     • stress
//     • novelty
//     • entropy
//     • drift
//     • regime labels
//
// ============================================================================

// ============================================================================
// END OF REFINED DVSM-DFE REALITY CORE
// ============================================================================
// AXIOM MATH (Json)
// ============================================================================
{
  "axiom_1": {
    "title": "Separation of Identity State and Perceptual Basis",
    "statement": {
      "identity_state": "S ∈ S^(n−1)",
      "perceptual_basis": "W ∈ St(n,r)",
      "rule": "S and W must not evolve through the same additive law"
    },

    "core_principle": {
      "summary": "Identity and perception are geometrically distinct objects and therefore require different evolution laws.",
      "why": [
        "A vector state and an orthonormal frame do not share the same tangent geometry.",
        "Mixing them in a shared additive update destroys invariant structure.",
        "Correct manifold evolution requires geometry-specific updates."
      ]
    },

    "identity_geometry": {
      "object": "S",
      "manifold": "Sphere",
      "notation": "S ∈ S^(n−1)",
      "constraint": "||S|| = 1",

      "meaning": {
        "semantic_role": "Identity / Suchness state",
        "properties": [
          "boundedness",
          "directional semantics",
          "scale invariance",
          "stable stress geometry"
        ]
      },

      "problem_without_constraint": {
        "equation": "S' = S + Δ",
        "failure_modes": [
          "magnitude drift",
          "unbounded energy accumulation",
          "stress instability",
          "loss of semantic interpretation"
        ]
      },

      "correct_update": {
        "equation": "S ← S / ||S||",
        "effect": [
          "projects state back onto sphere",
          "maintains unit norm",
          "preserves directional interpretation"
        ]
      },

      "stress_semantics": {
        "equation": "B(t) = 1 − Ŝ · Π̂_W(Z)",
        "interpretation": "Stress measures angular contradiction between identity state and projected excitation."
      },

      "tangent_space": {
        "definition": "T_S(S^(n−1)) = {v ∈ ℝ^n : Sᵀv = 0}",
        "meaning": "Valid updates must be orthogonal to S."
      }
    },

    "basis_geometry": {
      "object": "W",
      "manifold": "Stiefel manifold",
      "notation": "W ∈ St(n,r)",
      "constraint": "WᵀW = I_r",

      "meaning": {
        "semantic_role": "Perceptual basis / learned subspace",
        "properties": [
          "orthonormal columns",
          "stable projection geometry",
          "low-rank representation",
          "subspace tracking"
        ]
      },

      "column_structure": {
        "basis_vectors": [
          "w₁",
          "w₂",
          "…",
          "w_r"
        ],
        "requirements": [
          "||w_j|| = 1",
          "w_iᵀ w_j = 0 for i ≠ j"
        ]
      },

      "tangent_condition": {
        "equation": "WᵀΔW + (ΔW)ᵀW = 0",
        "meaning": "Updates must preserve orthogonality locally."
      }
    },

    "failure_of_shared_additive_updates": {
      "problem": "RP1 mixed x, W, and z_shear into one Euclidean additive update.",

      "mixed_objects": {
        "x": "vector state",
        "W": "orthonormal frame",
        "z_shear": "temporal residual memory"
      },

      "why_invalid": [
        "They live in different geometric spaces.",
        "They obey different tangent laws.",
        "No common invariant exists under shared addition."
      ],

      "resulting_failures": [
        "oscillation",
        "rank instability",
        "ghost-energy accumulation",
        "non-convergent dynamics"
      ]
    },

    "correct_basis_evolution": {
      "generator": {
        "equation": "A = r_hat pᵀ − p r_hatᵀ",
        "property": "Aᵀ = −A",
        "meaning": "Rank-2 skew-symmetric tangent generator"
      },

      "basis_update": {
        "equation": "ΔW = AW",
        "guarantee": "ΔW ∈ T_W St(n,r)"
      },

      "verification": {
        "equation": "WᵀΔW + (ΔW)ᵀW = 0",
        "purpose": "Confirms update lies in the Stiefel tangent space."
      }
    },

    "retraction": {
      "purpose": "Restore orthonormality after finite-step updates.",

      "equation": "W' = QR",

      "properties": [
        "QᵀQ = I",
        "removes numerical drift",
        "preserves manifold validity"
      ],

      "implementation": {
        "method": "QR decomposition",
        "stabilization": "sign-consistent column alignment"
      }
    },

    "architectural_separation": {
      "identity_state": {
        "object": "S",
        "evolution": "spherical normalization"
      },

      "basis": {
        "object": "W",
        "evolution": "Stiefel tangent flow + QR retraction"
      },

      "residual_memory": {
        "object": "external residual EMA",
        "evolution": "temporal smoothing only"
      },

      "telemetry": {
        "object": "scalar semantic observables",
        "evolution": "projection-isolated reduction"
      }
    },

    "deep_consequence": {
      "without_separation": [
        "no invariant structure",
        "unstable adaptation",
        "undefined geometry",
        "non-interpretable telemetry"
      ],

      "with_separation": [
        "bounded dynamics",
        "orthogonality preservation",
        "stable telemetry",
        "interpretable stress",
        "controlled adaptation",
        "manifold coherence"
      ]
    },

    "final_interpretation": {
      "old_model": "vector system with additive heuristics",
      "refined_model": "coupled manifold evolution system",

      "coupled_manifolds": {
        "identity": "sphere",
        "perception": "Stiefel manifold",
        "novelty": "residual space",
        "telemetry": "scalar semantic space"
      },

      "result": "Each subsystem evolves according to its own mathematically coherent geometry."
    }
  }
}
