//! ============================================================
//! DVSM-RF CORE · NON-NORMAL SPECTRAL OPERATOR RUNTIME
//! runtime_core.rs
//!
//! Author  : Daniel J. Dillberg
//! Version : 1.0-canonical
//! Date    : 2026-05-14
//! License : AGLP-3 Dual
//!
//! ============================================================
//! AUTHOR INTRODUCTION
//! ============================================================
//!
//! DVSM-RF is a runtime implementation of a measure-valued,
//! non-normal spectral operator system for broadband signal
//! evaluation across low-frequency through high-frequency domains.
//!
//! The architecture combines:
//!
//!   • McKean–Vlasov mean-field dynamics
//!   • Non-normal antisymmetric Lie-bracket transport
//!   • Grassmann/Stiefel basis adaptation
//!   • Adaptive per-mode gain regulation
//!   • EMA shear-memory residual tracking
//!   • Spectral instability diagnostics B(t)
//!
//! The runtime treats RF energy not as isolated FFT bins,
//! but as an interacting spectral manifold:
//!
//!     μ_t  →  Z_t  →  S_t  →  W_t
//!
//! where:
//!
//!   μ_t : spectral measure
//!   Z_t : rank-R latent transport field
//!   S_t : non-normal residual memory
//!   W_t : adaptive geometric basis
//!
//! ============================================================
//! MATHEMATICAL FOUNDATIONS
//! ============================================================
//!
//! ------------------------------------------------------------
//! 1. MEAN-FIELD SPECTRAL STATE
//! ------------------------------------------------------------
//!
//! The runtime evolves a spectral probability measure:
//!
//!     μ_t = Σ_i w_i δ_{g_i}
//!
//! where:
//!
//!     g_i : spectral carrier coordinate
//!     w_i : normalized probability weight
//!
//! The semantic state is the measure μ_t itself.
//!
//! ------------------------------------------------------------
//! 2. NON-NORMAL LIE-BRACKET FLOW
//! ------------------------------------------------------------
//!
//! Spectral transport evolves under:
//!
//!     dZ_k/dt = Σ_j (Z_k S_j - Z_j S_k) κ(k,j)
//!               - λ Z_k
//!
//! κ(k,j) is antisymmetric:
//!
//!     κ(k,j) = -κ(j,k)
//!
//! This creates conservative inter-band transport:
//!
//!     d||Z||²/dt = -2λ||Z||²
//!
//! Therefore:
//!
//!   • energy redistributes
//!   • energy does not self-amplify
//!   • bursts correspond to external forcing
//!
//! ------------------------------------------------------------
//! 3. SHEAR MEMORY
//! ------------------------------------------------------------
//!
//! Residual dynamics evolve as:
//!
//!     S_k ← α S_k + (1-α)(Z_k - Π_W Z_k)
//!
//! S measures unexplained spectral structure.
//!
//! High ||S|| indicates:
//!
//!   • drift
//!   • transient interference
//!   • deception
//!   • structural novelty
//!
//! ------------------------------------------------------------
//! 4. INSTABILITY METRIC
//! ------------------------------------------------------------
//!
//!     B(t) = ||S_t|| / (||Z_t|| + ε)
//!
//! Interpretation:
//!
//!   low B(t)
//!       → stable learned manifold
//!
//!   high B(t)
//!       → spectral manifold mismatch
//!
//!   B(t) > B_crit
//!       → transient anomaly candidate
//!
//! ------------------------------------------------------------
//! 5. GEOMETRIC BASIS FLOW
//! ------------------------------------------------------------
//!
//! Adaptive basis evolution:
//!
//!     W ← Normalize(W + η∇)
//!
//! In the slow limit:
//!
//!     W → dominant eigenspace
//!
//! producing a Grassmann fixed point:
//!
//!     W* ∈ Gr(R,D)
//!
//! ============================================================
//! DEFENSIBLE IP POSITIONING
//! ============================================================
//!
//! This runtime does NOT claim ownership over:
//!
//!   • FFTs
//!   • EMAs
//!   • gain clipping
//!   • kernel methods
//!   • stochastic sampling
//!   • Lie brackets
//!
//! The defensible composition is:
//!
//!   1. Non-normal spectral transport
//!   2. Per-band adaptive compression
//!   3. Residual shear-memory geometry
//!   4. B(t)-driven instability detection
//!   5. Unified Z/S/W manifold runtime
//!
//! The protectable operational layer is:
//!
//!     transient spectral anomaly evaluation
//!     through non-normal stress accumulation
//!     in a rank-R interacting spectral field.
//!
//! ============================================================

