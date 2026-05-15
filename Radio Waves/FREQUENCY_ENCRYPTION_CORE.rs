// AUTHOR: Daniel J. dillberg
// DATE  : 2026-05-15
// ============================================================
// DVSM-DFE · CURVATURE-STABILIZED SPECTRAL CORE
// SINGLE FILE · A/B/C ARCHITECTURE
// ============================================================

use nalgebra::{DMatrix, DVector};

// ============================================================
// §0 MATHEMATICAL SPEC (REFERENCE CONTRACT)
// ============================================================
//
// Z ∈ R^n         signal
// S ∈ R^n         memory (EMA, irreversible)
// W ∈ St(n,r)     orthonormal basis
//
// [Z,S]_κ[i] = Σ_j (Z_i S_j - Z_j S_i) κ_ij
// κᵀ = -κ
//
// Ż = [Z,S]_κ - λZ
// S_{t+1} = αS + (1-α)Z
//
// W update: r ⊗ c + QR retraction
//
// ============================================================

// ============================================================
// §1 TRANSPORT CORE (LIE BRACKET DYNAMICS)
// ============================================================

pub struct TransportCore {
    pub kappa: DMatrix<f64>,
    pub lambda: f64,
}

impl TransportCore {

    pub fn lie_bracket(&self, z: &DVector<f64>, s: &DVector<f64>) -> DVector<f64> {
        let n = z.len();
        let mut out = DVector::zeros(n);

        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..n {
                acc += (z[i] * s[j] - z[j] * s[i]) * self.kappa[(i, j)];
            }
            out[i] = acc;
        }
        out
    }

    pub fn step_z(&self, z: &DVector<f64>, s: &DVector<f64>) -> DVector<f64> {
        self.lie_bracket(z, s) - self.lambda * z
    }
}

// ============================================================
// §2 MEMORY CORE (IRREVERSIBLE EMA)
// ============================================================

pub struct MemoryCore {
    pub alpha: f64,
    pub s: DVector<f64>,
}

impl MemoryCore {

    pub fn new(n: usize, alpha: f64) -> Self {
        Self {
            alpha,
            s: DVector::zeros(n),
        }
    }

    pub fn update(&mut self, z: &DVector<f64>) {
        self.s = (&self.s * self.alpha) + (z * (1.0 - self.alpha));
    }
}

// ============================================================
// §3 BASIS CORE (STIEFEL MANIFOLD W ∈ St(n,r))
// ============================================================

pub struct BasisCore {
    pub w: DMatrix<f64>,
}

impl BasisCore {

    pub fn project(&self, z: &DVector<f64>) -> DVector<f64> {
        &self.w * (&self.w.transpose() * z)
    }

    pub fn residual(&self, z: &DVector<f64>) -> DVector<f64> {
        z - self.project(z)
    }

    pub fn update(&mut self, r: &DVector<f64>, c: &DVector<f64>, eta: f64) {
        let n = r.len();
        let k = c.len();

        for i in 0..n {
            for j in 0..k {
                self.w[(i, j)] += eta * r[i] * c[j];
            }
        }

        self.orthonormalize();
    }

    fn orthonormalize(&mut self) {
        let qr = self.w.clone().qr();
        self.w = qr.q();
    }
}

// ============================================================
// §4 LAYER A — ANALYSIS (NO MUTATION)
// ============================================================

pub struct LayerA;

pub struct AnalysisFrame {
    pub projection: DVector<f64>,
    pub residual: DVector<f64>,
    pub energy_residual: f64,
}

impl LayerA {

    pub fn run(basis: &BasisCore, z: &DVector<f64>) -> AnalysisFrame {
        let p = basis.project(z);
        let r = z - &p;

        AnalysisFrame {
            projection: p,
            residual: r.clone(),
            energy_residual: r.norm(),
        }
    }
}

// ============================================================
// §5 LAYER B — BEHAVIOR (SPECTRAL TRANSFORM ONLY)
// ============================================================

pub struct LayerB;

impl LayerB {

