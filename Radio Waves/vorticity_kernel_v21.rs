//! ============================================================
//! DVSM-RF · UNIFIED SPECTRAL TRANSPORT ENGINE
//! Single-file production-refactored architecture
//! ============================================================
//! ============================================================================
//! DVSM-RF · ENGINEERING INTRO BLOCK (FOR DEVELOPERS)
//! ============================================================================
//!
//! This system is a layered non-normal spectral transport runtime.
//! It is designed for engineers working on RF, DSP, anomaly detection,
//! and structured dynamical systems.
//!
//! ---------------------------------------------------------------------------
//! WHAT THIS SYSTEM IS
//! ---------------------------------------------------------------------------
//!
//! DVSM-RF treats a signal not as a static waveform, but as a *living
//! dynamical field* evolving across three coupled spaces:
//!
//!     μ(t) → Z(t) → S(t) → W(t)
//!
//! where:
//!
//!   μ(t) : input spectral measure (raw signal → distribution)
//!   Z(t) : latent spectral transport field (state dynamics)
//!   S(t) : residual shear memory (history mismatch / hysteresis)
//!   W(t) : adaptive basis geometry (learned projection manifold)
//!
//! ---------------------------------------------------------------------------
//! WHAT “GHOSTS” MEANS IN THIS SYSTEM
//! ---------------------------------------------------------------------------
//!
//! “Ghosts” are not metaphysical objects.
//!
//! They are mathematically defined residual structures:
//!
//!     G(t) = Z(t) − Π_W[Z(t)]
//!
//! Interpretation:
//!
//!   • Z(t) carries observed spectral energy
//!   • W(t) defines what the system believes is “explainable”
//!   • G(t) is what leaks through the model
//!
//! These residuals behave like persistent structure in time:
//!
//!     S(t+1) = α S(t) + (1 − α) G(t)
//!
//! So “ghosts” = *coherent prediction error stored in time*.
//!
//! They are not noise.
//! They are *unmodeled transport geometry*.
//!
//! ---------------------------------------------------------------------------
//! CORE ARITHMETIC (ENGINEERING FORM)
//! ---------------------------------------------------------------------------
//!
//! 1. NON-NORMAL TRANSPORT (core dynamics)
//!
//!     dZ_k/dt = Σ_j (Z_k S_j − Z_j S_k) κ(k,j) − λ Z_k
//!
//! Meaning:
//!   energy is redistributed, not created
//!   instability is structural, not random
//!
//! Energy bound:
//!
//!     d||Z||²/dt = −2λ||Z||²
//!
//! ---------------------------------------------------------------------------
//!
//! 2. RESIDUAL MEMORY (ghost accumulation)
//!
//!     S_k ← α S_k + (1 − α)(Z_k − W_k Z_k)
//!
//! This forms a hysteretic memory of mismatch.
//!
//! ---------------------------------------------------------------------------
//!
//! 3. INSTABILITY METRIC (system strain)
//!
//!     B(t) = ||S|| / (||Z|| + ε)
//!
//! Interpretation:
//!   B small  → model explains signal
//!   B large  → structure is drifting / unseen dynamics exist
//!
//! ---------------------------------------------------------------------------
//!
//! 4. GAIN REGULATION (stability clamp)
//!
//!     gain_k = min(1, T / |Z_k|)
//!
//! prevents local explosion while preserving global transport.
//!
//! ---------------------------------------------------------------------------
//!
//! 5. BASIS ADAPTATION (geometry update)
//!
//!     W ← Normalize(W + η (Z − S))
//!
//! W learns the explainable manifold.
//!
//! ---------------------------------------------------------------------------
//! ENGINEERING INTUITION
//! ---------------------------------------------------------------------------
//!
//! Think of the system as three coupled forces:
//!
//!   SIGNAL (Z)   → what exists now
//!   MEMORY (S)   → what used to be unexplained
//!   MODEL (W)    → what the system believes is normal
//!
//! The “ghost field” is:
//!
//!     G = mismatch between reality and learned geometry
//!
//! And DVSM-RF is a controlled way to:
//!
//!     1. measure it
//!     2. store it
//!     3. propagate it
//!     4. adapt to it
//!
//! ---------------------------------------------------------------------------
//! WHY THIS IS USEFUL IN PRACTICE
//! ---------------------------------------------------------------------------
//!
//! This architecture is useful when:
//!
//!   • FFT features are stable but behavior is not
//!   • anomalies are structural, not energetic
//!   • drift is gradual, not spiky
//!   • signal identity is in relationships, not amplitudes
//!
//! It detects:
//!
//!   • modulation drift
//!   • emitter deformation
//!   • interference structure
//!   • hidden coupling changes
//!   • non-stationary spectral topology
//!
//! ---------------------------------------------------------------------------
//! KEY DESIGN RULE FOR ENGINEERS
//! ---------------------------------------------------------------------------
//!
//! If you remove S(t), the system becomes a filter.
//! If you remove W(t), it becomes a detector.
//! If you remove Z(t), it becomes memory only.
//!
//! The system only works when all three coexist.
//!
//! ---------------------------------------------------------------------------
//! END OF ENGINEERING INTRO
//! ============================================================================
//! //! DVSM-RF · Vorticity (Non-Normal Spectral Definition)
//!
//! In classical DSP/CFD:
//!     vorticity ≈ spatial curl or phase gradient (local derivative)
//!
//! In DVSM-RF:
//!     vorticity is NOT geometric curl.
//!     It is antisymmetric spectral energy circulation across modes.
//!
//! ------------------------------------------------------------------
//! CORE IDEA
//! ------------------------------------------------------------------
//!
//! ω_k = Σ_j (Z_k * S_j − Z_j * S_k) κ(k,j)
//!
//! This measures:
//!   → directional energy imbalance
//!   → cross-mode circulation strength
//!   → non-normal rotational amplification
//!
//! Unlike classical methods:
//!   • no spatial grid required
//!   • no derivative operator
//!   • rotation emerges from coupling topology κ
//!
//! ------------------------------------------------------------------