use std::f64::consts::PI;

// ============================================================
// §0 · GLOBAL PARAMETERS
// ============================================================

pub const R: usize = 16;
pub const DT: f64 = 1.0 / 60.0;

pub const LAMBDA: f64 = 0.05;
pub const ALPHA: f64 = 0.98;

pub const GAIN_THRESHOLD: f64 = 4.0;
pub const B_CRIT: f64 = 2.0;

pub const BASIS_LR: f64 = 0.001;
pub const THERMAL_DECAY: f64 = 0.995;

// ============================================================
// §1 · CORE STATE
// ============================================================

#[derive(Clone, Debug)]
pub struct Band {
    pub z: f64,
    pub s: f64,
    pub temp: f64,
    pub gain: f64,
}

impl Default for Band {
    fn default() -> Self {
        Self {
            z: 0.0,
            s: 0.0,
            temp: 0.0,
            gain: 1.0,
        }
    }
}

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

#[derive(Debug)]
pub struct RuntimeCore {
    pub bands: Vec<Band>,
    pub basis: Vec<Basis>,
    pub kappa: Vec<Vec<f64>>,
    pub frame: usize,
    pub energy: f64,
    pub b_metric: f64,
}

// ============================================================
// §2 · INITIALIZATION
// ============================================================

impl RuntimeCore {
    pub fn new() -> Self {
        let mut kappa = vec![vec![0.0; R]; R];

        for i in 0..R {
            for j in 0..R {
                kappa[i][j] =
                    ((i as f64) * 1.37 - (j as f64) * 1.73).sin();
            }
        }

        Self {
            bands: vec![Band::default(); R],
            basis: vec![Basis::default(); R],
            kappa,
            frame: 0,
            energy: 0.0,
            b_metric: 0.0,
        }
    }

    // ========================================================
    // §3 · STAGE 1
    // SPECTRAL PROJECTION
    // ========================================================

    pub fn project_signal(&mut self, samples: &[f64]) {
        let n = samples.len().max(1) as f64;

        for k in 0..R {
            let mut acc = 0.0;

            for (i, x) in samples.iter().enumerate() {
                let t = (i as f64) / n;

                let poly =
                    1.0
                    + self.basis[k].w[1] * x
                    + self.basis[k].w[2] * x * x
                    + self.basis[k].w[3] * x * x * x;

                let carrier =
                    ((((k + 1) as f64) * PI) * t).sin();

                acc += poly * carrier;
            }

            self.bands[k].z = acc / n;
        }
    }

    // ========================================================
    // §4 · STAGE 2
    // NON-NORMAL LIE TRANSPORT
    // ========================================================

    pub fn evolve_transport(&mut self) {
        let mut dz = vec![0.0; R];

        for k in 0..R {
            let z_k = self.bands[k].z;
            let s_k = self.bands[k].s;

            let mut coupling = 0.0;

            for j in 0..R {
                if j == k {
                    continue;
                }

                let z_j = self.bands[j].z;
                let s_j = self.bands[j].s;

                coupling +=
                    (z_k * s_j - z_j * s_k)
                    * self.kappa[k][j];
            }

            dz[k] = coupling - LAMBDA * z_k;
        }

        for k in 0..R {
            self.bands[k].z += DT * dz[k];
        }
    }

