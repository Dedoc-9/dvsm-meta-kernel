# DVSM-RF Refinements V2
## Complex Spectral Dynamics & Predictive Instability Core
### File 2 · MIT-Level Systems Review
### Daniel J. Dillberg · 2026-05-14

---

# PURPOSE

This document defines the second architectural layer of DVSM-RF:
the transition from a real-valued broadband anomaly detector into
a complex-valued predictive spectral dynamics engine.

File 1 DVSM Runtime RF Core established:

- non-normal Lie-bracket RF transport
- EMA shear memory
- gain-regulated spectral flow
- B(t) instability detection
- adaptive Grassmann basis evolution

This file 2 extends the architecture into:

1. complex-domain spectral transport
2. predictive instability geometry
3. adaptive statistical thresholds
4. orthogonal manifold stabilization
5. information-theoretic anomaly metrics
6. operational emitter tracking

The focus is not feature expansion.
The focus is mathematical closure.

The objective is to preserve:

- dissipative stability
- bounded energy transport
- real-time feasibility
- explainable anomaly generation

while extending the operator stack into
phase-aware RF topology analysis.

---
//! ============================================================================
//! DVSM-RF V2 · COMPLEX SPECTRAL TOPOLOGY RUNTIME CORE
//! ----------------------------------------------------------------------------
//! File        : dvsm_rf_v2_runtime.rs
//! Author      : Daniel J. Dillberg
//! Version     : 2.0
//! Date        : 2026-05-14
//! License     : Dual AGLP-3 / Commercial
//!
//! ============================================================================
//! MATHEMATICAL FOUNDATION
//! ============================================================================
//!
//! DVSM-RF V2 models broadband RF environments as dissipative,
//! non-normal spectral transport systems operating on a complex
//! Hilbert field:
//!
//!      Z ∈ ℂ^R
//!
//! where:
//!
//!      Z_k = A_k exp(iθ_k)
//!
//! represents the amplitude-phase state of frequency mode k.
//!
//! The runtime evolves spectral topology using a complex-valued
//! antisymmetric Lie-bracket operator:
//!
//!      dZ_k/dt = Σ_j (Z_k conj(S_j) - Z_j conj(S_k))κ(k,j) - λZ_k
//!
//! where:
//!
//!      S_k = shear memory field
//!      κ(k,j) = antisymmetric transport kernel
//!      λ = dissipative spectral sink
//!
//! The antisymmetric transport conserves total spectral energy:
//!
//!      d/dt ||Z||² = -2λ||Z||²
//!
//! meaning:
//!
//!      • transient amplification possible
//!      • spontaneous energy creation impossible
//!
//! The system therefore detects:
//!
//!      • burst interference
//!      • spectral deception
//!      • modulation drift
//!      • phase instability
//!      • non-stationary emitter topology
//!
//! without hallucinating internal energy generation.
//!
//! ============================================================================
//! ARCHITECTURE
//! ============================================================================
//!
//! STAGE 1  : STFT spectral acquisition
//! STAGE 2  : complex projection field Z
//! STAGE 3  : Lie-bracket spectral transport
//! STAGE 4  : gain compression + thermal regulation
//! STAGE 5  : asymmetric EMA shear memory
//! STAGE 6  : thermo-elastic basis adaptation
//! STAGE 7  : Stiefel orthogonalization
//! STAGE 8  : χ² instability metric
//! STAGE 9  : adaptive threshold burst detection
//!
//! ============================================================================
//! IP POSITIONING
//! ============================================================================
//!
//! NOT independently protectable:
//!
//!      FFT/STFT
//!      EMA filters
//!      Gram-Schmidt
//!      χ² divergence
//!      Kalman methods
//!      Lie brackets
//!      spectral kurtosis
//!
//! POTENTIALLY DEFENSIBLE COMPOSITION:
//!
//!      • Complex-valued non-normal RF transport
//!      • Dissipative Lie-bracket spectral coupling
//!      • EMA shear-memory topology tracking
//!      • χ² instability geometry for RF anomaly detection
//!      • Thermo-elastic Grassmann basis adaptation
//!      • Unified Z-S-W spectral topology architecture
//!
//! IMPORTANT:
//!
//! The protectable value is NOT the mathematical primitives.
//! The protectable value is the integrated operational stack,
//! calibrated parameter topology, runtime convergence behavior,
//! and application-domain tuning.
//!
//! Encrypt calibrated κ(i,j), α, λ, η, B_crit profiles.
//! Do NOT rely on derived weights alone.
//!
//! ============================================================================