pub fn dvsm_vorticity_k(
    k: usize,
    z: &[f64],
    s: &[f64],
    kappa: &[Vec<f64>],
) -> f64 {
    let mut vort = 0.0;

    for j in 0..z.len() {
        if j == k {
            continue;
        }

        // antisymmetric transport contribution
        let transport =
            (z[k] * s[j]) - (z[j] * s[k]);

        vort += transport * kappa[k][j];
    }

    vort
}
//! =============================================================================

use std::f64::consts::PI;

// ============================================================
// §0 CONFIGURATION (SYSTEM PARAMETERS)
// ============================================================

pub const R: usize = 16;
pub const DT: f64 = 1.0 / 60.0;

pub const LAMBDA: f64 = 0.05;          // dissipation
pub const ALPHA: f64 = 0.97;           // EMA memory
pub const BETA: f64 = 0.25;            // elasticity coupling
pub const ETA: f64 = 0.001;            // basis adaptation rate

pub const B_CRIT: f64 = 2.0;
pub const EPS: f64 = 1e-9;

// ============================================================
// §1 CORE TYPES
// ============================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub fn new(re: f64, im: f64) -> Self { Self { re, im } }

    pub fn abs2(&self) -> f64 { self.re*self.re + self.im*self.im }
    pub fn abs(&self) -> f64 { self.abs2().sqrt() }

    pub fn conj(&self) -> Self {
        Self { re: self.re, im: -self.im }
    }
}

// arithmetic
use std::ops::{Add, Sub, Mul};

impl Add for Complex {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.re + o.re, self.im + o.im)
    }
}

impl Sub for Complex {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.re - o.re, self.im - o.im)
    }
}

impl Mul for Complex {
    type Output = Self;
    fn mul(self, o: Self) -> Self {
        Self::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
}

// ============================================================
// §2 STATE LAYERS (μ → Z → S → W)
// ============================================================

#[derive(Clone, Debug, Default)]
pub struct Band {
    pub z: Complex,   // spectral field Z
    pub s: Complex,   // shear memory S
}

#[derive(Clone, Debug)]
pub struct Basis {
    pub w: [f64; 4],
}

impl Default for Basis {
    fn default() -> Self {
        Self { w: [1.0, 0.5, 0.25, 0.125] }
    }
}

// ============================================================
// §3 Xi FEATURE VECTOR (INSTABILITY GEOMETRY)
// ============================================================

#[derive(Clone, Debug, Default)]
pub struct Xi {
    pub b: f64,       // instability magnitude
    pub db: f64,      // velocity
    pub d2b: f64,     // acceleration
}

// ============================================================
// §4 SCHMITT HYSTERESIS
// ============================================================

#[derive(Clone, Debug)]
pub struct Schmitt {
    pub low: f64,
    pub high: f64,
    pub state: bool,
}

impl Schmitt {
    pub fn new(low: f64, high: f64) -> Self {
        Self { low, high, state: false }
    }

    pub fn update(&mut self, x: f64) -> bool {
        if self.state && x < self.low {
            self.state = false;
        } else if !self.state && x > self.high {
            self.state = true;
        }
        self.state
    }
}

// ============================================================
// §5 CORE RUNTIME
// ============================================================

pub struct DVSM {
    pub bands: Vec<Band>,
    pub basis: Vec<Basis>,
    pub kappa: Vec<Vec<f64>>,

    pub prev_b: f64,
    pub schmitt: Schmitt,
    pub frame: usize,
}

impl DVSM {

    // --------------------------------------------------------
    // INIT
    // --------------------------------------------------------
    pub fn new() -> Self {
        let mut kappa = vec![vec![0.0; R]; R];

        for i in 0..R {
            for j in 0..R {
                kappa[i][j] =
                    ((i as f64 * 1.37) - (j as f64 * 1.73)).sin();
            }
        }

        Self {
            bands: vec![Band::default(); R],
            basis: vec![Basis::default(); R],
            kappa,
            prev_b: 0.0,
            schmitt: Schmitt::new(0.55, 0.75),
            frame: 0,
        }
    }

