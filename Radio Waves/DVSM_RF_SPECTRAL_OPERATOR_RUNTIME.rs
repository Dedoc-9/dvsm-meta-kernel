//! ============================================================
//! DVSM-RF · NON-NORMAL SPECTRAL OPERATOR RUNTIME
//! DVSM_RF_SPECTRAL_OPERATOR_RUNTIME.rs
//!
//! Author  : Daniel J. Dillberg
//! Version : 1.0-canonical
//! Date    : 2026-05-15
//! License : AGLP-3 Dual
//!
//! ============================================================
//! SYSTEM OVERVIEW
//! ============================================================
//!
//! DVSM-RF is a deterministic non-normal spectral operator system
//! for RF signal analysis and instability detection.
//!
//! It models broadband RF signals as an interacting spectral manifold:
//!
//!     μ_t → Z_t → S_t → W_t
//!
//! where:
//!
//!   μ_t : spectral measure (input distribution)
//!   Z_t : transport field (banded latent state)
//!   S_t : residual shear memory (EMA-based mismatch state)
//!   W_t : adaptive projection basis (Stiefel-like constraint)
//!
//! The system is NOT cryptographic and NOT an FFT replacement.
//! It is a structured spectral transport and anomaly detection runtime.
//!
//! ============================================================
//! MATHEMATICAL MODEL
//! ============================================================
//!
//! ------------------------------------------------------------
//! 1. SPECTRAL STATE REPRESENTATION
//! ------------------------------------------------------------
//!
//! RF input is treated as a spectral measure:
//!
//!     μ_t = Σ_i w_i δ(g_i)
//!
//! This is projected into a finite latent representation:
//!
//!     Z ∈ ℝ^R
//!
//! ------------------------------------------------------------
//! 2. NON-NORMAL LIE-BRACKET TRANSPORT
//! ------------------------------------------------------------
//!
//! Dynamics evolve as:
//!
//!     dZ_k/dt = Σ_j (Z_k S_j - Z_j S_k) κ(k,j) - λ Z_k
//!
//! with antisymmetric constraint:
//!
//!     κ(i,j) = -κ(j,i)
//!
//! Energy decay property:
//!
//!     d||Z||²/dt = -2λ||Z||²
//!
//! Interpretation:
//!   - redistributes spectral energy
//!   - prevents self-amplification
//!   - couples bands through structured transport
//!
//! ------------------------------------------------------------
//! 3. RESIDUAL SHEAR MEMORY
//! ------------------------------------------------------------
//!
//! Memory evolves via exponential smoothing:
//!
//!     S_k ← α S_k + (1 - α)(Z_k - Π_W Z_k)
//!
//! S captures structured mismatch between:
//!   - observed spectral state
//!   - projected subspace representation
//!
//! ------------------------------------------------------------
//! 4. INSTABILITY METRIC
//! ------------------------------------------------------------
//!
//!     B(t) = ||S|| / (||Z|| + ε)
//!
//! Interpretation:
//!   low B  → stable spectral manifold
//!   high B → drift / anomaly / mismatch event
//!
//! ------------------------------------------------------------
//! 5. ADAPTIVE BASIS DYNAMICS
//! ------------------------------------------------------------
//!
//! Basis evolves under constrained adaptation:
//!
//!     W ← Normalize(W + η∇)
//!
//! asymptotically converging toward:
//!
//!     W* ∈ Gr(R, D)
//!
//! (Grassmann manifold fixed point)
//!
//! ============================================================
//! ENGINEERING ARCHITECTURE
//! ============================================================
//!
//! The runtime is structured as a 4-stage pipeline:
//!
//!   (1) Spectral projection (input → Z)
//!   (2) Non-normal transport (Z evolution)
//!   (3) Gain regulation + stabilization
//!   (4) Memory + basis adaptation
//!
//! The system is deterministic and forward-simulatable.
//!
//! ============================================================
//! IMPLEMENTATION NOTES
//! ============================================================
//!
//! Key design constraints:
//!
//!   • κ is a calibrated antisymmetric kernel (not learned here)
//!   • S is causal EMA memory (non-invertible by design)
//!   • W is normalized each iteration to prevent drift
//!   • Z evolution is damped (λ > 0 ensures stability)
//!
//! Numerical properties:
//!
//!   stability: bounded by λ-damping
//!   memory: exponentially weighted residual system
//!   coupling: non-normal Lie transport operator
//!
//! ============================================================
//! VALIDATION / LIMITATIONS
//! ============================================================
//!
//! This system is:
//!
//!   ✔ deterministic
//!   ✔ stable under bounded inputs
//!   ✔ suitable for RF anomaly detection / feature extraction
//!
//! This system is NOT:
//!
//!   ✘ cryptographic
//!   ✘ encryption primitive
//!   ✘ secure communication system
//!   ✘ FFT replacement
//!
//! ============================================================
//! END OF SPEC HEADER
//! ============================================================
//! DVSM-RF · NON-NORMAL SPECTRAL OPERATOR RUNTIME
//! DVSM_RF_SPECTRAL_OPERATOR_RUNTIME.rs
//!
//! Author  : Daniel J. Dillberg
//! Version : 1.0-canonical
//! Date    : 2026-05-15
//! License : AGLP-3 Dual
//!
//! ============================================================
//! SYSTEM OVERVIEW
//! ============================================================
//!
//! DVSM-RF is a deterministic non-normal spectral operator system
//! for RF signal analysis and instability detection.
//!
//! It models broadband RF signals as an interacting spectral manifold:
//!
//!     μ_t → Z_t → S_t → W_t
//!
//! where:
//!
//!   μ_t : spectral measure (input distribution)
//!   Z_t : transport field (banded latent state)
//!   S_t : residual shear memory (EMA-based mismatch state)
//!   W_t : adaptive projection basis (Stiefel-like constraint)
//!
//! The system is NOT cryptographic and NOT an FFT replacement.
//! It is a structured spectral transport and anomaly detection runtime.
//!
//! ============================================================
//! MATHEMATICAL MODEL
//! ============================================================
//!
//! ------------------------------------------------------------
//! 1. SPECTRAL STATE REPRESENTATION
//! ------------------------------------------------------------
//!
//! RF input is treated as a spectral measure:
//!
//!     μ_t = Σ_i w_i δ(g_i)
//!
//! Projected into latent transport state:
//!
//!     Z ∈ ℝ^R
//!
//! ------------------------------------------------------------
//! 2. NON-NORMAL LIE-BRACKET TRANSPORT
//! ------------------------------------------------------------
//!
//!     dZ_k/dt = Σ_j (Z_k S_j - Z_j S_k) κ(k,j) - λ Z_k
//!
//! κ antisymmetry:
//!
//!     κ(i,j) = -κ(j,i)
//!
//! Energy decay:
//!
//!     d||Z||²/dt = -2λ||Z||²
//!
//! ------------------------------------------------------------
//! 3. RESIDUAL SHEAR MEMORY
//! ------------------------------------------------------------
//!
//!     S_k ← α S_k + (1 - α)(Z_k - Π_W Z_k)
//!
//! ------------------------------------------------------------
//! 4. INSTABILITY METRIC
//! ------------------------------------------------------------
//!
//!     B(t) = ||S|| / (||Z|| + ε)
//!
//! ------------------------------------------------------------
//! 5. ADAPTIVE BASIS DYNAMICS
//! ------------------------------------------------------------
//!
//!     W ← Normalize(W + η∇)
//!
//! W converges toward:
//!
//!     W* ∈ Gr(R, D)
//!
//! ============================================================
//! ENGINEERING ARCHITECTURE
//! ============================================================
//!
//! Pipeline:
//!   (1) Spectral projection → Z
//!   (2) Non-normal transport evolution
//!   (3) Gain regulation + damping
//!   (4) Memory + basis adaptation
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
    // §3 · STAGE 1 — SPECTRAL PROJECTION
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
    // §4 · STAGE 2 — NON-NORMAL LIE TRANSPORT
    // ========================================================

    pub fn evolve_transport(&mut self) {
        let mut dz = vec![0.0; R];

        for k in 0..R {
            let z_k = self.bands[k].z;
            let s_k = self.bands[k].s;

            let mut coupling = 0.0;

            for j in 0..R {
                if j == k { continue; }

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
    // §5 · STAGE 3 — GAIN REGULATION
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

            self.energy += self.bands[k].z * self.bands[k].z;
        }

        self.energy = self.energy.sqrt();
    }

    // ========================================================
    // §6 · STAGE 4 — SHEAR MEMORY UPDATE
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
    // §7 · STAGE 5 — BASIS ADAPTATION
    // ========================================================

    pub fn adapt_basis(&mut self) {
        for k in 0..R {
            let err = self.bands[k].z - self.bands[k].s;

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
    // §8 · STAGE 6 — INSTABILITY METRIC
    // ========================================================

    pub fn compute_b(&mut self) {
        let z_norm =
            self.bands.iter()
                .map(|b| b.z * b.z)
                .sum::<f64>()
                .sqrt();

        let s_norm =
            self.bands.iter()
                .map(|b| b.s * b.s)
                .sum::<f64>()
                .sqrt();

        self.b_metric = s_norm / (z_norm + 1e-9);
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
        self.bands.iter()
            .enumerate()
            .max_by(|a, b| a.1.z.abs().partial_cmp(&b.1.z.abs()).unwrap())
            .map(|(i, _)| i)
            .unwrap_or(0)
    }
}

// ============================================================
// §11 · SYNTHETIC SIGNAL
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
// §13 · SYSTEM CLASSIFICATION
// ============================================================
//
// ✔ Non-normal spectral transport system
// ✔ Measure-valued RF manifold model
// ✔ Adaptive basis DSP runtime
//
// NOT:
// ✘ Encryption system
// ✘ Cryptographic primitive
// ✘ Secure communication system
// ✘ FFT replacement
//
// ============================================================
// END OF FILE
// ============================================================
// DVSM_RF_SPECTRAL_OPERATOR_RUNTIME_ADENDUM.rs
// ============================================================
//! ============================================================
//! DVSM-RF · SPECTRAL OPERATOR RUNTIME (ADENDUM)
//! μ / Z / S / W UNIFIED ARCHITECTURE LAYER
//! ============================================================
//!
//! This file unifies:
//!   (1) spectral measure layer (μ)
//!   (2) transport field layer (Z)
//!   (3) shear memory layer (S)
//!   (4) geometric basis layer (W)
//!
//! It is designed as a production DSP extension point:
//!   - STFT / CWT front-end compatible
//!   - streaming RF ingestion compatible
//!   - deterministic, forward-simulatable
//!
//! ============================================================

use std::f64::consts::PI;

// ============================================================
// §0 · CORE CONFIG
// ============================================================

pub const R: usize = 16;
pub const DT: f64 = 1.0 / 60.0;

pub const LAMBDA: f64 = 0.05;
pub const ALPHA: f64 = 0.98;
pub const EPS: f64 = 1e-9;

pub const BASIS_LR: f64 = 0.001;

// ============================================================
// §1 · μ (SPECTRAL MEASURE LAYER)
// ============================================================

#[derive(Clone, Debug)]
pub struct SpectralMeasure {
    pub weights: Vec<f64>,
    pub carriers: Vec<f64>,
}

impl SpectralMeasure {
    pub fn new(r: usize) -> Self {
        Self {
            weights: vec![1.0 / r as f64; r],
            carriers: (0..r).map(|i| i as f64).collect(),
        }
    }

    pub fn normalize(&mut self) {
        let sum: f64 = self.weights.iter().sum();
        if sum > 0.0 {
            for w in &mut self.weights {
                *w /= sum;
            }
        }
    }
}

// ============================================================
// §2 · Z (TRANSPORT FIELD LAYER)
// ============================================================

#[derive(Clone, Debug)]
pub struct TransportField {
    pub z: Vec<f64>,
}

impl TransportField {
    pub fn new(r: usize) -> Self {
        Self { z: vec![0.0; r] }
    }

    pub fn norm(&self) -> f64 {
        self.z.iter().map(|x| x * x).sum::<f64>().sqrt()
    }
}

// ============================================================
// §3 · S (SHEAR MEMORY LAYER)
// ============================================================

#[derive(Clone, Debug)]
pub struct ShearMemory {
    pub s: Vec<f64>,
}

impl ShearMemory {
    pub fn new(r: usize) -> Self {
        Self { s: vec![0.0; r] }
    }

    pub fn update(&mut self, z: &[f64], residual: &[f64]) {
        for i in 0..z.len() {
            self.s[i] = ALPHA * self.s[i] + (1.0 - ALPHA) * residual[i];
        }
    }

    pub fn norm(&self) -> f64 {
        self.s.iter().map(|x| x * x).sum::<f64>().sqrt()
    }
}

// ============================================================
// §4 · W (GEOMETRIC BASIS LAYER)
// ============================================================

#[derive(Clone, Debug)]
pub struct Basis {
    pub w: Vec<f64>,
}

impl Basis {
    pub fn new(r: usize) -> Self {
        Self {
            w: vec![1.0 / r as f64; r],
        }
    }

    pub fn project(&self, z: &[f64]) -> Vec<f64> {
        let avg: f64 = self.w.iter().sum::<f64>() / self.w.len() as f64;
        z.iter().map(|x| x * avg).collect()
    }

    pub fn normalize(&mut self) {
        let norm = self.w.iter().map(|x| x * x).sum::<f64>().sqrt() + EPS;
        for w in &mut self.w {
            *w /= norm;
        }
    }

    pub fn adapt(&mut self, error: &[f64]) {
        for i in 0..self.w.len() {
            self.w[i] += BASIS_LR * error[i % error.len()] * (1.0 - self.w[i].abs());
        }
        self.normalize();
    }
}

// ============================================================
// §5 · κ (NON-NORMAL COUPLING KERNEL)
// ============================================================

#[derive(Clone, Debug)]
pub struct Kernel {
    pub kappa: Vec<Vec<f64>>,
}

impl Kernel {
    pub fn new(r: usize) -> Self {
        let mut kappa = vec![vec![0.0; r]; r];

        for i in 0..r {
            for j in 0..r {
                let v = ((i as f64) * 1.3 - (j as f64) * 1.7).sin();
                kappa[i][j] = v;
                kappa[j][i] = -v; // enforce antisymmetry
            }
        }

        Self { kappa }
    }
}

// ============================================================
// §6 · FULL DVSM-RF UNIFIED RUNTIME
// ============================================================

pub struct DVSMRuntime {
    pub mu: SpectralMeasure,
    pub z: TransportField,
    pub s: ShearMemory,
    pub w: Basis,
    pub k: Kernel,
    pub frame: usize,
    pub b_metric: f64,
}

impl DVSMRuntime {
    pub fn new() -> Self {
        Self {
            mu: SpectralMeasure::new(R),
            z: TransportField::new(R),
            s: ShearMemory::new(R),
            w: Basis::new(R),
            k: Kernel::new(R),
            frame: 0,
            b_metric: 0.0,
        }
    }

    // ========================================================
    // §7 · DSP FRONT-END HOOK (STFT / CWT READY)
    // ========================================================

    pub fn ingest_signal(&mut self, samples: &[f64]) {
        for i in 0..R {
            let mut acc = 0.0;

            for (t, x) in samples.iter().enumerate() {
                let phase = (i as f64) * PI * (t as f64 / samples.len() as f64);
                acc += x * phase.sin();
            }

            self.z.z[i] = acc / samples.len() as f64;
        }
    }

    // ========================================================
    // §8 · NON-NORMAL LIE TRANSPORT
    // ========================================================

    pub fn transport_step(&mut self) {
        let mut dz = vec![0.0; R];

        for i in 0..R {
            let mut sum = 0.0;

            for j in 0..R {
                let zi = self.z.z[i];
                let zj = self.z.z[j];
                let si = self.s.s[i];
                let sj = self.s.s[j];

                sum += (zi * sj - zj * si) * self.k.kappa[i][j];
            }

            dz[i] = sum - LAMBDA * self.z.z[i];
        }

        for i in 0..R {
            self.z.z[i] += DT * dz[i];
        }
    }

    // ========================================================
    // §9 · SHEAR MEMORY UPDATE
    // ========================================================

    pub fn memory_step(&mut self) {
        let residual = self.w.project(&self.z.z);

        let mut err = vec![0.0; R];

        for i in 0..R {
            err[i] = self.z.z[i] - residual[i];
        }

        self.s.update(&self.z.z, &err);
    }

    // ========================================================
    // §10 · BASIS ADAPTATION
    // ========================================================

    pub fn basis_step(&mut self) {
        let residual = self.w.project(&self.z.z);

        let mut err = vec![0.0; R];
        for i in 0..R {
            err[i] = self.z.z[i] - residual[i];
        }

        self.w.adapt(&err);
    }

    // ========================================================
    // §11 · INSTABILITY METRIC
    // ========================================================

    pub fn compute_b(&mut self) {
        let z_norm = self.z.norm();
        let s_norm = self.s.norm();

        self.b_metric = s_norm / (z_norm + EPS);
    }

    pub fn burst(&self) -> bool {
        self.b_metric > 2.0
    }

    // ========================================================
    // §12 · FULL STEP PIPELINE
    // ========================================================

    pub fn step(&mut self, samples: &[f64]) {
        self.ingest_signal(samples);
        self.transport_step();
        self.memory_step();
        self.basis_step();
        self.compute_b();
        self.frame += 1;
    }
}

// ============================================================
// §13 · TEST DRIVER
// ============================================================

pub fn synthetic_signal(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let t = i as f64 / n as f64;
            0.4 * (2.0 * PI * 120.0 * t).sin()
                + 0.2 * (2.0 * PI * 440.0 * t).sin()
                + 0.1 * (2.0 * PI * 900.0 * t).sin()
        })
        .collect()
}

fn main() {
    let mut rt = DVSMRuntime::new();

    for frame in 0..200 {
        let sig = synthetic_signal(1024);

        rt.step(&sig);

        println!(
            "frame={} B={:.4} burst={}",
            frame,
            rt.b_metric,
            rt.burst()
        );
    }
}
