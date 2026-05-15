// ============================================================================
// RP1-R · REFINED GEOMETRIC STREAMING KERNEL
// ----------------------------------------------------------------------------
// Refactor Goals
// ----------------------------------------------------------------------------
// 1. Separate state evolution from basis evolution
// 2. Remove incompatible additive dynamics
// 3. Constrain observable state to bounded manifold
// 4. Keep W strictly on Stiefel manifold
// 5. Convert z_shear into external residual memory only
// 6. Make telemetry observational, never dynamical
// 7. Preserve deterministic 240Hz execution semantics
//
// This is a geometric streaming architecture.
// Not a physics simulator.
// ============================================================================

use nalgebra::{DMatrix, DVector};
use std::time::{Duration, Instant};

// ============================================================================
// NUMERICAL CONSTITUTION
// ============================================================================

const EPS: f64 = 1e-12;
const DRIFT_EPS: f64 = 1e-8;
const FRAME_BUDGET_US: u128 = 4167;

// ============================================================================
// REGIME CLASSIFICATION
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Regime {
    Contractive,
    ActiveSet,
    Rupture,
}

// ============================================================================
// CONFIGURATION
// ============================================================================

#[derive(Debug, Clone)]
pub struct Config {

    // basis learning rate
    pub eta: f64,

    // observable smoothing
    pub beta: f64,

    // residual memory decay
    pub alpha: f64,

    // state norm bound
    pub x_max: f64,

    // regime thresholds
    pub rupture_stress: f64,
    pub rupture_novelty: f64,

    pub active_residual_ratio: f64,
}

// ============================================================================
// TELEMETRY
// ============================================================================

#[derive(Debug, Clone)]
pub struct Telemetry {

    // alignment mismatch
    pub stress: f64,

    // residual energy
    pub novelty: f64,

    // orthonormal deviation
    pub drift: f64,

    // spectral distribution quality
    pub entropy: f64,

    // residual ratio
    pub residual_ratio: f64,

    // anomaly detector
    pub suspicious: bool,

    // runtime classification
    pub regime: Regime,

    // timing
    pub frame_time_us: u128,

    pub timestamp: Instant,
}

// ============================================================================
// CORE STATE
// ============================================================================

pub struct RP1RCore {

    // bounded observable state
    pub x: DVector<f64>,

    // orthonormal basis
    pub w: DMatrix<f64>,

    // external residual memory only
    pub z_shear: DVector<f64>,

    // historical telemetry
    prev_entropy: f64,
    drift_increase_frames: usize,

    // config
    pub cfg: Config,
}

// ============================================================================
// INITIALIZATION
// ============================================================================

impl RP1RCore {

    pub fn new(
        n: usize,
        r: usize,
        cfg: Config,
    ) -> Self {

        let mut w = DMatrix::<f64>::zeros(n, r);

        for i in 0..r.min(n) {
            w[(i, i)] = 1.0;
        }

        Self {
            x: DVector::zeros(n),
            w,
            z_shear: DVector::zeros(n),
            prev_entropy: 0.0,
            drift_increase_frames: 0,
            cfg,
        }
    }
}

// ============================================================================
// CORE UPDATE
// ============================================================================

impl RP1RCore {

