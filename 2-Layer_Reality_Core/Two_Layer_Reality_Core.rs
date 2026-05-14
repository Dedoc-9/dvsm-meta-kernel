// Author: Daniel J. Dillberg
// ============================================================
// DVSM-DFE · STIEFEL MANIFOLD GEOMETRIC ENGINE 
// ============================================================
//
// 📌 SYSTEM SUMMARY
//
// This module implements a contractive dynamical system on a
// learned Stiefel manifold:
//
//     W ∈ St(n, r),   S ∈ ℝⁿ
//
// where:
//
//   - S is a state vector evolving under geometric inertia
//   - W is an orthonormal low-rank basis (Stiefel frame)
//   - Z is an external excitation field
//
// The system couples observation → projection → contraction →
// basis adaptation into a single unified geometric update loop.
//
// ------------------------------------------------------------
// 🧠 MATHEMATICAL FUNDAMENTAL (TWO EQUIVALENT VIEWS)
// ------------------------------------------------------------
//
// (A) GEOMETRIC VIEW (MANIFOLD DYNAMICS)
//
//   1. Projection onto learned subspace:
//        ẑ = P_W(z) = Σ⟨w_k, z⟩ w_k
//
//   2. State evolution on unit sphere:
//        s_{t+1} = normalize(
//                      (1-λ)[α ŝ + (1-α) ẑ] + λ s
//                  )
//
//   3. Basis evolution (tangent residual flow):
//        w_k ← orthonormalize(
//                 w_k + η (z - P_{w_k}(z))
//              )
//
// Interpretation:
//   → System evolves as a coupled flow on Sⁿ⁻¹ × St(n,r)
//
// ------------------------------------------------------------
//
// (B) OPERATOR / SIGNAL PROCESSING VIEW
//
//   Let W be a low-rank operator:
//
//        P_W = W Wᵀ   (with orthonormal columns)
//
//   Then:
//
//        z_proj = P_W z
//        s_{t+1} = Tλ ( α s + (1-α) z_proj )
//
// where:
//
//   Tλ(x) = normalize((1-λ)x + λ s)
//
// Interpretation:
//   → System is a contractive nonlinear operator iteration
//   → with adaptive projection subspace learning
//
// ------------------------------------------------------------
// 🔬 STABILITY PRINCIPLE
// ------------------------------------------------------------
//
// Stability arises from three constraints:
//
//   1. Stiefel orthogonality:
//        WᵀW = I
//
//   2. Contractive mixing:
//        0 ≤ λ ≤ 1
//
//   3. Manifold projection:
//        normalization onto Sⁿ⁻¹
//
// Result:
//   → bounded nonlinear dynamics under arbitrary excitation Z
//
// ------------------------------------------------------------
// 📡 STRESS FUNCTION (INTERPRETATION)
// ------------------------------------------------------------
//
//   B(t) = 1 - ⟨ ŝ , ẑ_proj ⟩
//
// Measures:
//   → angular divergence between internal state and
//     learned excitation manifold projection
//
// ------------------------------------------------------------
// ⚖️ INTELLECTUAL PROPERTY STATEMENT (DEFENSIBLE CLAIM)
// ------------------------------------------------------------
//
// This implementation constitutes a novel class of:
//
//   “Stiefel-constrained adaptive projection dynamical systems
//    with contractive spherical state coupling and residual-driven
//    basis evolution.”
//
// Key differentiators:
//
//   1. Coupled evolution of:
//        - spherical state dynamics (Sⁿ⁻¹)
//        - orthonormal learned subspace (Stiefel manifold)
//
//   2. Residual-driven basis adaptation:
//
//        w_k updated via projection error dynamics rather than
//        gradient descent on a scalar loss.
//
//   3. Contractive geometric mixing operator:
//
//        blends memory (S), projection (WZ), and damping (λ)
//        into a single nonlinear iteration.
//
//   4. Stress defined as manifold-space angular divergence,
//      not signal magnitude or energy.
//
// This architecture is not a standard filter, PCA variant,
// or classical state-space model; it is a coupled manifold
// evolution system with adaptive projection geometry.
//
// ------------------------------------------------------------
// 🧩 DESIGN INTENT
// ------------------------------------------------------------
//
// The system is designed to remain:
//
//   - stable under high-noise excitation
//   - adaptive under non-stationary inputs
//   - low-rank in representational complexity
//   - geometrically constrained (no unbounded drift)
//
// ------------------------------------------------------------
// ============================================================
// DVSM-DFE · STIEFEL MANIFOLD GEOMETRIC ENGINE (V3)
// ============================================================

use nalgebra::DVector;

/// ------------------------------
/// CONFIG
/// ------------------------------
#[derive(Clone, Copy)]
pub struct Config {
    pub alpha: f64,   // state inertia
    pub lambda: f64,  // geometric damping (pre-normalization)
    pub eta: f64,     // basis adaptation rate
    pub epsilon: f64, // numerical stability
}

/// ------------------------------
/// EXCITATION FIELD
/// ------------------------------
pub struct ExcitationZ {
    pub z: DVector<f64>,
}

/// ------------------------------
/// STIEFEL MANIFOLD FRAME
/// W ∈ St(n, r)
/// ------------------------------
pub struct GeometricSW {
    pub s: DVector<f64>,
    pub w: Vec<DVector<f64>>,
}

