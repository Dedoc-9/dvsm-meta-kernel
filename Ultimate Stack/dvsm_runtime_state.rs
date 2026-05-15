// ===============================================================
// dvsm_runtime_state.rs
// DVSM-π+++ · Runtime Ghost State Machine Layer
// File 2 in unified system progression
//
// PURPOSE:
// This layer formalizes "Ghost Mode" as a deterministic,
// bounded runtime state machine over spectral observables.
//
// It sits ABOVE particle dynamics (μ_t) and BELOW GPU kernels.
// It is the CONTRACT layer that all GPU execution must obey.
//
// CRITICAL ARCHITECTURE RULE:
// μ_t  → drives → Z_t, S_t, W_t
// Z_t  → NEVER feeds back into μ_t   (AIR-GAP INVARIANT)
// ===============================================================

use std::f64::consts::EPSILON;

// ===============================================================
// GHOST STATE MACHINE
// ===============================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GhostState {
    Dormant,
    Echo,
    Burst,
    FullGhost,
}

// ===============================================================
// SPECTRAL OBSERVABLES (CPU TRUTH LAYER)
// ===============================================================

#[derive(Debug, Clone)]
pub struct SpectralMetrics {
    pub z_norm: f64,             // spectral field energy
    pub s_norm: f64,             // memory field energy
    pub ess: f64,               // particle degeneracy proxy
    pub burst_metric: f64,      // S/Z mismatch ratio
    pub non_normal_energy: f64, // Lie-bracket amplification proxy
}

// ===============================================================
// GHOST CLASSIFIER (DETERMINISTIC CONTRACT)
// ===============================================================

pub fn classify_ghost(m: &SpectralMetrics) -> GhostState {
    // Hard collapse regime: particle system degeneracy
    if m.ess < 0.25 {
        return GhostState::Burst;
    }

    // Full spectral autonomy condition (operator detachment)
    if m.burst_metric > 2.5 && m.non_normal_energy > 1.8 {
        return GhostState::FullGhost;
    }

    // Pre-instability resonance
    if m.burst_metric > 1.2 {
        return GhostState::Echo;
    }

    GhostState::Dormant
}

// ===============================================================
// NON-NORMAL ENERGY ESTIMATOR
// ===============================================================

/// Measures Lie-bracket amplification without eigen-decomposition.
/// This is the GPU-compatible proxy for transient growth potential.
pub fn non_normal_energy(z: &[f64], s: &[f64]) -> f64 {
    let n = z.len().max(1);
    let mut energy = 0.0;

    for i in 0..z.len() {
        for j in 0..s.len() {
            if i == j { continue; }

            // Lie-bracket style antisymmetric interaction
            let term = (z[i] * s[j] - z[j % s.len()] * s[i]).abs();
            energy += term;
        }
    }

    energy / (n as f64)
}

// ===============================================================
// BURST METRIC (NON-NORMALITY OBSERVABLE)
// ===============================================================

pub fn burst_metric(z: &[f64], s: &[f64]) -> f64 {
    let z_norm = l2_norm(z);
    let s_norm = l2_norm(s);

    s_norm / (z_norm + EPSILON)
}

// ===============================================================
// L2 NORM (STABLE REDUCTION PRIMITIVE)
// ===============================================================

pub fn l2_norm(x: &[f64]) -> f64 {
    let mut sum = 0.0;
    for v in x {
        sum += v * v;
    }
    sum.sqrt()
}

// ===============================================================
// SPECTRAL METRIC BUILDER (CPU CONSENSUS LAYER)
// ===============================================================

pub fn build_metrics(
    z: &[f64],
    s: &[f64],
    ess: f64,
) -> SpectralMetrics {
    SpectralMetrics {
        z_norm: l2_norm(z),
        s_norm: l2_norm(s),
        ess,
        burst_metric: burst_metric(z, s),
        non_normal_energy: non_normal_energy(z, s),
    }
}

// ===============================================================
// AIR-GAP INVARIANT (CRITICAL SYSTEM RULE)
// ===============================================================

/// AIR-GAP RULE:
/// - μ_t (particle measure) drives Z_t
/// - Z_t MUST NOT influence μ_t directly
/// - only observables (metrics) can be reported back
///
/// This prevents:
/// - recursive spectral collapse
/// - GPU feedback loops
/// - uncontrolled "ghost self-amplification"
pub fn enforce_air_gap() {
    // intentionally empty: this is a structural invariant, not a runtime call
}

// ===============================================================
// NEXT-STAGE EXECUTION NOTES (DEV LAYER MAP)
// ===============================================================
//
// STAGE 3: GPU Z-FIELD KERNEL
// -----------------------------------------
// - Implement Lie-bracket update in WGSL
// - Z-field becomes parallel antisymmetric flow
// - Must consume ONLY μ_t-derived inputs
// - Must NOT modify μ_t directly
//
// STAGE 4: GPU REDUCTION PASS
// -----------------------------------------
// - ESS computation on GPU
// - L2 norms via tree reduction
// - B(t) burst metric streaming
//
// STAGE 5: VR FIELD RENDERER
// -----------------------------------------
// Z_t → vertex displacement field
// W_t → basis orientation vectors
// S_t → temporal blur / hysteresis layer
//
// STAGE 6: FULL EXECUTABLE DVSM ENGINE
// -----------------------------------------
// CPU:
//   - particle SDE + SMC
//   - ghost classifier
//   - CLT diagnostics
//
// GPU:
//   - spectral evolution (Z)
//   - reduction kernels
//
// OUTPUT:
//   - VR manifold + stability overlays
//
// ===============================================================
//
// GHOST THEORY NOTE (IMPORTANT):
// --------------------------------
// "Full Ghost" is not a simulation target.
// It is a detected regime where:
//
//   non_normal_energy >> dissipation
//   AND burst_metric exceeds stability envelope
//
// This is treated as a CONTROL STATE, not a goal.
//
// ===============================================================