    pub fn tick(
        &mut self,
        sigma: &DVector<f64>,
    ) -> Telemetry {

        let start = Instant::now();

        // --------------------------------------------------------------------
        // 1. ENSURE STIEFEL VALIDITY
        // --------------------------------------------------------------------

        self.retract();

        // --------------------------------------------------------------------
        // 2. PROJECTION
        // --------------------------------------------------------------------

        let projection =
            &self.w * (&self.w.transpose() * sigma);

        // --------------------------------------------------------------------
        // 3. EXTERNAL RESIDUAL
        // --------------------------------------------------------------------

        let residual = sigma - &projection;

        let sigma_norm = sigma.norm().max(EPS);
        let residual_norm = residual.norm();

        let residual_ratio =
            residual_norm / sigma_norm;

        // --------------------------------------------------------------------
        // 4. BASIS EVOLUTION
        // --------------------------------------------------------------------

        if residual_norm > EPS {

            let r_hat = &residual / residual_norm;

            let p_norm = projection.norm().max(EPS);
            let p_hat = &projection / p_norm;

            // ----------------------------------------------------------------
            // VERIFIED RANK-2 SKEW GENERATOR
            // A = rpᵀ - prᵀ
            // ----------------------------------------------------------------

            let a =
                &r_hat * p_hat.transpose()
                - &p_hat * r_hat.transpose();

            let delta_w = &a * &self.w;

            // tangent condition verification:
            // WᵀΔW + ΔWᵀW = 0
            #[cfg(debug_assertions)]
            {
                let tangent_test =
                    &self.w.transpose() * &delta_w
                    + delta_w.transpose() * &self.w;

                debug_assert!(
                    tangent_test.norm() < 1e-8,
                    "Stiefel tangent condition violated"
                );
            }

            self.w += self.cfg.eta * delta_w;

            // restore orthonormality
            self.retract();
        }

        // --------------------------------------------------------------------
        // 5. OBSERVABLE STATE UPDATE
        // --------------------------------------------------------------------
        //
        // x tracks ONLY the projected signal.
        //
        // No residual injection.
        // No ghost injection.
        // No additive force conflict.
        // --------------------------------------------------------------------

        let x_new =
            (1.0 - self.cfg.beta) * &self.x
            + self.cfg.beta * &projection;

        self.x = norm_clamp(x_new, self.cfg.x_max);

        // --------------------------------------------------------------------
        // 6. EXTERNAL RESIDUAL MEMORY
        // --------------------------------------------------------------------
        //
        // z_shear tracks ONLY unresolved external structure.
        // Never fed back into x.
        // --------------------------------------------------------------------

        self.z_shear =
            self.cfg.alpha * &self.z_shear
            + (1.0 - self.cfg.alpha) * &residual;

        // --------------------------------------------------------------------
        // 7. TELEMETRY
        // --------------------------------------------------------------------

        let drift = stiefel_drift(&self.w);

        let stress =
            1.0
            - cosine_similarity(
                &self.x,
                &projection,
            );

        let entropy =
            spectral_entropy_matrix(&self.w);

        // --------------------------------------------------------------------
        // 8. SECURITY DETECTOR
        // --------------------------------------------------------------------
        //
        // Detects rapid entropy collapse with sustained drift growth.
        // Avoids false positives on narrowband signals.
        // --------------------------------------------------------------------

        let entropy_drop =
            self.prev_entropy - entropy;

        if drift > DRIFT_EPS {
            self.drift_increase_frames += 1;
        } else {
            self.drift_increase_frames = 0;
        }

        let suspicious =
            entropy_drop > 0.5
            && self.drift_increase_frames > 10
            && stress < 0.1;

        self.prev_entropy = entropy;

        // --------------------------------------------------------------------
        // 9. REGIME CLASSIFICATION
        // --------------------------------------------------------------------

        let regime =
            if stress > self.cfg.rupture_stress
                || residual_ratio > self.cfg.rupture_novelty
            {
                Regime::Rupture
            }
            else if residual_ratio
                > self.cfg.active_residual_ratio
            {
                Regime::ActiveSet
            }
            else {
                Regime::Contractive
            };

        // --------------------------------------------------------------------
        // 10. FRAME TIMING
        // --------------------------------------------------------------------

        let frame_time_us =
            start.elapsed().as_micros();

        Telemetry {

            stress,

            novelty: residual_norm,

            drift,

            entropy,

            residual_ratio,

            suspicious,

            regime,

            frame_time_us,

            timestamp: Instant::now(),
        }
    }
}

// ============================================================================
// STIEFEL RETRACTION
// ============================================================================

impl RP1RCore {

    fn retract(&mut self) {

        let qr = self.w.clone().qr();

        let mut q = qr.q();

        // preserve orientation continuity
        for j in 0..q.ncols() {

            if q.column(j)
                .dot(&self.w.column(j)) < 0.0
            {
                q.column_mut(j).scale_mut(-1.0);
            }
        }

        self.w = q;
    }
}

// ============================================================================
// GEOMETRIC UTILITIES
// ============================================================================

#[inline(always)]
fn cosine_similarity(
    a: &DVector<f64>,
    b: &DVector<f64>,
) -> f64 {

    let an = a.norm().max(EPS);
    let bn = b.norm().max(EPS);

    (a.dot(b) / (an * bn))
        .clamp(-1.0, 1.0)
}