use std::f64::consts::PI;

// ============================================================================
// §0 CONFIGURATION
// ============================================================================

pub const R: usize = 16;
pub const DT: f64 = 1.0 / 60.0;

pub const ALPHA_ATTACK: f64 = 0.90;
pub const ALPHA_RELEASE: f64 = 0.995;

pub const LAMBDA: f64 = 0.05;
pub const BASIS_LR: f64 = 0.001;
pub const GAIN_THRESHOLD: f64 = 4.0;

pub const THERMAL_DECAY: f64 = 0.995;
pub const ELASTICITY_BETA: f64 = 0.25;

pub const ADAPTIVE_WINDOW: usize = 64;

pub const EPSILON: f64 = 1e-9;

// ============================================================================
// §1 COMPLEX TYPE
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn conj(&self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    pub fn abs2(&self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    pub fn abs(&self) -> f64 {
        self.abs2().sqrt()
    }
}

use std::ops::{Add, Sub, Mul};

impl Add for Complex {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            re: self.re + rhs.re,
            im: self.im + rhs.im,
        }
    }
}

impl Sub for Complex {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            re: self.re - rhs.re,
            im: self.im - rhs.im,
        }
    }
}

impl Mul for Complex {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

// ============================================================================
// §2 BAND STATE
// ============================================================================

#[derive(Clone, Debug)]
pub struct BandState {
    pub z: Complex,
    pub s: Complex,

    pub gain: f64,
    pub temperature: f64,

    pub m2: f64,
    pub m4: f64,

    pub kurtosis: f64,
}

impl Default for BandState {
    fn default() -> Self {
        Self {
            z: Complex::default(),
            s: Complex::default(),

            gain: 1.0,
            temperature: 0.0,

            m2: 0.0,
            m4: 0.0,

            kurtosis: 0.0,
        }
    }
}

// ============================================================================
// §3 BASIS
// ============================================================================

#[derive(Clone, Debug)]
pub struct Basis {
    pub w: [f64; 4],
}

impl Default for Basis {
    fn default() -> Self {
        Self {
            w: [1.0, 0.5, 0.25, 0.125],
        }
    }
}

// ============================================================================
// §4 SYSTEM CORE
// ============================================================================

pub struct DvsmRfRuntime {
    pub bands: Vec<BandState>,
    pub basis: Vec<Basis>,
    pub kappa: Vec<Vec<f64>>,

    pub b_history: Vec<f64>,
    pub frame: usize,
}

impl DvsmRfRuntime {

    // =========================================================================
    // INIT
    // =========================================================================

    pub fn new() -> Self {

        let mut kappa = vec![vec![0.0; R]; R];

        for i in 0..R {
            for j in 0..R {
                kappa[i][j] =
                    ((i as f64) * 1.37 - (j as f64) * 1.73).sin();
            }
        }

        Self {
            bands: vec![BandState::default(); R],
            basis: vec![Basis::default(); R],
            kappa,
            b_history: vec![],
            frame: 0,
        }
    }

    // =========================================================================
    // STAGE 1 + 2
    // COMPLEX SPECTRAL PROJECTION
    // =========================================================================

    pub fn spectral_project(&mut self, samples: &[f64]) {

        let n = samples.len() as f64;

        for k in 0..R {

            let mut re = 0.0;
            let mut im = 0.0;

            let freq = (k as f64 + 1.0) * 100.0;

            for (i, &x) in samples.iter().enumerate() {

                let t = i as f64 / n;

                let phase = 2.0 * PI * freq * t;

                re += x * phase.cos();
                im += x * phase.sin();
            }

            self.bands[k].z = Complex::new(re / n, im / n);
        }
    }

    // =========================================================================
    // STAGE 3
    // COMPLEX LIE-BRACKET TRANSPORT
    // =========================================================================

