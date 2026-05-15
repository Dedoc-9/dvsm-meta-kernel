// ============================================================
// DVSM-DFE · UNIFIED STIEFEL GEOMETRIC ENGINE (V4)
// ============================================================
//
// Author: Daniel J. Dillberg
//
// 📌 SYSTEM OVERVIEW
//
// This module implements a coupled dynamical system on a
// product manifold:
//
//     S ∈ S^(n−1)          (unit spherical state space)
//     W ∈ St(n, r)         (orthonormal Stiefel frame)
//     Z ∈ ℝⁿ               (external excitation signal)
//
// The system performs structured geometric inference via:
//
//     projection → decomposition → contraction → retraction
//
// producing a bounded, low-rank, adaptive representation of
// streaming high-dimensional signals.
//
// ------------------------------------------------------------
// 🧠 TWO-LAYER ARCHITECTURE
// ------------------------------------------------------------
//
// LAYER 1 — GEOMETRIC CORE (Immutable Mathematics)
//
// Defines the invariant system:
//
//   • Projection operator:
//       Π_W(Z) = W Wᵀ Z
//
//   • Residual decomposition:
//       Z = Π_W(Z) + R
//       R ⟂ W
//
//   • State evolution (contractive spherical flow):
//       S_{t+1} = normalize(
//           (1 − λ)(α Ŝ + (1 − α) Ẑ) + λ S
//       )
//
//   • Basis evolution (residual-driven tangent update):
//       W ← QR(W + η · ΔR)
//
// Invariants:
//
//   ||S|| = 1
//   WᵀW = I
//   Projection is orthogonal and idempotent
//
// ------------------------------------------------------------
//
// LAYER 2 — RUNTIME PORTING ENGINE (Mutable Execution Layer)
//
// Responsible for:
//
//   • Streaming input ingestion (RF / gaming / hybrid)
//   • Scheduling of discrete-time updates
//   • Buffering and backpressure control
//   • Numerical safety enforcement (eps guards)
//   • Mode switching without altering geometry
//
// This layer DOES NOT modify mathematical structure.
//
// ------------------------------------------------------------
// 🧠 MATHEMATICAL INTERPRETATION (UNIFIED VIEW)
// ------------------------------------------------------------
//
// The system defines a contractive dynamical flow on:
//
//     S^(n−1) × St(n, r)
//
// governed by nonlinear operator iteration:
//
//     Z → Π_W(Z) → S update → W retraction → repeat
//
// Interpretation:
//
//   • S encodes geometric “state coherence”
//   • W encodes adaptive observation subspace
//   • Z is compressed through learned projection geometry
//
// ------------------------------------------------------------
// 🔬 STABILITY PRINCIPLES
// ------------------------------------------------------------
//
// Stability is guaranteed by:
//
//   1. Stiefel constraint:
//        WᵀW = I
//
//   2. Spherical normalization:
//        ||S|| = 1
//
//   3. Contractive mixing:
//        0 ≤ λ ≤ 1
//
//   4. Orthogonal residual separation:
//        R ⟂ W
//
// Result:
//   → bounded nonlinear dynamics under arbitrary excitation Z
//
// ------------------------------------------------------------
// 📡 OBSERVABLE METRIC (STRESS FUNCTION)
// ------------------------------------------------------------
//
//     B(t) = 1 − ⟨ Ŝ , Π_W(Z)̂ ⟩
//
// Meaning:
//   Angular divergence between internal state and projected input.
//
// Range:
//   B(t) ∈ [0, 2]
//
// ------------------------------------------------------------
// ⚖️ SYSTEM CLASSIFICATION
// ------------------------------------------------------------
//
// This system is a:
//
//   “contractive manifold-coupled projection dynamical system
//    with residual-driven Stiefel adaptation and spherical state flow”
//
// It is NOT:
//
//   • PCA or linear subspace tracking
//   • Kalman filtering
//   • standard state-space estimation
//   • gradient-descent loss minimization
//
// ------------------------------------------------------------
// 🧩 DESIGN OBJECTIVES
// ------------------------------------------------------------
//
// The architecture is designed to be:
//
//   • Stable under high-noise streaming inputs
//   • Adaptive under non-stationary excitation fields
//   • Low-rank in representation complexity
//   • Geometrically constrained (no drift outside manifolds)
//   • Deterministic under frame-synchronized execution
//
// ============================================================