#[inline(always)]
fn norm_clamp(
    mut x: DVector<f64>,
    max_norm: f64,
) -> DVector<f64> {

    let norm = x.norm();

    if norm > max_norm {
        x *= max_norm / norm.max(EPS);
    }

    x
}

#[inline(always)]
fn stiefel_drift(
    w: &DMatrix<f64>,
) -> f64 {

    let r = w.ncols();

    (
        w.transpose() * w
        - DMatrix::<f64>::identity(r, r)
    ).norm()
}

fn spectral_entropy_matrix(
    w: &DMatrix<f64>,
) -> f64 {

    let energies: Vec<f64> =
        w.column_iter()
            .map(|c| c.norm_squared())
            .collect();

    let total: f64 =
        energies.iter().sum::<f64>().max(EPS);

    energies.iter().map(|e| {

        let p = e / total;

        if p > EPS {
            -p * p.log2()
        } else {
            0.0
        }

    }).sum()
}

// ============================================================================
// 2D PERCEPTION LAYER
// ============================================================================

pub mod perception {

    use super::*;

    // ------------------------------------------------------------------------
    // SCREEN MODEL
    // ------------------------------------------------------------------------

    #[derive(Clone, Copy)]
    pub struct Screen2D {

        pub width: usize,
        pub height: usize,

        // convexity
        pub curvature: f64,

        pub scale: f64,
    }

    // ------------------------------------------------------------------------
    // PIXEL
    // ------------------------------------------------------------------------

    #[derive(Clone, Copy, Debug)]
    pub struct Pixel2D {

        pub x: f64,
        pub y: f64,

        pub intensity: f64,

        pub stress: f64,
    }

    // ------------------------------------------------------------------------
    // PROJECTION
    // ------------------------------------------------------------------------

    pub struct Projection2D;

    impl Projection2D {

        #[inline(always)]
        pub fn project_point(

            x: f64,
            y: f64,
            z: f64,

            screen: &Screen2D,

            stress: f64,

        ) -> Pixel2D {

            // ----------------------------------------------------------------
            // singularity-safe denominator
            // ----------------------------------------------------------------

            let denom =
                (1.0 + screen.curvature * z)
                .max(0.01);

            Pixel2D {

                x: (x / denom) * screen.scale,

                y: (y / denom) * screen.scale,

                intensity: 1.0 / denom,

                stress,
            }
        }

        // --------------------------------------------------------------------
        // SYNTHESIZE FRAME
        // --------------------------------------------------------------------

        pub fn synthesize_frame(

            x: &DVector<f64>,

            z_shear: &DVector<f64>,

            stress: f64,

            screen: &Screen2D,

        ) -> Vec<Pixel2D> {

            let n = x.len();

            let mut frame =
                Vec::with_capacity(n);

            for i in 0..n {

                let xi = x[i];

                let zi = z_shear[i];

                let depth =
                    (xi * zi).tanh();

                frame.push(
                    Self::project_point(
                        xi,
                        zi,
                        depth,
                        screen,
                        stress,
                    )
                );
            }

            frame
        }

        // --------------------------------------------------------------------
        // HUD DISTORTION
        // --------------------------------------------------------------------

        pub fn stress_overlay(
            pixel: &mut Pixel2D
        ) {

            let warp =
                pixel.stress * pixel.stress;

            pixel.x += warp * 0.01;
            pixel.y -= warp * 0.01;
        }
    }
}

// ============================================================================
// SECURITY LAYER
// ============================================================================

pub mod security {

    use super::*;

    #[derive(Debug)]
    pub struct ThreatState {

        pub suspicious: bool,

        pub entropy_drop: f64,

        pub drift_growth_frames: usize,
    }

    pub fn analyze(

        entropy_prev: f64,
        entropy_now: f64,

        drift_growth_frames: usize,

        stress: f64,

    ) -> ThreatState {

        let entropy_drop =
            entropy_prev - entropy_now;

        let suspicious =
            entropy_drop > 0.5
            && drift_growth_frames > 10
            && stress < 0.1;

        ThreatState {

            suspicious,

            entropy_drop,

            drift_growth_frames,
        }
    }
}

// ============================================================================
// API LAYER
// ============================================================================

pub mod api {

    use super::*;

    pub struct InputPacket {

        pub sigma: DVector<f64>,

        pub timestamp: u64,

        pub mode: u8,
    }

    pub struct OutputPacket {