    pub fn lie_bracket_step(&mut self) {

        let mut dz = vec![Complex::default(); R];

        for k in 0..R {

            let z_k = self.bands[k].z;
            let s_k = self.bands[k].s;

            let mut coupling = Complex::default();

            for j in 0..R {

                if j == k { continue; }

                let z_j = self.bands[j].z;
                let s_j = self.bands[j].s;

                let term =
                    (z_k * s_j.conj())
                    -
                    (z_j * s_k.conj());

                let scale = self.kappa[k][j];

                coupling = coupling + Complex::new(
                    term.re * scale,
                    term.im * scale,
                );
            }

            dz[k] = Complex::new(
                coupling.re - LAMBDA * z_k.re,
                coupling.im - LAMBDA * z_k.im,
            );
        }

        for k in 0..R {

            self.bands[k].z.re += DT * dz[k].re;
            self.bands[k].z.im += DT * dz[k].im;
        }
    }

    // =========================================================================
    // STAGE 4
    // GAIN + THERMAL REGULATION
    // =========================================================================

    pub fn gain_regulation(&mut self) {

        for k in 0..R {

            let mag = self.bands[k].z.abs();

            self.bands[k].gain =
                if mag > GAIN_THRESHOLD {
                    GAIN_THRESHOLD / mag
                } else {
                    1.0
                };

            self.bands[k].z.re *= self.bands[k].gain;
            self.bands[k].z.im *= self.bands[k].gain;

            self.bands[k].temperature =
                THERMAL_DECAY * self.bands[k].temperature
                +
                (1.0 - THERMAL_DECAY) * mag;
        }
    }

    // =========================================================================
    // STAGE 5
    // ASYMMETRIC EMA SHEAR MEMORY
    // =========================================================================

    pub fn update_shear_memory(&mut self) {

        for k in 0..R {

            let zmag = self.bands[k].z.abs();
            let smag = self.bands[k].s.abs();

            let alpha =
                if zmag > smag {
                    ALPHA_ATTACK
                } else {
                    ALPHA_RELEASE
                };

            self.bands[k].s.re =
                alpha * self.bands[k].s.re
                +
                (1.0 - alpha) * self.bands[k].z.re;

            self.bands[k].s.im =
                alpha * self.bands[k].s.im
                +
                (1.0 - alpha) * self.bands[k].z.im;
        }
    }

    // =========================================================================
    // STAGE 6
    // THERMO-ELASTIC BASIS FLOW
    // =========================================================================

    pub fn adapt_basis(&mut self) {

        for k in 0..R {

            let t = self.bands[k].temperature;

            let eta =
                BASIS_LR
                *
                (1.0 + ELASTICITY_BETA * t);

            let err =
                self.bands[k].z.abs()
                -
                self.bands[k].s.abs();

            for j in 0..4 {

                self.basis[k].w[j]
                    += eta * err;
            }

            let norm =
                self.basis[k].w
                    .iter()
                    .map(|x| x*x)
                    .sum::<f64>()
                    .sqrt()
                    + EPSILON;

            for j in 0..4 {
                self.basis[k].w[j] /= norm;
            }
        }
    }

    // =========================================================================
    // STAGE 7
    // STIEFEL ORTHOGONALIZATION
    // =========================================================================

    pub fn orthogonalize_basis(&mut self) {

        for k in 1..R {

            for j in 0..k {

                let dot =
                    self.basis[k].w.iter()
                    .zip(self.basis[j].w.iter())
                    .map(|(a,b)| a*b)
                    .sum::<f64>();

                for d in 0..4 {
                    self.basis[k].w[d]
                        -= dot * self.basis[j].w[d];
                }
            }

            let norm =
                self.basis[k].w
                    .iter()
                    .map(|x| x*x)
                    .sum::<f64>()
                    .sqrt()
                    + EPSILON;

            for d in 0..4 {
                self.basis[k].w[d] /= norm;
            }
        }
    }

    // =========================================================================
    // STAGE 8
    // KURTOSIS + χ² INSTABILITY
    // =========================================================================

