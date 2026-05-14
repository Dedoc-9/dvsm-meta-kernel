/*
===========================================================
ENGINE V2.2 — ANISOTROPIC BURST GOVERNOR
Per-Mode Spectral Stabilization
Author: Daniel J. Dillberg
===========================================================

Instead of one global gain:

    gain = threshold / ||Z||

we compute:

    gain_k = threshold / |Z_k|

This prevents:

- dominant mode collapse
- rank starvation
- transient operator locking

===========================================================
*/

const MODE_THRESHOLD: f32 = 4.0;
const THERMAL_DECAY: f32 = 0.995;

pub struct AdaptiveEngine {
    // Mean field
    pub z: [f32; R],

    // EMA memory
    pub s: [f32; R],

    // Adaptive basis
    pub w: [[f32; 4]; R],

    // Global energy
    pub energy_norm: f32,

    // Per-mode gain
    pub gain: [f32; R],

    // Mode temperature
    pub temperature: [f32; R],
}

impl AdaptiveEngine {

    // =====================================================
    // PASS 1 — EXPECTATION + LOCAL BURST GOVERNOR
    // =====================================================

    #[inline(always)]
    pub fn compute_expectation(
        &mut self,
        samples: &[f32]
    ) {
        let mut local_z = [0.0f32; R];

        // ---------------------------------------------
        // projection accumulation
        // ---------------------------------------------
        for &sample in samples {

            let x2 = sample * sample;
            let x3 = x2 * sample;

            let b = [1.0, sample, x2, x3];

            for k in 0..R {

                let wk = &self.w[k];

                let phi =
                    wk[0] * b[0]
                    + wk[1] * b[1]
                    + wk[2] * b[2]
                    + wk[3] * b[3];

                local_z[k] += phi;
            }
        }

        // ---------------------------------------------
        // normalization
        // ---------------------------------------------
        let inv_n =
            1.0 / (samples.len() as f32 + 1e-6);

        self.energy_norm = 0.0;

        for k in 0..R {

            self.z[k] =
                local_z[k] * inv_n;

            let e =
                self.z[k].abs();

            // -----------------------------------------
            // mode-local gain compression
            // -----------------------------------------
            self.gain[k] =
                if e > MODE_THRESHOLD {
                    MODE_THRESHOLD / e
                } else {
                    1.0
                };

            // -----------------------------------------
            // anisotropic stabilization
            // -----------------------------------------
            self.z[k] *= self.gain[k];

            // -----------------------------------------
            // thermal accumulation
            // -----------------------------------------
            self.temperature[k] =
                THERMAL_DECAY
                * self.temperature[k]
                + (1.0 - THERMAL_DECAY)
                * e;

            self.energy_norm +=
                self.z[k] * self.z[k];
        }

        self.energy_norm =
            self.energy_norm.sqrt();
    }

    // =====================================================
    // PASS 2 — NON-NORMAL MEMORY UPDATE
    // =====================================================

    #[inline(always)]
    pub fn update_memory(&mut self) {

        for k in 0..R {

            let residual =
                self.z[k] - self.s[k];

            // -----------------------------------------
            // thermal damping
            // hot modes update slower
            // -----------------------------------------
            let thermal_gate =
                1.0 /
                (1.0 + self.temperature[k]);

            let alpha =
                EMA_ALPHA * thermal_gate;

            self.s[k] =
                alpha * self.s[k]
                + (1.0 - alpha)
                * residual;
        }
    }

    // =====================================================
    // PASS 3 — BASIS COOLING
    // =====================================================
    //
    // overheated modes are softly decorrelated
    //
    // prevents:
    // - spectral locking
    // - basis collapse
    // - runaway resonance
    //
    // =====================================================

    #[inline(always)]
    pub fn cool_basis(&mut self) {

        for k in 0..R {

            let t =
                self.temperature[k];

            if t < MODE_THRESHOLD {
                continue;
            }

            let cool =
                1.0 / (1.0 + 0.05 * t);

            for j in 0..4 {
                self.w[k][j] *= cool;
            }

            // renormalize
            let mut n = 0.0;

            for j in 0..4 {
                n += self.w[k][j]
                    * self.w[k][j];
            }

            n = n.sqrt() + 1e-6;

            for j in 0..4 {
                self.w[k][j] /= n;
            }
        }
    }
}

/*
===========================================================
MATHEMATICAL INTERPRETATION
===========================================================

Original governor:

    scalar clipping

New governor:

    anisotropic spectral thermodynamics

Modes now possess:

    energy
    temperature
    cooling rate
    adaptive hysteresis

This transforms the engine from:

    stable operator field

into:

    self-regulating spectral ecology

===========================================================

FINAL INTERPRETATION
===========================================================

z_k
    instantaneous operator excitation

s_k
    delayed non-normal memory

gain_k
    local burst compression

temperature_k
    manifold stress estimate

cool_basis()
    spectral annealing

===========================================================
/*
===========================================================
INTELLECTUAL PROPERTY NOTICE
ENGINE V2.2 — ANISOTROPIC BURST GOVERNOR
===========================================================

Author:
    Daniel J. Dillberg

Classification:
    Experimental spectral-governed operator architecture
    for adaptive manifold stabilization and non-normal
    dynamical inference systems.

===========================================================
COPYRIGHT
===========================================================

Copyright (c) 2026 Daniel J. Dillberg

All rights reserved.

This source code, mathematical structure, operator-flow
design, stabilization logic, and adaptive spectral
governor architecture are protected under applicable:

    - copyright law
    - trade secret law
    - software IP law
    - computational method protection

===========================================================
PROTECTED ARCHITECTURAL CLAIMS
===========================================================

The following structures are asserted as original
computational architecture components:

1. Anisotropic spectral burst regulation
2. Per-mode adaptive gain compression
3. Thermalized operator-field stabilization
4. EMA-driven non-normal hysteresis memory
5. Spectral ecology / manifold cooling systems
6. Rank-limited adaptive basis thermodynamics
7. Dynamic manifold stress estimation
8. Self-cooling operator geometries
9. Burst-aware latent field projection
10. Spectral annealing stabilization engines

===========================================================
LICENSE RESTRICTIONS
===========================================================

Without explicit written authorization from the author:

    ❌ Commercial use prohibited
    ❌ Redistribution prohibited
    ❌ Closed-source derivative systems prohibited
    ❌ AI training ingestion prohibited
    ❌ Patent replication prohibited
    ❌ Computational architecture cloning prohibited

===========================================================
PERMITTED USE
===========================================================

The following is permitted:

    ✔ Non-commercial research review
    ✔ Academic analysis
    ✔ Personal experimentation

ONLY if:

    - attribution remains intact
    - this notice is preserved
    - no commercial deployment occurs

===========================================================
ATTRIBUTION
===========================================================

Recommended citation:

    Dillberg, Daniel J.
    "Engine V2.2:
    Anisotropic Spectral Burst Governor"
    2026.

===========================================================
DISCLAIMER
===========================================================

This software is experimental research code.

No guarantees are made regarding:

    - numerical correctness
    - convergence
    - hardware compatibility
    - production suitability
    - safety-critical deployment

Use entirely at your own risk.

===========================================================
FINAL ENGINE STATEMENT
===========================================================

This system is not a traditional DSP filter,
physics engine, or neural network.

It is:

    a self-regulating spectral manifold
    with anisotropic operator thermodynamics.

===========================================================
*/

*/