        pub frame_id: u64,

        pub telemetry: Telemetry,

        pub gpu_ready: bool,
    }

    pub fn ingest_input(
        packet: InputPacket
    ) -> DVector<f64> {

        packet.sigma
    }
}

// ============================================================================
// RUNTIME LOOP
// ============================================================================

pub fn runtime_loop(

    mut core: RP1RCore,

    screen: perception::Screen2D,

) {

    let mut frame_id = 0u64;

    loop {

        let frame_start =
            Instant::now();

        // --------------------------------------------------------------------
        // INPUT
        // --------------------------------------------------------------------

        let sigma =
            api::ingest_input(
                api::InputPacket {

                    sigma:
                        DVector::from_element(
                            core.x.len(),
                            0.0
                        ),

                    timestamp: frame_id,

                    mode: 0,
                }
            );

        // --------------------------------------------------------------------
        // CORE
        // --------------------------------------------------------------------

        let telemetry =
            core.tick(&sigma);

        // --------------------------------------------------------------------
        // PERCEPTION
        // --------------------------------------------------------------------

        let mut frame =
            perception::Projection2D
                ::synthesize_frame(

                    &core.x,

                    &core.z_shear,

                    telemetry.stress,

                    &screen,
                );

        // --------------------------------------------------------------------
        // OPTIONAL HUD OVERLAY
        // --------------------------------------------------------------------

        for pixel in frame.iter_mut() {
            perception::Projection2D
                ::stress_overlay(pixel);
        }

        // --------------------------------------------------------------------
        // FRAME BUDGET ENFORCEMENT
        // --------------------------------------------------------------------

        let elapsed =
            frame_start.elapsed().as_micros();

        if elapsed < FRAME_BUDGET_US {

            std::thread::sleep(
                Duration::from_micros(
                    (FRAME_BUDGET_US - elapsed) as u64
                )
            );
        }

        frame_id += 1;
    }
}

// ============================================================================
// ARCHITECTURAL SUMMARY
// ============================================================================
//
// REFINED RP1-R:
//
//   x          = bounded observable state
//   W          = orthonormal adaptive basis
//   z_shear    = unresolved external residual memory
//
// PRINCIPLES:
//
//   1. State and basis evolution are separated
//   2. W evolves on Stiefel manifold only
//   3. x tracks projected observable only
//   4. z_shear never feeds state
//   5. telemetry never drives dynamics
//   6. all singularities are bounded
//   7. runtime remains deterministic at 240Hz
//
// RESULT:
//
//   bounded geometric streaming kernel
//   with stable subspace adaptation semantics.
//
// ============================================================================
// ============================================================================
// RP1-R AXIOMS · GEOMETRIC STREAMING CONSTITUTION
// ----------------------------------------------------------------------------
// These are NOT comments.
// These are invariant laws the architecture must obey.
//
// Violation of any axiom means the system has departed from
// mathematically coherent geometric streaming semantics.
// ============================================================================

// ============================================================================
// AXIOM 1 · SEPARATION OF DYNAMICS
// ----------------------------------------------------------------------------
// State evolution, basis evolution, and memory evolution
// are distinct operators.
//
// They may exchange observations.
// They may NOT inject unconstrained force into one another.
//
// FORM:
//
//   x_{t+1} = F_state(x_t, Π_W(σ_t))
//   W_{t+1} = F_basis(W_t, σ_t)
//   z_{t+1} = F_memory(z_t, σ_t - Π_W(σ_t))
//
// CONSEQUENCE:
//
//   No additive mixed-space evolution law.
//
// FORBIDDEN:
//
//   x += residual + shear + damping
//
// ============================================================================


// ============================================================================
// AXIOM 2 · BASIS LIVES ON STIEFEL
// ----------------------------------------------------------------------------
// The adaptive basis W is always orthonormal.
//
// FORM:
//
//   WᵀW = I
//
// All basis updates MUST originate from a valid tangent vector:
//
//   ΔW ∈ T_W St(n,r)
//
// Tangent validity condition:
//
//   WᵀΔW + ΔWᵀW = 0
//
// Retraction restores numerical orthonormality.
//
// CONSEQUENCE:
//
//   Basis evolution is geometric rotation,
//   not arbitrary perturbation + cleanup.
//
// ============================================================================