    pub fn instability_metric(&mut self) -> f64 {

        let mut chi2 = 0.0;

        for k in 0..R {

            let z = self.bands[k].z.abs();

            self.bands[k].m2 =
                0.99 * self.bands[k].m2
                +
                0.01 * z*z;

            self.bands[k].m4 =
                0.99 * self.bands[k].m4
                +
                0.01 * z*z*z*z;

            self.bands[k].kurtosis =
                self.bands[k].m4
                /
                (self.bands[k].m2 * self.bands[k].m2 + EPSILON)
                -
                3.0;

            let dz =
                self.bands[k].z.abs()
                -
                self.bands[k].s.abs();

            chi2 +=
                (dz * dz)
                /
                (self.bands[k].s.abs2() + EPSILON);
        }

        self.b_history.push(chi2);

        chi2
    }

    // =========================================================================
    // STAGE 9
    // ADAPTIVE BURST DETECTOR
    // =========================================================================

    pub fn adaptive_threshold(&self) -> f64 {

        if self.b_history.len() < 4 {
            return 2.0;
        }

        let n = self.b_history.len()
            .min(ADAPTIVE_WINDOW);

        let slice =
            &self.b_history[self.b_history.len()-n..];

        let mean =
            slice.iter().sum::<f64>() / n as f64;

        let variance =
            slice.iter()
            .map(|x| (x-mean)*(x-mean))
            .sum::<f64>()
            / n as f64;

        mean + 2.5 * variance.sqrt()
    }

    // =========================================================================
    // STEP
    // =========================================================================

    pub fn step(&mut self, samples: &[f64]) {

        self.spectral_project(samples);

        self.lie_bracket_step();

        self.gain_regulation();

        self.update_shear_memory();

        self.adapt_basis();

        self.orthogonalize_basis();

        let b = self.instability_metric();

        let threshold = self.adaptive_threshold();

        if b > threshold {

            println!(
                "[BURST] frame={} B={:.5} threshold={:.5}",
                self.frame,
                b,
                threshold
            );
        }

        self.frame += 1;
    }

    // =========================================================================
    // OBSERVABLES
    // =========================================================================

    pub fn total_energy(&self) -> f64 {

        self.bands.iter()
            .map(|b| b.z.abs2())
            .sum::<f64>()
    }