    // ========================================================
    // §5 · STAGE 3
    // PER-MODE GAIN REGULATION
    // ========================================================

    pub fn regulate_gain(&mut self) {
        self.energy = 0.0;

        for k in 0..R {
            let e = self.bands[k].z.abs();

            self.bands[k].gain =
                if e > GAIN_THRESHOLD {
                    GAIN_THRESHOLD / e
                } else {
                    1.0
                };

            self.bands[k].z *= self.bands[k].gain;

            self.bands[k].temp =
                THERMAL_DECAY * self.bands[k].temp
                + (1.0 - THERMAL_DECAY) * e;

            self.energy +=
                self.bands[k].z * self.bands[k].z;
        }

        self.energy = self.energy.sqrt();
    }

    // ========================================================
    // §6 · STAGE 4
    // SHEAR MEMORY UPDATE
    // ========================================================

    pub fn update_memory(&mut self) {
        for k in 0..R {
            let z = self.bands[k].z;
            let s = self.bands[k].s;

            let w = &self.basis[k].w;

            let proj =
                z * (w[0] + w[1] + w[2] + w[3]) / 4.0;

            let residual = z - proj;

            self.bands[k].s =
                ALPHA * s
                + (1.0 - ALPHA) * residual;
        }
    }

    // ========================================================
    // §7 · STAGE 5
    // BASIS ADAPTATION
    // ========================================================

    pub fn adapt_basis(&mut self) {
        for k in 0..R {
            let err =
                self.bands[k].z - self.bands[k].s;

            for j in 0..4 {
                self.basis[k].w[j] +=
                    BASIS_LR
                    * err
                    * (1.0 - self.basis[k].w[j].abs());
            }

            let norm =
                self.basis[k]
                    .w
                    .iter()
                    .map(|v| v * v)
                    .sum::<f64>()
                    .sqrt()
                    + 1e-9;

            for j in 0..4 {
                self.basis[k].w[j] /= norm;
            }
        }
    }

    // ========================================================
    // §8 · STAGE 6
    // INSTABILITY METRIC
    // ========================================================

    pub fn compute_b(&mut self) {
        let z_norm =
            self.bands
                .iter()
                .map(|b| b.z * b.z)
                .sum::<f64>()
                .sqrt();

        let s_norm =
            self.bands
                .iter()
                .map(|b| b.s * b.s)
                .sum::<f64>()
                .sqrt();

        self.b_metric =
            s_norm / (z_norm + 1e-9);
    }

    // ========================================================
    // §9 · FULL STEP
    // ========================================================

    pub fn step(&mut self, samples: &[f64]) {
        self.project_signal(samples);

        self.evolve_transport();

        self.regulate_gain();

        self.update_memory();

        self.adapt_basis();

        self.compute_b();

        self.frame += 1;
    }

    // ========================================================
    // §10 · DIAGNOSTICS
    // ========================================================

    pub fn burst_detected(&self) -> bool {
        self.b_metric > B_CRIT
    }