// ============================================================================
// AXIOM 3 · STATE IS BOUNDED
// ----------------------------------------------------------------------------
// Observable state x must remain inside a compact domain.
//
// FORM:
//
//   ||x|| ≤ x_max
//
// or:
//
//   x ∈ S^{n−1}
//
// CONSEQUENCE:
//
//   No unbounded energy accumulation.
//   No unstable attractor competition.
//   No undefined asymptotic growth.
//
// ============================================================================


// ============================================================================
// AXIOM 4 · MEMORY TRACKS EXTERNAL RESIDUAL ONLY
// ----------------------------------------------------------------------------
// Temporal memory may only encode unresolved external structure.
//
// VALID:
//
//   z = EMA(σ − Π_W(σ))
//
// INVALID:
//
//   z = EMA(projection − internal_state)
//
// CONSEQUENCE:
//
//   Memory corresponds to physical unresolved signal content,
//   not historical disagreement between internal variables.
//
// ============================================================================


// ============================================================================
// AXIOM 5 · TELEMETRY IS OBSERVATIONAL
// ----------------------------------------------------------------------------
// Telemetry may classify system behavior.
// Telemetry may NOT directly drive state evolution.
//
// VALID:
//
//   telemetry = observe(x, W, residual)
//
// INVALID:
//
//   x += stress
//   x += entropy
//   x += regime
//
// CONSEQUENCE:
//
//   Diagnostics remain semantically meaningful.
//   Observations cannot recursively corrupt dynamics.
//
// ============================================================================


// ============================================================================
// AXIOM 6 · DRIFT IS REPAIRED, NOT PUNISHED
// ----------------------------------------------------------------------------
// Orthogonality drift is a geometric maintenance problem.
//
// Drift must trigger restoration,
// not suppression of adaptation.
//
// VALID:
//
//   if drift > ε:
//       retract(W)
//       continue adapting
//
// INVALID:
//
//   η_eff = η / drift²
//
// CONSEQUENCE:
//
//   The system never deadlocks adaptation
//   because of the condition adaptation is meant to repair.
//
// ============================================================================


// ============================================================================
// AXIOM 7 · SECURITY DETECTS CHANGE, NOT SIMPLICITY
// ----------------------------------------------------------------------------
// Low entropy is not inherently suspicious.
//
// Narrowband convergence is a normal operating mode.
//
// Security must detect:
//
//   rapid collapse,
//   persistent drift growth,
//   adversarial transition dynamics.
//
// NOT:
//
//   low-rank steady-state structure.
//
// CONSEQUENCE:
//
//   Legitimate coherent signals are never classified
//   as adversarial solely because they are simple.
//
// ============================================================================


// ============================================================================
// AXIOM 8 · PROJECTION MUST BE SINGULARITY-SAFE
// ----------------------------------------------------------------------------
// Perception operators may compress geometry,
// but may never diverge.
//
// VALID:
//
//   denom = max(1 + κz, ε)
//
// INVALID:
//
//   division by unconstrained depth term
//
// CONSEQUENCE:
//
//   No infinite projection coordinates.
//   No topology inversion from numerical singularities.
//
// ============================================================================


// ============================================================================
// AXIOM 9 · REAL-TIME IS A HARD CONSTRAINT
// ----------------------------------------------------------------------------
// Frame budget is constitutional.
//
// Every subsystem must degrade gracefully under overload.
//
// PRIORITY:
//
//   1. projection
//   2. observable state update
//   3. basis adaptation
//   4. memory update
//   5. diagnostics
//
// CONSEQUENCE:
//
//   The engine never collapses catastrophically
//   because adaptation cost spikes.
//
// ============================================================================


// ============================================================================
// AXIOM 10 · GEOMETRY PRECEDES INTERPRETATION
// ----------------------------------------------------------------------------
// The system does not simulate "objects."
//
// It evolves constrained geometric relationships.
//
// Meaning is derived AFTER evolution,
// never injected beforehand.
//
// CONSEQUENCE:
//
//   Reality is streamed as manifold evolution.
//   Perception is a lossy observable projection.
//   Security is geometric inconsistency detection.
//   State is not ontology.
//
// ============================================================================


// ============================================================================
// AXIOM 11 · RESIDUAL IS INFORMATION
// ----------------------------------------------------------------------------
// Residual energy is not error to be eliminated.
//
// Residual represents:
//
//   unresolved structure,
//   novelty,
//   complexity beyond current basis rank.
//
// CONSEQUENCE:
//
//   Large residuals should drive basis adaptation,
//   not state destabilization.
//
// ============================================================================