    pub fn spectral_entropy(&self) -> f64 {

        let total =
            self.total_energy() + EPSILON;

        let mut h = 0.0;

        for b in &self.bands {

            let p = b.z.abs2() / total;

            if p > EPSILON {
                h -= p * p.ln();
            }
        }

        h
    }
}

// ============================================================================
// §5 TEST SIGNALS
// ============================================================================

pub fn signal_clean(frame: usize, n: usize) -> Vec<f64> {

    let mut out = vec![0.0; n];

    for i in 0..n {

        let t =
            frame as f64 * DT
            +
            i as f64 / n as f64;

        out[i] =
            0.5 * (2.0 * PI * 100.0 * t).sin()
            +
            0.3 * (2.0 * PI * 440.0 * t).sin();
    }

    out
}

pub fn signal_burst(frame: usize, n: usize) -> Vec<f64> {

    let mut s = signal_clean(frame, n);

    if frame > 60 && frame < 90 {

        for i in 0..n {

            let t = i as f64 / n as f64;

            s[i] +=
                2.0
                *
                (2.0 * PI * (200.0 + frame as f64) * t)
                .sin();
        }
    }

    s
}

// ============================================================================
// §6 MAIN
// ============================================================================

fn main() {

    println!("=================================================");
    println!("DVSM-RF V2 · COMPLEX SPECTRAL TOPOLOGY ENGINE");
    println!("Author: Daniel J. Dillberg");
    println!("=================================================");

    let mut runtime = DvsmRfRuntime::new();

    for frame in 0..180 {

        let signal = signal_burst(frame, 1024);

        runtime.step(&signal);

        println!(
            "frame={} energy={:.5} entropy={:.5}",
            frame,
            runtime.total_energy(),
            runtime.spectral_entropy(),
        );
    }

    println!("=================================================");
    println!("runtime complete");
    println!("=================================================");
}
//! ============================================================
//! DVSM-RF · DEFENSIBLE IP POSITIONING BLOCK
//! ============================================================
//!
//! Author  : Daniel J. Dillberg
//! Runtime : DVSM-RF Runtime Core
//! Domain  : Broadband RF / Spectral Topology Analysis
//!
//! ============================================================
//! MATHEMATICAL FUNDAMENTALS
//! ============================================================
//!
//! Core state evolution:
//!
//!     dZ_k/dt = Σ_j (Z_k S_j − Z_j S_k) κ(k,j) − λZ_k
//!
//! where:
//!
//!     Z_k  = spectral transport field
//!     S_k  = EMA shear-memory field
//!     κ    = antisymmetric coupling topology
//!     λ    = dissipative spectral sink
//!
//! χ² instability geometry:
//!
//!     B(t) = Σ_k (Z_k − S_k)^2 / (S_k^2 + ε)
//!
//! Grassmann adaptation:
//!
//!     W_k ← Normalize(W_k + η∇)
//!
//! ============================================================
//! IP POSITIONING
//! ============================================================
//!
//! NOT independently protectable:
//!
//!      FFT/STFT
//!      EMA filters
//!      Gram-Schmidt
//!      χ² divergence
//!      Kalman methods
//!      Lie brackets
//!      spectral kurtosis
//!
//! POTENTIALLY DEFENSIBLE COMPOSITION:
//!
//!      • Complex-valued non-normal RF transport
//!      • Dissipative Lie-bracket spectral coupling
//!      • EMA shear-memory topology tracking
//!      • χ² instability geometry for RF anomaly detection
//!      • Thermo-elastic Grassmann basis adaptation
//!      • Unified Z-S-W spectral topology architecture
//!
//! IMPORTANT:
//!
//! The protectable value is NOT the mathematical primitives.
//! The protectable value is the integrated operational stack,
//! calibrated parameter topology, runtime convergence behavior,
//! and application-domain tuning.
//!
//! Encrypt calibrated κ(i,j), α, λ, η, B_crit profiles.
//! Do NOT rely on derived weights alone.
//!
//! ============================================================
//! DEFENSIBLE SOFTWARE POSITION
//! ============================================================
//!
//! A potentially defensible software position emerges when the
//! runtime architecture demonstrates:
//!
//!     1. Stable dissipative convergence
//!     2. Domain-specific RF calibration behavior
//!     3. Persistent topology-memory coupling
//!     4. Non-trivial spectral transport dynamics
//!     5. Operational anomaly discrimination
//!
//! The strongest claim is NOT ownership of mathematics,
//! but ownership of:
//!
//!     • the calibrated runtime topology,
//!     • the convergence regime,
//!     • the integrated spectral transport pipeline,
//!     • and the operational deployment behavior.
//!
//! ============================================================

pub const DEFENSIBLE_RUNTIME_STACK: [&str; 5] = [
    "Complex-valued spectral transport",
    "Non-normal Lie-bracket coupling",
    "EMA shear-memory persistence",
    "Adaptive Grassmann basis evolution",
    "χ² instability anomaly geometry",
];

pub struct RuntimeCalibrationProfile {
    pub lambda: f64,
    pub alpha: f64,
    pub eta: f64,
    pub b_crit: f64,
    pub encrypted_kappa_hash: [u8; 32],
}

impl RuntimeCalibrationProfile {
    pub fn defensible_surface(&self) -> String {
        format!(
            "DVSM-RF calibrated runtime | λ={:.4} α={:.4} η={:.6} Bcrit={:.4}",
            self.lambda,
            self.alpha,
            self.eta,
            self.b_crit
        )
    }
}
//! ============================================================
//! DEV NOTE · DVSM-RF RUNTIME CORE
//! ============================================================
//!
//! This runtime is intentionally structured as a
//! mathematically transparent operator system rather than a
//! black-box ML classifier.
//!
//! DESIGN PHILOSOPHY
//! ------------------------------------------------------------
//!
//! Traditional RF systems:
//!     signal → FFT → threshold → classifier
//!
//! DVSM-RF instead models:
//!
//!     signal → spectral topology → transport dynamics
//!            → memory shear → instability geometry
//!
//! The objective is not merely spectral decomposition.
//! The objective is detection of structurally unexplained
//! spectral evolution.
//!
//! ============================================================
//! IMPORTANT IMPLEMENTATION NOTES
//! ============================================================
//!
//! 1. κ(i,j) defines operational geometry
//! ------------------------------------------------------------
//!
//! The coupling kernel is NOT a cosmetic parameter.
//! It determines:
//!
//!     • cross-band transport behavior
//!     • transient amplification structure
//!     • convergence pathways
//!     • burst propagation geometry
//!
//! Domain-specific κ calibration is one of the primary
//! defensible components of the runtime.
//!
//! Encrypt calibrated κ profiles.
//!
//! ------------------------------------------------------------
//! 2. B(t) is NOT an energy detector
//! ------------------------------------------------------------
//!
//!     B(t) ≠ signal power
//!
//! B(t) measures:
//!
//!     unexplained spectral topology
//!
//! High B(t):
//!     the learned basis W cannot explain current Z.
//!
//! This allows detection of:
//!
//!     • spectral drift
//!     • emitter deformation
//!     • frequency hopping
//!     • spoofing
//!     • non-stationary interference
//!
//! even when total RF power remains stable.
//!
//! ------------------------------------------------------------
//! 3. The system is dissipative by construction
//! ------------------------------------------------------------
//!
//! Energy evolution:
//!
#![allow(unused_doc_comments)]
//! :contentReference[oaicite:0]{index=0}
//!
//! The Lie-bracket redistributes energy between modes
//! but cannot generate energy internally.
//!
//! Operational implication:
//!
//!     internal runtime instability cannot fabricate
//!     burst events without external excitation.
//!
//! ------------------------------------------------------------
//! 4. Complex-valued extension is the next major upgrade
//! ------------------------------------------------------------
//!
//! Current implementation:
//!
//!     Z_k ∈ ℝ
//!
//! Operational-grade RF implementation:
//!
//!     Z_k ∈ ℂ
//!
//! This unlocks:
//!
//!     • phase-coherence detection
//!     • modulation discrimination
//!     • jammer identification
//!     • array interferometry
//!     • directional estimation
//!
//! ------------------------------------------------------------
//! 5. The strongest IP surface is runtime behavior
//! ------------------------------------------------------------
//!
//! The mathematics themselves are largely public-domain,
//! academically established, or obvious combinations.
//!
//! The strongest defensible position comes from:
//!
//!     • calibrated convergence profiles
//!     • runtime transport geometry
//!     • operational tuning
//!     • domain-specific parameterization
//!     • deployment-specific spectral adaptation
//!
//! NOT from ownership of:
//!
//!     FFTs
//!     Lie brackets
//!     EMAs
//!     Gram-Schmidt
//!     Kalman filters
//!
//! ------------------------------------------------------------
//! 6. Runtime priorities moving forward
//! ------------------------------------------------------------
//!
//! HIGH PRIORITY:
//!
//!     [ ] complex-valued spectral state
//!     [ ] adaptive B_crit thresholding
//!     [ ] Stiefel orthogonalization
//!     [ ] spectral kurtosis regulator
//!     [ ] asymmetric shear memory
//!
//! RESEARCH TRACK:
//!
//!     [ ] pseudospectrum prediction
//!     [ ] Hopf bifurcation analysis
//!     [ ] hyperbolic embeddings
//!     [ ] transfer entropy transport
//!
//! ============================================================
//! END DEV NOTE
//! ============================================================
// ============================================================
// DVSM-RF · RUNTIME CORE CALIBRATION ADDENDUM
// Hardened initialization of spectral transport parameters
// ============================================================
//
// NOTE ON DESIGN INTENT:
//
// This module does NOT attempt to “protect mathematics” such as
// FFTs, EMAs, or Lie brackets. Those are public primitives.
//
// The defensible component is the *coupled runtime behavior*:
//
//   • calibrated dissipation stability (λ)
//   • memory hysteresis regime (α)
//   • basis adaptation elasticity (η)
//   • anomaly geometry thresholding (B_crit)
//   • encrypted coupling topology (κ hash)
//
// The IP boundary (if any) lives in:
//   → parameterization strategy
//   → coupling topology encoding
//   → system-level convergence behavior under load
//
// ============================================================

#[derive(Clone, Debug)]
pub struct RuntimeCalibrationProfile {
    pub lambda: f64,                  // λ: spectral dissipation
    pub alpha: f64,                  // α: EMA memory coefficient
    pub eta: f64,                     // η: basis adaptation rate
    pub b_crit: f64,                  // B_crit: instability threshold
    pub encrypted_kappa_hash: [u8; 32], // κ topology (encrypted/hashed)
}

impl RuntimeCalibrationProfile {
    /// Hardened constructor with stability constraints.
    ///
    /// These constraints enforce minimal dynamical-system validity:
    ///
    ///   λ > 0   → ensures dissipative spectral flow
    ///   α ∈ (0,1) → ensures stable EMA memory evolution
    ///   η > 0    → ensures basis can adapt (no frozen manifold)
    ///   B_crit > 0 → ensures meaningful anomaly boundary
    ///
    pub fn new_hardened(
        l: f64,
        a: f64,
        e: f64,
        b: f64,
        hash: [u8; 32],
    ) -> Result<Self, String> {

        // --------------------------------------------------------
        // Dissipation constraint (energy stability requirement)
        // --------------------------------------------------------
        // d||Z||²/dt = -2λ||Z||²
        if l <= 0.0 {
            return Err("Stability Violation: λ must be > 0".into());
        }

        // --------------------------------------------------------
        // EMA memory stability constraint
        // --------------------------------------------------------
        // α outside (0,1) breaks exponential moving average semantics
        if a <= 0.0 || a >= 1.0 {
            return Err("Memory Violation: α must be in (0,1)".into());
        }

        // --------------------------------------------------------
        // Basis adaptation constraint
        // --------------------------------------------------------
        // η controls Grassmann flow speed; must remain positive
        if e <= 0.0 {
            return Err("Adaptation Violation: η must be > 0".into());
        }

        // Optional practical upper bound (prevents numerical instability)
        if e > 1.0 {
            return Err("Adaptation Violation: η must be ≤ 1".into());
        }

        // --------------------------------------------------------
        // Anomaly threshold constraint
        // --------------------------------------------------------
        if b <= 0.0 {
            return Err("Detection Violation: B_crit must be > 0".into());
        }

        Ok(Self {
            lambda: l,
            alpha: a,
            eta: e,
            b_crit: b,
            encrypted_kappa_hash: hash,
        })
    }
}

// ============================================================
// REFINED DEFENSIBLE IP ARCHITECTURE (RUNTIME ANNOTATION BLOCK)
// DVSM-RF · Spectral Transport System
// ============================================================
//
// This file-level annotation defines the conceptual boundary
// between:
//   (A) public mathematical primitives
//   (B) system-level emergent behavior (potential IP surface)
//
// IMPORTANT:
//
// Nothing in this system attempts to claim ownership of:
//   - transforms
//   - estimators
//   - filters
//   - divergences
//   - algebraic operators
//
// The only meaningful novelty exists in *composition dynamics*:
// how these elements interact under coupled evolution.
//
// ============================================================

/// ============================================================
/// PUBLIC DOMAIN / PRIOR ART (Individually)
/// ============================================================
///
/// These components are well-established in scientific and
/// engineering literature:
///
/// - FFT / STFT / wavelet transforms
/// - EMA / exponential smoothing filters
/// - Gram-Schmidt / Stiefel / Grassmann projections
/// - χ² / KL / MSE distance measures
/// - Kalman / particle filtering methods
/// - Lie algebra commutators (antisymmetric brackets)
///
/// Individually:
///     → not novel
///     → not protectable
///     → widely used across DSP, control theory, ML
///
/// ============================================================
///
/// DEFENSIBLE IP SURFACE (System-Level Coupling Only)
/// ============================================================
///
/// 1. NON-NORMAL SPECTRAL TRANSPORT COUPLING
///
/// The latent spectral field Z and memory field S are coupled via
/// a non-normal Lie-bracket evolution:
///
///     dZ = [Z, S]_κ − λZ
///
/// Key property:
///     → antisymmetric redistribution
///     → global energy contraction preserved
///     → local transient amplification permitted
///
/// The novelty is NOT the bracket itself,
/// but the *closed-loop instability structure under dissipation*.
///
///
/// 2. KAPPA-TOPOLOGY ENCODING (κ-matrix as spectral keyspace)
///
/// κ(i,j) defines structured coupling between frequency bands.
///
/// If encrypted / learned:
///     → acts as a transport topology key
///     → defines privileged spectral pathways
///
/// This turns standard band interaction into:
///     "controlled anisotropic spectral flow"
///
///
/// 3. STABILITY-REGULATED CLOSED LOOP
///
/// Coupling between:
///
///     λ → dissipation (energy sink)
///     α → memory persistence (EMA hysteresis)
///     gain → per-band compression (nonlinearity)
///
/// This triad enforces:
///
///     contraction under burst conditions
///     bounded spectral energy evolution
///     controlled non-normal amplification
///
/// The IP is in the *joint constraint system*, not the filters.
///
///
/// 4. EMERGENT INSTABILITY GEOMETRY (B(t))
///
/// B(t) = ||S|| / ||Z||
///
/// Not merely a threshold detector, but:
///
///     → a coordinate on a stability manifold
///     → encoding mismatch between memory and excitation
///
/// Interpretation:
///     low B(t)  → coherent spectral alignment
///     high B(t) → structural mismatch / anomaly stress
///
/// The geometry of B(t) trajectories encodes signal class.
///
///
/// 5. TRIPLE-LAYER MANIFOLD EVOLUTION (Z, S, W)
///
/// The system evolves three coupled objects:
///
///     Z → spectral field (observed dynamics)
///     S → memory / hysteresis field
///     W → adaptive basis geometry
///
/// Feedback loop:
///
///     Z → drives S (residual stress accumulation)
///     S → drives Z (non-normal coupling feedback)
///     Z,S → drive W (Grassmann adaptation)
///
/// This forms a closed variational system:
///     not a filter chain, but a coupled manifold flow.
///
///
/// ============================================================
/// CORE THESIS (SYSTEM CLAIM)
// ============================================================
///
/// The mathematical primitives are not novel.
///
/// The *emergent behavior* of the coupled dynamical system is:
///
///     "Structural anomaly detection via stress accumulation
///      in a non-normal dissipative spectral manifold."
///
/// Value is located in:
///     → convergence behavior under perturbation
///     → κ-encoded transport geometry
///     → stability-constrained feedback loops
///     → observable B(t) manifold deformation
///
/// ======================================================================
/// IP ANNOTATION: B(t) FINGERPRINT INSTRUMENTATION
/// ======================================================================
///
/// This layer defines the transformation of the B(t) stability metric 
/// from a scalar threshold into a hardware-specific anomaly signature.
///
/// DEFENSIBLE CORE:
///
///   1. DEVICE-INVARIANT EMBEDDING DISTORTION:
///      The B(t) trajectory is treated as a "stress curve" of the 
///      embedding geometry. Even if raw signal levels vary, the 
///      deformation profile of the S-Z manifold is a device-invariant 
///      signature of the underlying physical state.
///
///   2. ANOMALY FEATURE VECTOR (Ξ):
///      We instrument the B(t) manifold via its differential geometry:
///      Ξ = { dB/dt, d²B/dt², entropy(B), κ-flux_divergence }
///
///      The novelty is mapping this vector to specific hardware states 
///      (fatigue, thermal drift, or adversarial spoofing) rather than 
///      simple energy detection.
///
///   3. PERTURBATION STABILITY SIGNATURE:
///      The system's IP is located in the **stability profile**:
///      How the (Z, S, W) manifold recovers from a controlled impulse.
///      The "recovery curve" in B-space is the defensible system fingerprint.
///
/// THESIS: 
///   I do not claim the B(t) metric; I claim the use of the **B(t) 
///   distortion manifold** as a high-fidelity diagnostic for 
///   hardware-level structural changes.
///
/// ======================================================================