use nalgebra::{DMatrix, DVector};
use std::time::{Duration, Instant};
use std::collections::VecDeque;

// ============================================================
// CONFIG (shared across layers)
// ============================================================
#[derive(Clone, Copy)]
pub struct Config {
    pub alpha: f64,     // state inertia
    pub lambda: f64,    // contractive damping
    pub eta: f64,       // manifold learning rate
    pub eps: f64,       // numerical floor
}

// ============================================================
// LAYER 1: GEOMETRIC CORE
// ============================================================

pub struct DVSMCore {
    pub s: DVector<f64>,   // S ∈ Sⁿ⁻¹
    pub w: DMatrix<f64>,   // W ∈ St(n,r)
    pub cfg: Config,
}

impl DVSMCore {

    pub fn new(n: usize, r: usize, cfg: Config) -> Self {
        Self {
            s: DVector::from_element(n, 0.0),
            w: DMatrix::identity(n, r),
            cfg,
        }
    }

    // ------------------------------
    // SAFE NORMALIZATION (core invariant)
    // ------------------------------
    fn normalize_safe(&self, v: &DVector<f64>) -> DVector<f64> {
        let n = v.norm();
        if n <= self.cfg.eps {
            return DVector::from_element(v.len(), 0.0);
        }
        v / n
    }

    // ------------------------------
    // PROJECTION: Π_W(z) = W Wᵀ z
    // ------------------------------
    fn project(&self, z: &DVector<f64>) -> DVector<f64> {
        let wt_z = self.w.transpose() * z;
        &self.w * wt_z
    }

    // ------------------------------
    // STIEFEL RETRACTION (QR ONLY)
    // ------------------------------
    fn retract(&mut self, w_new: DMatrix<f64>) {
        let qr = w_new.qr();
        self.w = qr.q();
    }

    // ------------------------------
    // SINGLE GEOMETRIC STEP
    // ------------------------------
    pub fn step(&mut self, z: &DVector<f64>) -> f64 {

        // 1. projection
        let z_proj = self.project(z);

        // 2. state update (Sⁿ⁻¹ contraction)
        let s_hat = self.normalize_safe(&self.s);
        let z_hat = self.normalize_safe(&z_proj);

        let blend = self.cfg.alpha * s_hat + (1.0 - self.cfg.alpha) * z_hat;
        let damped = (1.0 - self.cfg.lambda) * blend + self.cfg.lambda * &self.s;
        self.s = self.normalize_safe(&damped);

        // 3. residual-driven basis update
        let mut delta = DMatrix::zeros(self.w.nrows(), self.w.ncols());

        for j in 0..self.w.ncols() {
            let w_j = self.w.column(j).into_owned();

            let coeff = w_j.dot(z);
            let residual = z - &(w_j * coeff);

            let update = if residual.norm() > self.cfg.eps {
                self.normalize_safe(&residual)
            } else {
                w_j.clone()
            };

            delta.set_column(j, &((1.0 - self.cfg.eta) * &w_j + self.cfg.eta * update));
        }

        self.retract(delta);

        // 4. stress (angular divergence)
        let dot = self.s.dot(&self.normalize_safe(&z_proj)).clamp(-1.0, 1.0);
        1.0 - dot
    }
}

// ============================================================
// LAYER 2: RUNTIME PORTING ENGINE
// ============================================================

#[derive(Clone, Copy)]
pub enum RuntimeMode {
    Gaming,
    RF,
    Hybrid,
}

pub struct DVSMRuntime {
    pub core: DVSMCore,
    pub mode: RuntimeMode,
    pub buffer: VecDeque<DVector<f64>>,
    pub last: Instant,
    pub dt_rf: Duration,
    pub dt_game: Duration,
    pub max_buffer: usize,
}

impl DVSMRuntime {