    pub fn encode(
        transport: &TransportCore,
        memory: &DVector<f64>,
        z: &DVector<f64>,
    ) -> DVector<f64> {

        transport.step_z(z, memory)
    }
}

// ============================================================
// §6 LAYER C — CONTROL (SLOW ADAPTATION ONLY)
// ============================================================

pub struct LayerC {
    pub ema_r: DVector<f64>,
    pub beta: f64,
}

impl LayerC {

    pub fn new(n: usize, beta: f64) -> Self {
        Self {
            ema_r: DVector::zeros(n),
            beta,
        }
    }

    pub fn update_ema(&mut self, r: &DVector<f64>) {
        self.ema_r = (&self.ema_r * self.beta) + (r * (1.0 - self.beta));
    }

    pub fn adapt_basis(
        &self,
        basis: &mut BasisCore,
        coeff: &DVector<f64>,
        eta: f64,
    ) {
        let n = self.ema_r.len();
        let r = coeff.len();

        for i in 0..n {
            for j in 0..r {
                basis.w[(i, j)] += eta * self.ema_r[i] * coeff[j];
            }
        }

        basis.orthonormalize();
    }
}

// ============================================================
// §7 RUNTIME ORCHESTRATION (STRICT A/B/C SEPARATION)
// ============================================================

pub struct DfeRuntime {
    pub transport: TransportCore,
    pub memory: MemoryCore,
    pub basis: BasisCore,
    pub control: LayerC,
}

impl DfeRuntime {

    pub fn step(&mut self, z: DVector<f64>) -> DVector<f64> {

        // =========================
        // A: ANALYSIS (observe only)
        // =========================
        let analysis = LayerA::run(&self.basis, &z);

        // =========================
        // B: BEHAVIOR (transform only)
        // =========================
        let z_enc = LayerB::encode(
            &self.transport,
            &self.memory.s,
            &z
        );

        // =========================
        // MEMORY (causal irreversible)
        // =========================
        self.memory.update(&z);

        // =========================
        // C: CONTROL (slow learning)
        // =========================
        self.control.update_ema(&analysis.residual);

        let coeff = &self.basis.w.transpose() * &z_enc;

        self.control.adapt_basis(
            &mut self.basis,
            &coeff,
            0.001
        );

        z_enc
    }
}
// ============================================================
// DVSM-DFE · CURVATURE-STABILIZED TRAJECTORY ENGINE
// ABC LAYERED ARCHITECTURE (OBSERVATION / TRANSFORM / ADAPT)
// ============================================================
//
// IMPORTANT SECURITY CLARIFICATION:
// This is NOT a cryptographic encryption system.
// It is a dynamical systems + spectral transformation engine.
//
// Crypto-adjacent use cases only (see bottom section).
// ============================================================

use nalgebra::{DMatrix, DVector};

// ============================================================
// §A OBSERVATION LAYER (STATE INGESTION / FEATURE VIEW)
// ============================================================

pub struct ObservationLayer {
    pub dimension: usize,
}

impl ObservationLayer {

    pub fn new(n: usize) -> Self {
        Self { dimension: n }
    }

    /// OBSERVATION MAP:
    /// Z(t) = raw signal state vector
    pub fn observe(&self, z_raw: &DVector<f64>) -> DVector<f64> {
        // Identity observation (can be extended to filtering / sensing noise models)
        z_raw.clone()
    }
}

// ============================================================
// §B TRANSFORMATION LAYER (LIE BRACKET + MEMORY FLOW)
// ============================================================

pub struct TransformLayer {
    pub kappa: DMatrix<f64>,
    pub lambda: f64,
}

impl TransformLayer {

    pub fn new(kappa: DMatrix<f64>, lambda: f64) -> Self {
        assert!(lambda > 0.0);
        Self { kappa, lambda }
    }