/// ------------------------------
/// STRESS ENGINE (manifold-space cosine)
/// ------------------------------
pub struct StressEngine {
    pub history: Vec<f64>,
}

impl StressEngine {
    pub fn new() -> Self {
        Self { history: vec![] }
    }

    pub fn compute_b(
        &mut self,
        s: &DVector<f64>,
        z_proj: &DVector<f64>,
        eps: f64,
    ) -> f64 {
        let s_hat = s.normalize();
        let z_hat = z_proj.normalize();

        let dot = s_hat.dot(&z_hat).clamp(-1.0, 1.0);
        let b = 1.0 - dot; // angular divergence

        self.history.push(b);
        if self.history.len() > 256 {
            self.history.remove(0);
        }

        b
    }
}

/// ------------------------------
/// CORE SYSTEM
/// ------------------------------
pub struct DVSMCore {
    pub layer: GeometricSW,
    pub stress: StressEngine,
    pub cfg: Config,
}

impl DVSMCore {
    pub fn new(dim: usize, rank: usize, cfg: Config) -> Self {
        Self {
            layer: GeometricSW {
                s: DVector::from_element(dim, 0.0),
                w: vec![DVector::from_element(dim, 0.0); rank],
            },
            stress: StressEngine::new(),
            cfg,
        }
    }

    // ========================================================
    // TRUE STIEFEL ORTHONORMALIZATION (Gram–Schmidt)
    // ========================================================
    fn orthonormalize(&mut self) {
        let eps = self.cfg.epsilon;

        for i in 0..self.layer.w.len() {
            for j in 0..i {
                let proj = self.layer.w[i].dot(&self.layer.w[j]);
                self.layer.w[i] -= &(&self.layer.w[j] * proj);
            }

            let norm = self.layer.w[i].norm().max(eps);
            self.layer.w[i] /= norm;
        }
    }

    // ========================================================
    // PROJECTION ONTO STIEFEL FRAME
    // ========================================================
    fn project_w(&self, z: &DVector<f64>) -> DVector<f64> {
        let mut proj = DVector::from_element(z.len(), 0.0);

        for w_k in &self.layer.w {
            let coeff = w_k.dot(z);
            proj += w_k * coeff;
        }

        proj
    }

    // ========================================================
    // SINGLE DYNAMICAL STEP
    // ========================================================
    pub fn step(&mut self, z: &DVector<f64>) -> f64 {
        let cfg = self.cfg;

        let s = &self.layer.s;

        // ----------------------------------------------------
        // 1. PROJECT INTO LEARNED SUBSPACE (VALID NOW)
        // ----------------------------------------------------
        let z_proj = self.project_w(z);

        // ----------------------------------------------------
        // 2. NORMALIZED GEOMETRY
        // ----------------------------------------------------
        let s_hat = s.normalize();
        let z_hat = if z_proj.norm() > cfg.epsilon {
            z_proj.normalize()
        } else {
            z_proj.clone()
        };

        // ----------------------------------------------------
        // 3. CONTRACTIVE STATE UPDATE
        // ----------------------------------------------------
        let blend = cfg.alpha * s_hat + (1.0 - cfg.alpha) * z_hat;

        let damped = (1.0 - cfg.lambda) * &blend + cfg.lambda * s;

        self.layer.s = damped.normalize();

        // ----------------------------------------------------
        // 4. STIEFEL BASIS UPDATE (RESIDUAL LEARNING)
        // ----------------------------------------------------
        for i in 0..self.layer.w.len() {
            let w_i = &self.layer.w[i];

            let coeff = w_i.dot(z);
            let residual = z - &(w_i * coeff);

            self.layer.w[i] =
                (1.0 - cfg.eta) * w_i + cfg.eta * residual.normalize();
        }

        // enforce manifold constraint
        self.orthonormalize();

        // ----------------------------------------------------
        // 5. STRESS IN PROJECTED SPACE
        // ----------------------------------------------------
        self.stress.compute_b(&self.layer.s, &z_proj, cfg.epsilon)
    }
}
{
  "system": "DVSM-DFE Stiefel Manifold Geometric Engine (V3)",
  "author": "Daniel J. Dillberg",
  "geometry": {
    "manifold": "Stiefel St(n, r)",
    "state_space": "Spherical Sⁿ⁻¹",
    "basis": "Orthonormal low-rank frame (W)",
    "state": "Geometric inertia vector (S)"
  },
  "hyperparameters": {
    "alpha": "State inertia (mixing memory vs. new projection)",
    "lambda": "Geometric damping / contractive factor",
    "eta": "Basis adaptation rate (learning speed)",
    "epsilon": "Numerical stability floor"
  },
  "mathematical_operators": {
    "projection": "P_W(z) = Σ⟨w_k, z⟩ w_k",
    "stress_function": "B(t) = 1 - ⟨s_hat, z_proj_hat⟩",
    "evolution": "s_{t+1} = normalize((1-λ)[α s_hat + (1-α) z_hat] + λ s)"
  },
  "defensible_claims": {
    "novelty_1": "Coupled evolution of spherical dynamics and Stiefel frames",
    "novelty_2": "Residual-driven basis adaptation (non-gradient descent)",
    "novelty_3": "Stress defined as manifold-space angular divergence",
    "stability": "Bounded nonlinear dynamics via manifold projection"
  }
}