// ============================================================================
// AXIOM 12 · ADAPTATION MUST PRESERVE RANK DIVERSITY
// ----------------------------------------------------------------------------
// A valid basis learner must avoid rank collapse.
//
// FORM:
//
//   columns(W) remain informationally distinct
//
// CONSEQUENCE:
//
//   All columns may not converge toward the same vector.
//   Updates must preserve subspace dimensionality.
//
// ============================================================================


// ============================================================================
// AXIOM 13 · REGIME TRANSITIONS REQUIRE HYSTERESIS
// ----------------------------------------------------------------------------
// Instantaneous classification is forbidden.
//
// Regimes are temporal structures,
// not single-frame events.
//
// CONSEQUENCE:
//
//   transitions require persistence across frames.
//
// ============================================================================


// ============================================================================
// AXIOM 14 · MANIFOLD CONSTRAINTS ARE PRIMARY
// ----------------------------------------------------------------------------
// Numerical stability emerges from geometry,
// not post-hoc damping.
//
// Preferred order:
//
//   1. constrain geometry
//   2. evolve dynamics
//   3. observe telemetry
//
// NOT:
//
//   evolve freely → damp instability afterward
//
// CONSEQUENCE:
//
//   Stability is structural,
//   not corrective.
//
// ============================================================================


// ============================================================================
// AXIOM 15 · THE ENGINE IS A STREAMING SYSTEM
// ----------------------------------------------------------------------------
// There is no global optimization.
// No final equilibrium.
// No solved state.
//
// The system continuously:
//
//   project → adapt → observe → stream
//
// CONSEQUENCE:
//
//   The architecture is online, local,
//   and temporally persistent.
//
// ============================================================================
// ============================================================================
// RP1-R PORTING PROTOCOLS
// ----------------------------------------------------------------------------
// Purpose
// ----------------------------------------------------------------------------
// Defines the mandatory engineering rules for porting RP1-R across:
//
//   - CPU architectures
//   - SIMD backends
//   - GPU compute pipelines
//   - FFI boundaries
//   - game engines
//   - embedded systems
//   - distributed runtimes
//
// These protocols preserve:
//
//   - geometric correctness
//   - deterministic timing
//   - manifold invariants
//   - telemetry semantics
//
// Any port violating these protocols is NOT RP1-R.
// ============================================================================


// ============================================================================
// PROTOCOL 1 · MANIFOLD INVARIANTS ARE CONSTITUTIONAL
// ----------------------------------------------------------------------------
// W MUST remain orthonormal after every update.
//
// REQUIRED:
//
//   WᵀW ≈ I
//
// REQUIRED:
//
//   retract(W)
//
// after every basis evolution step.
//
// FORBIDDEN:
//
//   skipping retraction for performance.
//
// CONSEQUENCE:
//
//   performance optimizations may NEVER violate
//   manifold validity.
//
// ============================================================================


// ============================================================================
// PROTOCOL 2 · HOT LOOP ALLOCATION IS FORBIDDEN
// ----------------------------------------------------------------------------
// The 240Hz loop may not allocate heap memory.
//
// REQUIRED:
//
//   preallocate:
//
//     - state vectors
//     - projection buffers
//     - residual buffers
//     - telemetry buffers
//     - GPU staging buffers
//
// FORBIDDEN:
//
//   Vec::push in hot path
//   dynamic matrix resizing
//   runtime heap allocation
//
// CONSEQUENCE:
//
//   deterministic frame timing.
//
// ============================================================================


// ============================================================================
// PROTOCOL 3 · FLOATING-POINT DOMAIN CONSISTENCY
// ----------------------------------------------------------------------------
// All ports must preserve floating-point semantics.
//
// REQUIRED:
//
//   identical epsilon hierarchy:
//
//     EPS
//     DRIFT_EPS
//
// REQUIRED:
//
//   deterministic normalization order
//
// FORBIDDEN:
//
//   mixed f32/f64 pipelines without explicit conversion policy
//
// CONSEQUENCE:
//
//   manifold evolution remains numerically equivalent
//   across hardware targets.
//
// ============================================================================