    /// Lie-bracket transport:
    /// [Z,S]_κ[i] = Σ_j (Z_i S_j - Z_j S_i) κ_ij
    pub fn lie_bracket(&self, z: &DVector<f64>, s: &DVector<f64>) -> DVector<f64> {
        let n = z.len();
        let mut out = DVector::from_element(n, 0.0);

        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..n {
                acc += (z[i] * s[j] - z[j] * s[i]) * self.kappa[(i, j)];
            }
            out[i] = acc;
        }

        out
    }

    /// Z dynamics:
    /// dZ = LieBracket(Z,S) - λZ
    pub fn transform(&self, z: &DVector<f64>, s: &DVector<f64>) -> DVector<f64> {
        self.lie_bracket(z, s) - (self.lambda * z)
    }
}

// ============================================================
// §C ADAPTATION LAYER (MEMORY + GEOMETRIC BASIS LEARNING)
// ============================================================

pub struct AdaptationLayer {
    pub alpha: f64,
    pub s: DVector<f64>,
    pub w: DMatrix<f64>,
}

impl AdaptationLayer {

    pub fn new(n: usize, r: usize, alpha: f64) -> Self {
        Self {
            alpha,
            s: DVector::from_element(n, 0.0),
            w: DMatrix::identity(n, r),
        }
    }

    /// EMA MEMORY (CAUSAL, NON-INVERTIBLE)
    pub fn update_memory(&mut self, z_t: &DVector<f64>) {
        self.s = (&self.s * self.alpha) + (z_t * (1.0 - self.alpha));
    }

    /// Projection onto learned subspace
    pub fn project(&self, x: &DVector<f64>) -> DVector<f64> {
        &self.w * (&self.w.transpose() * x)
    }

    pub fn residual(&self, x: &DVector<f64>) -> DVector<f64> {
        x - self.project(x)
    }

    /// Rank-1 / low-rank adaptive update
    pub fn adapt_basis(
        &mut self,
        residual: &DVector<f64>,
        coeff: &DVector<f64>,
        eta: f64
    ) {
        let n = residual.len();
        let r = coeff.len();

        for i in 0..n {
            for k in 0..r {
                self.w[(i,k)] += eta * residual[i] * coeff[k];
            }
        }

        self.orthonormalize();
    }

    fn orthonormalize(&mut self) {
        let qr = self.w.clone().qr();
        self.w = qr.q();
    }
}

// ============================================================
// §D FULL DVSM-DFE ENGINE (ABC COMPOSITION)
// ============================================================

pub struct DfeEngine {
    pub obs: ObservationLayer,
    pub trans: TransformLayer,
    pub adapt: AdaptationLayer,
}

impl DfeEngine {

    pub fn step(&mut self, z_raw: DVector<f64>) -> DVector<f64> {

        // A: OBSERVE
        let z = self.obs.observe(&z_raw);

        // B: TRANSFORM (uses memory state)
        let z_enc = self.trans.transform(&z, &self.adapt.s);

        // C1: MEMORY UPDATE (causal EMA)
        self.adapt.update_memory(&z);

        // C2: GEOMETRIC DECOMPOSITION
        let r = self.adapt.residual(&z_enc);
        let c = self.adapt.w.transpose() * &z_enc;

        // C3: BASIS ADAPTATION
        self.adapt.adapt_basis(&r, &c, 0.001);

        z_enc
    }
}