    // --------------------------------------------------------
    // §1 μ → Z (SPECTRAL PROJECTION)
    // --------------------------------------------------------
    pub fn project(&mut self, samples: &[f64]) {
        let n = samples.len().max(1) as f64;

        for k in 0..R {
            let mut re = 0.0;
            let mut im = 0.0;

            let freq = (k as f64 + 1.0) * 120.0;

            for (i, &x) in samples.iter().enumerate() {
                let t = i as f64 / n;
                let p = 2.0 * PI * freq * t;

                re += x * p.cos();
                im += x * p.sin();
            }

            self.bands[k].z = Complex::new(re / n, im / n);
        }
    }

    // --------------------------------------------------------
    // §2 NON-NORMAL LIE TRANSPORT (Z evolution)
    // --------------------------------------------------------
    pub fn transport(&mut self) {
        let mut dz = vec![Complex::default(); R];

        for k in 0..R {
            let zk = self.bands[k].z;
            let sk = self.bands[k].s;

            let mut acc = Complex::default();

            for j in 0..R {
                if j == k { continue; }

                let zj = self.bands[j].z;
                let sj = self.bands[j].s;

                let term =
                    (zk * sj.conj()) - (zj * sk.conj());

                let scale = self.kappa[k][j];

                acc = acc + Complex::new(
                    term.re * scale,
                    term.im * scale,
                );
            }

            dz[k] = Complex::new(
                acc.re - LAMBDA * zk.re,
                acc.im - LAMBDA * zk.im,
            );
        }

        for k in 0..R {
            self.bands[k].z.re += DT * dz[k].re;
            self.bands[k].z.im += DT * dz[k].im;
        }
    }

    // --------------------------------------------------------
    // §3 EMA SHEAR MEMORY (S update)
    // --------------------------------------------------------
    pub fn memory(&mut self) {
        for k in 0..R {
            let z = self.bands[k].z;
            let s = &mut self.bands[k].s;

            s.re = ALPHA * s.re + (1.0 - ALPHA) * z.re;
            s.im = ALPHA * s.im + (1.0 - ALPHA) * z.im;
        }
    }

    // --------------------------------------------------------
    // §4 BASIS ADAPTATION (W update)
    // --------------------------------------------------------
    pub fn adapt_basis(&mut self) {
        for k in 0..R {
            let z = self.bands[k].z.abs();
            let s = self.bands[k].s.abs();

            let err = z - s;

            for j in 0..4 {
                self.basis[k].w[j] += ETA * err;
            }

            let norm = self.basis[k]
                .w.iter()
                .map(|x| x*x).sum::<f64>()
                .sqrt() + EPS;

            for j in 0..4 {
                self.basis[k].w[j] /= norm;
            }
        }
    }

    // --------------------------------------------------------
    // §5 Xi EXTRACTION (instability geometry)
    // --------------------------------------------------------
    pub fn xi(&self) -> Xi {
        let mut z2 = 0.0;
        let mut s2 = 0.0;

        for b in &self.bands {
            z2 += b.z.abs2();
            s2 += b.s.abs2();
        }

        let b = s2 / (z2 + EPS);
        let db = b - self.prev_b;

        Xi { b, db, d2b: db }
    }

    // --------------------------------------------------------
    // §6 RISK SCORE (UNIFIED FORM)
    // --------------------------------------------------------
    pub fn score(&self, xi: &Xi) -> f64 {
        (xi.b * (1.0 + xi.db.abs()) * (1.0 + xi.d2b.abs()))
            .tanh()
    }

    // --------------------------------------------------------
    // §7 STEP PIPELINE
    // --------------------------------------------------------
    pub fn step(&mut self, samples: &[f64]) -> (f64, &'static str) {

        self.project(samples);
        self.transport();
        self.memory();
        self.adapt_basis();

        let xi = self.xi();
        let score = self.score(&xi);

        let triggered = self.schmitt.update(score);

        let band = match triggered {
            false => "STABLE",
            true if score < 0.3 => "LOW",
            true if score < 0.7 => "MEDIUM",
            _ => "HIGH",
        };

        self.prev_b = xi.b;
        self.frame += 1;

        (score, band)
    }
}

// ============================================================
// §6 TEST SIGNAL
// ============================================================

pub fn signal(n: usize) -> Vec<f64> {
    let mut v = vec![0.0; n];

    for i in 0..n {
        let t = i as f64 / n as f64;

        v[i] =
            0.4 * (2.0 * PI * 120.0 * t).sin()
            + 0.2 * (2.0 * PI * 440.0 * t).sin();
    }

    v
}

// ============================================================
// §7 MAIN
// ============================================================

fn main() {
    let mut dvsm = DVSM::new();

    for _ in 0..200 {
        let s = signal(1024);
        let (score, band) = dvsm.step(&s);

        println!("score={:.4} band={}", score, band);
    }
}