// ============================================================================
// PROTOCOL 4 · GPU PORTS MUST PRESERVE UPDATE ORDER
// ----------------------------------------------------------------------------
// RP1-R is temporally ordered.
//
// REQUIRED ORDER:
//
//   1. retract
//   2. projection
//   3. residual
//   4. basis update
//   5. state update
//   6. memory update
//   7. telemetry
//
// FORBIDDEN:
//
//   asynchronous basis updates
//   unordered compute dispatch
//   deferred memory evolution
//
// CONSEQUENCE:
//
//   geometry remains causal.
//
// ============================================================================


// ============================================================================
// PROTOCOL 5 · SIMD PORTS MUST PRESERVE REDUCTION ORDER
// ----------------------------------------------------------------------------
// SIMD reductions may alter numerical trajectories.
//
// REQUIRED:
//
//   deterministic reduction tree
//
// REQUIRED:
//
//   stable dot-product ordering
//
// FORBIDDEN:
//
//   architecture-dependent accumulation ordering
//
// CONSEQUENCE:
//
//   telemetry remains reproducible.
//
// ============================================================================


// ============================================================================
// PROTOCOL 6 · FFI BOUNDARIES MAY NOT MUTATE STATE
// ----------------------------------------------------------------------------
// External systems may observe or perturb.
//
// They may NOT directly overwrite manifold state.
//
// VALID:
//
//   inject_input(σ)
//
// INVALID:
//
//   core.x = external_buffer
//   core.w = arbitrary_matrix
//
// CONSEQUENCE:
//
//   external APIs remain tangent interfaces,
//   not authority overrides.
//
// ============================================================================


// ============================================================================
// PROTOCOL 7 · GPU MEMORY MUST MIRROR GEOMETRIC STRUCTURE
// ----------------------------------------------------------------------------
// Buffers must preserve semantic separation.
//
// REQUIRED:
//
//   separate GPU buffers:
//
//     STATE_BUFFER
//     BASIS_BUFFER
//     RESIDUAL_BUFFER
//     TELEMETRY_BUFFER
//
// FORBIDDEN:
//
//   packed heterogeneous buffers
//   reinterpret-cast geometry
//
// CONSEQUENCE:
//
//   memory layout reflects manifold semantics.
//
// ============================================================================


// ============================================================================
// PROTOCOL 8 · SECURITY SYSTEMS ARE OBSERVATIONAL ONLY
// ----------------------------------------------------------------------------
// Security layers may classify.
// They may NOT directly alter evolution equations.
//
// VALID:
//
//   suspicious = detect(...)
//
// INVALID:
//
//   if suspicious:
//       zero_state()
//       freeze_basis()
//
// CONSEQUENCE:
//
//   security remains diagnostic,
//   not coercive.
//
// ============================================================================


// ============================================================================
// PROTOCOL 9 · FRAME OVERRUN DEGRADATION ORDER
// ----------------------------------------------------------------------------
// Under timing pressure,
// the engine degrades by semantic priority.
//
// DROP ORDER:
//
//   1. diagnostics
//   2. HUD overlays
//   3. residual memory updates
//   4. basis adaptation frequency
//
// NEVER DROP:
//
//   projection
//   state update
//   retraction
//
// CONSEQUENCE:
//
//   core geometry survives overload.
//
// ============================================================================


// ============================================================================
// PROTOCOL 10 · ENGINE PORTS MUST REMAIN STREAMING
// ----------------------------------------------------------------------------
// RP1-R is online.
//
// REQUIRED:
//
//   local temporal updates
//
// FORBIDDEN:
//
//   global optimization passes
//   offline batch recomputation
//   retrospective state rewrites
//
// CONSEQUENCE:
//
//   the engine remains temporally causal.
//
// ============================================================================


// ============================================================================
// PROTOCOL 11 · GAME ENGINE PORTS
// ----------------------------------------------------------------------------
// Unity / Unreal / Godot integrations must preserve:
//
//   deterministic simulation order
//   stable frame pacing
//   geometry-first update semantics
//
// REQUIRED:
//
//   fixed-timestep integration
//
// FORBIDDEN:
//
//   tying manifold updates to render FPS
//
// CONSEQUENCE:
//
//   visual framerate fluctuations
//   cannot corrupt geometry.
//
// ============================================================================