// ============================================================
// §E CRYPTO-ADJACENT APPLICATIONS (ENGINEERING REALITY)
// ============================================================
//
// IMPORTANT:
// This system is NOT secure encryption.
// It is a *dynamical transformation layer* useful for:
// signal shaping, obfuscation, and feature-space encoding.
//
// ------------------------------------------------------------
//
// 1. SPECTRAL OBFUSCATION (RF / SIGNAL SECURITY)
// ------------------------------------------------------------
// - Lie bracket destroys stable spectral bins
// - Adaptive basis prevents fixed FFT interpretation
// - Moving subspace = moving spectral frame
//
// USE IN:
//   • RF watermarking
//   • anti-jamming waveform shaping
//   • covert channel distortion layers
//
// LIMITATION:
//   not reversible without full state (W, S, κ history)
//
// ------------------------------------------------------------
//
// 2. FEATURE-LEVEL SECURITY (STRUCTURAL HIDING)
// ------------------------------------------------------------
// Instead of sending raw Z:
//
//   send:
//     - W (subspace basis)
//     - coeff = WᵀZ
//     - energy stats
//
// EFFECT:
//   observer sees structure, not waveform detail
//
// USE IN:
//   • telemetry compression
//   • anomaly-aware streaming
//   • edge inference privacy layers
//
// ------------------------------------------------------------
//
// 3. κ-KEYED BEHAVIOR (WEAK SECURITY PRIMITIVE)
// ------------------------------------------------------------
// If κ is treated as a secret parameter:
//
//   small κ mismatch → large divergence
//
// This creates:
//   sensitivity amplification
//   trajectory separation
//
// BUT:
//   NOT cryptographic hardness
//   just non-normal dynamics instability
//
// ------------------------------------------------------------
//
// 4. SPREAD-SPECTRUM ANALOGY
// ------------------------------------------------------------
// Behavior resembles:
//   • adaptive spread-spectrum encoding
//   • time-varying basis modulation
//   • whitening / scrambling channels
//
// ------------------------------------------------------------
//
// 5. DO NOT USE FOR:
// ------------------------------------------------------------
// ❌ password encryption
// ❌ key exchange
// ❌ digital signatures
// ❌ secure storage
//
// Reason:
//   deterministic system → fully simulatable
//
// ============================================================
// FINAL ENGINEERING INTERPRETATION
// ============================================================
//
// A: Observation  → signal ingestion layer
// B: Transform    → Lie-bracket transport dynamics
// C: Adaptation   → memory + geometric subspace learning
//
// CORE RESULT:
//   - controlled spectral drift
//   - non-normal transport dynamics
//   - adaptive coordinate system evolution
//
// ============================================================
// ============================================================
// DVSM-DFE · 3-IN-1 CURVATURE-STABILIZED ENGINE
// ============================================================
//
// MODES (single system, 3 operational behaviors):
//
//   MODE 1 → RF SPECTRAL ENGINE (waveform scrambling)
//   MODE 2 → FEATURE SECURITY ENGINE (structural hiding)
//   MODE 3 → κ-KEYED DYNAMICAL OBSCURATION LAYER
//
// Core math stays identical:
//   Z → Lie-bracket transport
//   S → causal EMA memory
//   W → Stiefel subspace adaptation
//
// ============================================================

use nalgebra::{DMatrix, DVector};

// ============================================================
// MODE SELECTOR
// ============================================================

#[derive(Clone, Copy)]
pub enum DfeMode {
    RF_SPECTRAL,
    FEATURE_SECURITY,
    KAPPA_KEYED,
}

// ============================================================
// §A OBSERVATION LAYER
// ============================================================

pub struct Observation {
    pub n: usize,
}

impl Observation {
    pub fn observe(&self, z: &DVector<f64>) -> DVector<f64> {
        z.clone()
    }
}

// ============================================================
// §B TRANSFORM LAYER (LIE BRACKET DYNAMICS)
// ============================================================

pub struct Transform {
    pub kappa: DMatrix<f64>,
    pub lambda: f64,
}

impl Transform {

    pub fn new(kappa: DMatrix<f64>, lambda: f64) -> Self {
        Self { kappa, lambda }
    }

    pub fn lie_bracket(&self, z: &DVector<f64>, s: &DVector<f64>) -> DVector<f64> {
        let n = z.len();
        let mut out = DVector::from_element(n, 0.0);

        for i in 0..n {
            let mut acc = 0.0;
            for j in 0..n {
                acc += (z[i] * s[j] - z[j] * s[i]) * self.kappa[(i, j)];
            }
            out[i] = acc;
        }

        out
    }

    pub fn step(&self, z: &DVector<f64>, s: &DVector<f64>) -> DVector<f64> {
        self.lie_bracket(z, s) - self.lambda * z
    }
}

// ============================================================
// §C ADAPTATION LAYER (EMA + STIEFEL BASIS)
// ============================================================