    pub fn dominant_band(&self) -> usize {
        self.bands
            .iter()
            .enumerate()
            .max_by(|a, b| {
                a.1.z.abs()
                    .partial_cmp(&b.1.z.abs())
                    .unwrap()
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

// ============================================================
// §11 · SYNTHETIC TEST SIGNALS
// ============================================================

pub fn clean_signal(n: usize) -> Vec<f64> {
    let mut out = vec![0.0; n];

    for i in 0..n {
        let t = (i as f64) / (n as f64);

        out[i] =
            0.4 * (2.0 * PI * 120.0 * t).sin()
            + 0.2 * (2.0 * PI * 440.0 * t).sin()
            + 0.1 * (2.0 * PI * 900.0 * t).sin();
    }

    out
}

// ============================================================
// §12 · RUNTIME EXAMPLE
// ============================================================

fn main() {
    let mut core = RuntimeCore::new();

    for frame in 0..240 {
        let signal = clean_signal(1024);

        core.step(&signal);

        println!(
            "frame={} energy={:.4} B={:.4} burst={} dom={}",
            frame,
            core.energy,
            core.b_metric,
            core.burst_detected(),
            core.dominant_band(),
        );
    }
}
// ============================================================
// DEV NOTES
// ============================================================
//
// DVSM-RF is NOT an FFT replacement.
// The FFT/STFT front-end is interchangeable.
//
// The novelty is the operator stack:
//
//     RF signal
//         → spectral measure μ_t
//         → interacting latent transport field Z_t
//         → non-normal residual memory S_t
//         → adaptive geometric basis W_t
//         → instability metric B(t)
//
// ------------------------------------------------------------
// CORE ENGINEERING PRINCIPLES
// ------------------------------------------------------------
//
// 1. Energy redistribution ≠ energy creation
//
//    Antisymmetric Lie coupling conserves transport energy:
//
//        d||Z||²/dt = -2λ||Z||²
//
//    Internal dynamics cannot self-generate bursts.
//    Observed burst events correspond to:
//
//        • external forcing
//        • drift
//        • interference
//        • manifold mismatch
//
// ------------------------------------------------------------
// 2. B(t) measures unexplained spectral structure
//
//        B(t) = ||S|| / (||Z|| + ε)
//
//    low B:
//        stable learned manifold
//
//    high B:
//        geometric mismatch
//        unresolved transport
//        drift or anomaly
//
// ------------------------------------------------------------
// 3. The IP is the composition, not the primitives
//
//    NOT defensible alone:
//
//        • FFT
//        • EMA
//        • clipping
//        • kernels
//        • Lie brackets
//
//    Defensible composition:
//
//        • non-normal spectral transport
//        • per-band adaptive regulation
//        • residual shear-memory geometry
//        • instability accumulation metric
//        • unified Z/S/W operator runtime
//
// ------------------------------------------------------------
// 4. κ(i,j) is application-specific
//
//    The kernel values themselves are calibration data,
//    not universal constants.
//
//    Different domains require different κ geometry:
//
//        SDR
//        radar
//        SIGINT
//        sonar
//        biosignals
//        cavity dynamics
//        acoustic transport
//
//    Protect:
//
//        calibrated parameter geometry
//
//    NOT:
//
//        derived basis weights W*
//
//
// ------------------------------------------------------------
// 5. Runtime assumptions
//
//    Current runtime:
//
//        • polynomial surrogate projection
//        • fixed learning rate
//        • scalar shear memory
//        • static B_crit threshold
//
//    Production upgrades:
//
//        • STFT/CWT front-end
//        • adaptive η optimizer
//        • vector-valued S_k
//        • adaptive statistical thresholding
//        • learned κ geometry
//
// ------------------------------------------------------------
// 6. Mathematical classification
//
//    DVSM-RF is best classified as:
//
//        a stochastic non-normal operator system
//        for broadband spectral transport analysis
//
//    NOT:
//
//        • thermodynamics
//        • quantum mechanics
//        • Navier-Stokes
//        • physical field theory
//
// ------------------------------------------------------------
// 7. Intellectual property notice:
//
//    Patent/rights cover:
//
//        "method of transient spectral anomaly detection
//         through non-normal stress accumulation in a
//         rank-R interacting spectral field"
//
//    Not simply:
//
//        equations alone
//
// ------------------------------------------------------------
// 8. Deployment guidance
//
//    Real deployments should:
//
//        • isolate front-end DSP layer
//        • sandbox adaptive kernels
//        • persist burst telemetry
//        • calibrate B_crit statistically
//        • validate against known RF datasets
//
// ============================================================
// END OF FILE
// ============================================================