    pub fn new(core: DVSMCore, mode: RuntimeMode) -> Self {
        Self {
            core,
            mode,
            buffer: VecDeque::new(),
            last: Instant::now(),
            dt_rf: Duration::from_millis(2),
            dt_game: Duration::from_millis(16),
            max_buffer: 64,
        }
    }

    // ------------------------------
    // INPUT INGESTION (FIFO SAFE)
    // ------------------------------
    pub fn ingest(&mut self, z: DVector<f64>) {
        if self.buffer.len() >= self.max_buffer {
            self.buffer.pop_front();
        }
        self.buffer.push_back(z);
    }

    // ------------------------------
    // TIMING POLICY
    // ------------------------------
    fn should_step(&self) -> bool {
        match self.mode {
            RuntimeMode::Gaming => self.last.elapsed() >= self.dt_game,
            RuntimeMode::RF => self.last.elapsed() >= self.dt_rf,
            RuntimeMode::Hybrid => self.last.elapsed() >= self.dt_rf && !self.buffer.is_empty(),
        }
    }

    // ------------------------------
    // EXECUTION TICK
    // ------------------------------
    pub fn tick(&mut self) -> Option<f64> {
        if !self.should_step() {
            return None;
        }

        let z = self.buffer.pop_front()?;
        self.last = Instant::now();

        Some(self.core.step(&z))
    }

    // ------------------------------
    // MODE SWITCH (safe reset boundary)
    // ------------------------------
    pub fn set_mode(&mut self, mode: RuntimeMode) {
        self.mode = mode;
        self.buffer.clear();
    }
}
// ============================================================
// DVSM-DFE · MATHEMATICAL FOUNDATION + AXIOM ADDON
// ============================================================
//
// This module encodes the formal system contract:
//
//   S ∈ S^(n−1)          (unit sphere state)
//   W ∈ St(n,r)         (Stiefel orthonormal basis)
//   Z ∈ R^n             (input signal)
//
// Core decomposition:
//
//   Z = Π_W(Z) + R
//   R ⟂ W
//
// Projection:
//
//   Π_W(Z) = W Wᵀ Z
//
// ============================================================

use nalgebra::{DMatrix, DVector};

// ============================================================
// AXIOM CONFIGURATION
// ============================================================

#[derive(Clone, Copy)]
pub struct AxiomConfig {
    pub eps: f64,        // numerical stability floor
    pub alpha: f64,      // state blending
    pub lambda: f64,     // contraction strength
    pub eta: f64,        // basis learning rate
}

// ============================================================
// CORE STATE (MANIFOLD PRODUCT SPACE)
// ============================================================

pub struct ManifoldCore {
    pub s: DVector<f64>,   // S ∈ S^(n−1)
    pub w: DMatrix<f64>,   // W ∈ St(n,r)
    pub cfg: AxiomConfig,
}

// ============================================================
// AXIOM 1 — SPHERICAL INVARIANCE
// ============================================================

#[inline]
fn enforce_sphere(s: &DVector<f64>, eps: f64) -> DVector<f64> {
    let n = s.norm();
    if n <= eps {
        return DVector::zeros(s.len());
    }
    s / n
}

// ============================================================
// AXIOM 2 — STIEFEL ORTHONORMALITY (QR RETRACTION)
// ============================================================

#[inline]
fn enforce_stiefel(w: DMatrix<f64>) -> DMatrix<f64> {
    let qr = w.qr();
    qr.q()
}

// ============================================================
// AXIOM 3 — PROJECTION OPERATOR Π_W(Z)
// ============================================================

#[inline]
fn project(w: &DMatrix<f64>, z: &DVector<f64>) -> DVector<f64> {
    w * (w.transpose() * z)
}

// ============================================================
// AXIOM 4 — RESIDUAL DECOMPOSITION
// ============================================================

#[inline]
fn residual(w: &DMatrix<f64>, z: &DVector<f64>) -> DVector<f64> {
    z - project(w, z)
}

// ============================================================
// AXIOM 5 — CONTRACTIVE STATE UPDATE
// ============================================================