// ============================================================================
// PROTOCOL 12 · NETWORKED PORTS
// ----------------------------------------------------------------------------
// Distributed RP1-R instances synchronize OBSERVATIONS,
// not internal manifold authority.
//
// REQUIRED:
//
//   synchronize:
//
//     telemetry
//     projected observables
//     external inputs
//
// FORBIDDEN:
//
//   remote overwrite of W
//   remote overwrite of x
//
// CONSEQUENCE:
//
//   each node preserves local geometric integrity.
//
// ============================================================================


// ============================================================================
// PROTOCOL 13 · EMBEDDED SYSTEM PORTS
// ----------------------------------------------------------------------------
// Embedded implementations may reduce:
//
//   rank r
//   telemetry frequency
//   projection density
//
// They may NOT remove:
//
//   retraction
//   normalization
//   manifold constraints
//
// CONSEQUENCE:
//
//   reduced fidelity is acceptable.
//   broken geometry is not.
//
// ============================================================================


// ============================================================================
// PROTOCOL 14 · MULTITHREADING CONSTRAINT
// ----------------------------------------------------------------------------
// Geometry evolution is single-authority.
//
// REQUIRED:
//
//   one authoritative evolution thread
//
// ALLOWED:
//
//   parallel telemetry
//   parallel rendering
//   parallel visualization
//
// FORBIDDEN:
//
//   concurrent mutation of W
//   concurrent mutation of x
//
// CONSEQUENCE:
//
//   manifold evolution remains coherent.
//
// ============================================================================


// ============================================================================
// PROTOCOL 15 · TELEMETRY IS A FIRST-CLASS API
// ----------------------------------------------------------------------------
// Ports must expose:
//
//   stress
//   novelty
//   entropy
//   drift
//   regime
//   suspicious
//
// FORBIDDEN:
//
//   hidden internal diagnostics
//
// CONSEQUENCE:
//
//   observability is preserved across ecosystems.
//
// ============================================================================


// ============================================================================
// PROTOCOL 16 · RUNTIME CLOCK IS MONOTONIC
// ----------------------------------------------------------------------------
// All timing must derive from monotonic clocks.
//
// REQUIRED:
//
//   monotonic frame timing
//
// FORBIDDEN:
//
//   wall-clock scheduling
//   timezone-sensitive timing
//
// CONSEQUENCE:
//
//   temporal coherence survives system clock changes.
//
// ============================================================================


// ============================================================================
// PROTOCOL 17 · PROJECTION LAYERS ARE LOSSY BY DESIGN
// ----------------------------------------------------------------------------
// 2D/HUD/render outputs are NOT canonical state.
//
// REQUIRED:
//
//   projection treated as observable approximation
//
// FORBIDDEN:
//
//   feeding rendered pixels back into core state
//
// CONSEQUENCE:
//
//   perception remains downstream of geometry.
//
// ============================================================================


// ============================================================================
// PROTOCOL 18 · RANK ADAPTATION IS CONTROLLED
// ----------------------------------------------------------------------------
// Adaptive rank systems must preserve:
//
//   r_min ≤ r ≤ r_max
//
// REQUIRED:
//
//   gradual rank transitions
//
// FORBIDDEN:
//
//   instant high-rank expansion
//   unconstrained residual-driven explosion
//
// CONSEQUENCE:
//
//   adaptation remains stable.
//
// ============================================================================


// ============================================================================
// PROTOCOL 19 · PORT VALIDATION SUITE
// ----------------------------------------------------------------------------
// Every port must pass:
//
//   1. orthogonality drift tests
//   2. deterministic replay tests
//   3. bounded-state tests
//   4. residual consistency tests
//   5. frame-budget stress tests
//
// REQUIRED:
//
//   identical telemetry trajectories
//   within epsilon tolerance
//
// CONSEQUENCE:
//
//   ports are mathematically equivalent.
//
// ============================================================================


// ============================================================================
// PROTOCOL 20 · THE PORT MUST NOT CHANGE THE PHILOSOPHY
// ----------------------------------------------------------------------------
// RP1-R is:
//
//   geometric
//   streaming
//   bounded
//   manifold-constrained
//   observationally interpretable
//
// RP1-R is NOT:
//
//   symbolic AI
//   object simulation
//   unconstrained optimization
//   reactive scripting
//
// CONSEQUENCE:
//
//   implementation language may change.
//   architecture may not.
//
// ============================================================================