pub struct Adaptation {
    pub alpha: f64,
    pub s: DVector<f64>,
    pub w: DMatrix<f64>,
}

impl Adaptation {

    pub fn new(n: usize, r: usize, alpha: f64) -> Self {
        Self {
            alpha,
            s: DVector::from_element(n, 0.0),
            w: DMatrix::identity(n, r),
        }
    }

    pub fn update_memory(&mut self, z: &DVector<f64>) {
        self.s = &self.s * self.alpha + z * (1.0 - self.alpha);
    }

    pub fn project(&self, x: &DVector<f64>) -> DVector<f64> {
        &self.w * (&self.w.transpose() * x)
    }

    pub fn residual(&self, x: &DVector<f64>) -> DVector<f64> {
        x - self.project(x)
    }

    pub fn adapt_basis(&mut self, r: &DVector<f64>, c: &DVector<f64>, eta: f64) {
        let n = r.len();
        let k = c.len();

        for i in 0..n {
            for j in 0..k {
                self.w[(i, j)] += eta * r[i] * c[j];
            }
        }

        let qr = self.w.clone().qr();
        self.w = qr.q();
    }
}

// ============================================================
// §D MAIN ENGINE (3-IN-1 BEHAVIOR SWITCH)
// ============================================================

pub struct DfeEngine {
    pub obs: Observation,
    pub trans: Transform,
    pub adapt: Adaptation,
    pub mode: DfeMode,
}

impl DfeEngine {

    pub fn step(&mut self, z_raw: DVector<f64>) -> DVector<f64> {

        // A: OBSERVATION
        let z = self.obs.observe(&z_raw);

        // B: TRANSFORM (mode-dependent behavior)
        let z_enc = match self.mode {

            // -------------------------------------------------
            // MODE 1: RF SPECTRAL ENGINE
            // strong spectral drift, fast basis scrambling
            // -------------------------------------------------
            DfeMode::RF_SPECTRAL => {
                self.trans.step(&z, &self.adapt.s)
            }

            // -------------------------------------------------
            // MODE 2: FEATURE SECURITY ENGINE
            // suppress waveform identity, emphasize structure
            // -------------------------------------------------
            DfeMode::FEATURE_SECURITY => {
                let z_t = self.trans.step(&z, &self.adapt.s);
                self.adapt.project(&z_t)   // only structured projection survives
            }

            // -------------------------------------------------
            // MODE 3: κ-KEYED DYNAMICAL OBSCURATION
            // sensitive trajectory divergence layer
            // -------------------------------------------------
            DfeMode::KAPPA_KEYED => {
                let mut z_t = self.trans.step(&z, &self.adapt.s);

                // amplify non-normal sensitivity
                z_t *= 1.0 + self.trans.kappa.norm();

                z_t
            }
        };

        // C1: MEMORY UPDATE (always causal)
        self.adapt.update_memory(&z);

        // C2: ADAPT BASIS
        let r = self.adapt.residual(&z_enc);
        let c = self.adapt.w.transpose() * &z_enc;

        self.adapt.adapt_basis(&r, &c, 0.001);

        z_enc
    }
}

// ============================================================
// CRYPTO-ADJACENT ENGINEERING INTERPRETATION (3-IN-1 VIEW)
// ============================================================
//
// MODE 1 — RF SPECTRAL ENGINE
//   • moving frequency basis
//   • spectral whitening
//   • anti-jam waveform distortion
//
// MODE 2 — FEATURE SECURITY ENGINE
//   • transmits only subspace projections
//   • removes phase-level reconstructability
//   • used in telemetry / edge inference privacy
//
// MODE 3 — κ-KEYED OBSCURATION
//   • parameter-sensitive divergence system
//   • behaves like “dynamic key field”
//   • NOT cryptographic security
//
// ============================================================
//
// HARD LIMITATIONS:
//   ❌ not encryption
//   ❌ not key exchange
//   ❌ not signature system
//
// Because:
//   deterministic + forward simulatable dynamics
//
// ============================================================