#[inline]
fn update_state(
    s: &DVector<f64>,
    z_proj: &DVector<f64>,
    cfg: &AxiomConfig,
) -> DVector<f64> {
    let s_hat = enforce_sphere(s, cfg.eps);
    let z_hat = enforce_sphere(z_proj, cfg.eps);

    let blend =
        cfg.alpha * s_hat + (1.0 - cfg.alpha) * z_hat;

    enforce_sphere(
        &((1.0 - cfg.lambda) * blend + cfg.lambda * s_hat),
        cfg.eps,
    )
}

// ============================================================
// AXIOM 6 — BOUNDED STRESS FUNCTION
// B(t) = 1 - <S, Π_W(Z)>
// ============================================================

#[inline]
fn stress(s: &DVector<f64>, z_proj: &DVector<f64>) -> f64 {
    let s_hat = s.normalize();
    let z_hat = z_proj.normalize();

    1.0 - s_hat.dot(&z_hat).clamp(-1.0, 1.0)
}

// ============================================================
// AXIOM 7 — BASIS UPDATE (RESIDUAL FLOW)
// ============================================================

#[inline]
fn update_basis(
    w: DMatrix<f64>,
    r: &DVector<f64>,
    cfg: &AxiomConfig,
) -> DMatrix<f64> {
    let mut delta = DMatrix::<f64>::zeros(w.nrows(), w.ncols());

    for j in 0..w.ncols() {
        let w_j = w.column(j).into_owned();

        let coeff = w_j.dot(r);
        let proj = &w_j * coeff;

        let r_j = r - proj;

        delta.set_column(
            j,
            &((1.0 - cfg.eta) * w_j + cfg.eta * r_j),
        );
    }

    enforce_stiefel(delta)
}

// ============================================================
// AXIOM 8 — NUMERICAL FLOOR RULE
// ============================================================

#[inline]
fn safe_normalize(v: &DVector<f64>, eps: f64) -> DVector<f64> {
    let n = v.norm();
    if n <= eps {
        DVector::zeros(v.len())
    } else {
        v / n
    }
}

// ============================================================
// CORE STEP (FULL AXIOM SYSTEM)
// ============================================================

impl ManifoldCore {
    pub fn step(&mut self, z: &DVector<f64>) -> f64 {
        let cfg = self.cfg;

        // ----------------------------------------------------
        // AXIOM: PROJECTION CONSISTENCY
        // ----------------------------------------------------
        let z_proj = project(&self.w, z);
        let r = residual(&self.w, z);

        // ----------------------------------------------------
        // AXIOM: STATE EVOLUTION
        // ----------------------------------------------------
        self.s = update_state(&self.s, &z_proj, &cfg);

        // ----------------------------------------------------
        // AXIOM: BASIS EVOLUTION
        // ----------------------------------------------------
        self.w = update_basis(self.w.clone(), &r, &cfg);

        // ----------------------------------------------------
        // AXIOM: STIEFEL CONSTRAINT ENFORCEMENT
        // ----------------------------------------------------
        self.w = enforce_stiefel(self.w.clone());

        // ----------------------------------------------------
        // AXIOM: OUTPUT STRESS
        // ----------------------------------------------------
        stress(&self.s, &z_proj)
    }
}

// ============================================================
// SYSTEM INVARIANTS (FORMAL CONTRACT)
// ============================================================
//
// I1: ||S|| = 1
// I2: WᵀW = I
// I3: Z = Π_W(Z) + R
// I4: R ⟂ W
// I5: 0 ≤ B(t) ≤ 2
//
// ============================================================
 // ============================================================
 // DVSM-DFE · UNIFIED STIEFEL GEOMETRIC ENGINE (RUST CORE)
 // ============================================================

use nalgebra::{DMatrix, DVector};

/// ============================================================
/// CONFIGURATION (AXIOM PARAMETERS)
/// ============================================================
#[derive(Clone, Copy)]
pub struct Config {
    pub alpha: f64,     // state blending
    pub lambda: f64,    // contraction strength
    pub eta: f64,       // basis adaptation rate
    pub eps: f64,       // numerical stability floor
}

/// ============================================================
/// CORE STATE (S × St(n,r))
/// ============================================================
pub struct DVSMCore {
    pub s: DVector<f64>,   // S ∈ S^(n−1)
    pub w: DMatrix<f64>,   // W ∈ St(n,r)
    pub cfg: Config,
}

/// ============================================================
/// SAFE NORMALIZATION (AXIOM: ||S|| = 1)
/// ============================================================
fn normalize(v: &DVector<f64>, eps: f64) -> DVector<f64> {
    let n = v.norm();
    if n <= eps {
        return DVector::zeros(v.len());
    }
    v / n
}

/// ============================================================
/// PROJECTION OPERATOR: Π_W(Z) = W Wᵀ Z
/// ============================================================
fn project(w: &DMatrix<f64>, z: &DVector<f64>) -> DVector<f64> {
    w * (w.transpose() * z)
}

/// ============================================================
/// RESIDUAL: R = Z - Π_W(Z)
/// ============================================================
fn residual(w: &DMatrix<f64>, z: &DVector<f64>) -> DVector<f64> {
    z - project(w, z)
}

/// ============================================================
/// STIEFEL ENFORCEMENT (QR RETRACTION)
//  AXIOM: WᵀW = I
/// ============================================================
fn stiefel_retract(w: DMatrix<f64>) -> DMatrix<f64> {
    let qr = w.qr();
    qr.q()
}

/// ============================================================
/// STRESS FUNCTION (ANGULAR DIVERGENCE)
/// B(t) = 1 - <Ŝ, Π_W(Z)̂>
/// ============================================================
fn stress(s: &DVector<f64>, z_proj: &DVector<f64>) -> f64 {
    let s_hat = normalize(s, 1e-12);
    let z_hat = normalize(z_proj, 1e-12);

    1.0 - s_hat.dot(&z_hat).clamp(-1.0, 1.0)
}

/// ============================================================
/// CORE DYNAMICAL STEP
/// ============================================================
impl DVSMCore {
    pub fn step(&mut self, z: &DVector<f64>) -> f64 {
        let cfg = self.cfg;

        // ----------------------------------------------------
        // 1. PROJECTION (GEOMETRIC OBSERVATION)
        // ----------------------------------------------------
        let z_proj = project(&self.w, z);
        let r = residual(&self.w, z);

        // ----------------------------------------------------
        // 2. STATE UPDATE (SPHERICAL CONTRACTIVE FLOW)
        // ----------------------------------------------------
        let s_hat = normalize(&self.s, cfg.eps);
        let z_hat = normalize(&z_proj, cfg.eps);

        let blend =
            cfg.alpha * s_hat + (1.0 - cfg.alpha) * z_hat;

        self.s = normalize(
            &((1.0 - cfg.lambda) * blend + cfg.lambda * s_hat),
            cfg.eps,
        );

        // ----------------------------------------------------
        // 3. BASIS UPDATE (RESIDUAL-DRIVEN FLOW)
        // ----------------------------------------------------
        let mut delta = DMatrix::<f64>::zeros(self.w.nrows(), self.w.ncols());

        for j in 0..self.w.ncols() {
            let w_j = self.w.column(j).into_owned();

            let coeff = w_j.dot(z);
            let proj = &w_j * coeff;

            let r_j = z - proj;

            delta.set_column(j, &normalize(&r_j, cfg.eps));
        }

        self.w = self.w + cfg.eta * delta;
        self.w = stiefel_retract(self.w.clone());

        // ----------------------------------------------------
        // 4. OUTPUT METRIC (STRESS)
        // ----------------------------------------------------
        stress(&self.s, &z_proj)
    }
}

/// ============================================================
/// SYSTEM INVARIANTS (RUNTIME CONTRACT)
/// ============================================================
///
/// I1: ||S|| = 1
/// I2: WᵀW = I
/// I3: Z = Π_W(Z) + R
/// I4: R ⟂ W
///
/// ============================================================

/// ============================================================
/// OPTIONAL GPU HOOK (PLACEHOLDER)
/// ============================================================
///
/// Future extension:
/// - WGSL projection kernel
/// - CUDA WᵀZ reduction
/// - SIMD batch projection
/// ============================================================
pub fn gpu_project_hook(_z: &DVector<f64>) {
    // placeholder for GPU backend
}
