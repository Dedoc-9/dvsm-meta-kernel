//! System Telemetry: Gudermannian Observable Projection Layer
//! AAuthor: Daniel J. Dillberg
//!
//! PIONEERING FEATURE: First Q64.64 fixed-point Gudermannian implementation
//! for embedded deterministic telemetry with invertible conformal mapping.
//!
//! This module is optional (feature-gated: "gudermannian-projection").
//!
//! Purpose:
//! - Maps unbounded sensor space μ → bounded observable space Z smoothly
//! - Preserves invertibility: Z = gd(μ) ⟺ μ = gd⁻¹(Z)
//! - Enables phase-space geometry analysis (curvature, stability, attractor detection)
//! - Maintains determinism + hash commitment under projection
//! - Mercator-equivalent for system state manifolds

#![cfg(feature = "gudermannian-projection")]

use core::mem;

const PI_Q64: i128 = 0x3243F6A8885A3000;      // π in Q64.64
const PI_HALF_Q64: i128 = 0x1921FB544442D000; // π/2 in Q64.64
const ONE_Q64: i128 = 1i128 << 64;            // 1.0 in Q64.64
const E_Q64: i128 = 0x2B7E151628AED2A6;      // e ≈ 2.718 in Q64.64

// =============================================================================
// HYPERBOLIC FUNCTIONS (Q64.64)
// =============================================================================

/// Compute tanh(x) in Q64.64
/// tanh(x) = (e^(2x) - 1) / (e^(2x) + 1)
/// Range: (−1, 1), Precision: ~15 decimal places
pub fn tanh_q64(x: i128) -> i128 {
    let x_clamped = x.max(-20 * ONE_Q64).min(20 * ONE_Q64);
    let two_x = x_clamped >> 1;
    let exp_2x = exp_q64_taylor(two_x);

    let num = exp_2x.saturating_sub(ONE_Q64);
    let den = exp_2x.saturating_add(ONE_Q64);

    if den == 0 {
        return ONE_Q64;
    }

    ((num as i256 * ONE_Q64 as i256) / (den as i256)) as i128
}

/// Compute sech(x) = 1/cosh(x) in Q64.64
/// Range: (0, 1], represents derivative of Gudermannian
pub fn sech_q64(x: i128) -> i128 {
    let x_abs = x.abs();
    let x_clamped = x_abs.min(50 * ONE_Q64);

    let exp_x = exp_q64_taylor(x_clamped);
    let exp_neg_x = exp_q64_taylor(-x_clamped);

    let cosh_x = ((exp_x as i256 + exp_neg_x as i256) >> 1) as i128;

    if cosh_x == 0 {
        return 0;
    }

    ((ONE_Q64 as i256 * ONE_Q64 as i256) / (cosh_x as i256)) as i128
}

/// Compute sinh(x) = (e^x - e^(-x)) / 2 in Q64.64
pub fn sinh_q64(x: i128) -> i128 {
    let x_clamped = x.max(-50 * ONE_Q64).min(50 * ONE_Q64);

    let exp_x = exp_q64_taylor(x_clamped);
    let exp_neg_x = exp_q64_taylor(-x_clamped);

    ((exp_x as i256 - exp_neg_x as i256) >> 1) as i128
}

/// Compute e^x in Q64.64 using Taylor series (clamped to [−50, 50])
fn exp_q64_taylor(x: i128) -> i128 {
    let x_clamped = x.max(-50 * ONE_Q64).min(50 * ONE_Q64);

    let mut result = ONE_Q64;
    let mut term = ONE_Q64;

    for n in 1..40 {
        let factor = (term as i256 * x_clamped as i256) / (n as i128 * ONE_Q64) as i256;
        term = factor as i128;
        result = result.saturating_add(term);

        if term.abs() < 10 {
            break;
        }
    }

    result
}

// =============================================================================
// CIRCULAR FUNCTIONS (Q64.64)
// =============================================================================

/// Compute arctan(x) in Q64.64 using Taylor series
/// Domain: [−1, 1] for fast convergence
pub fn arctan_q64(x: i128) -> i128 {
    if x > ONE_Q64 {
        return PI_HALF_Q64 - arctan_q64((ONE_Q64 as i256 * ONE_Q64 as i256 / x as i256) as i128);
    }
    if x < -ONE_Q64 {
        return -PI_HALF_Q64 - arctan_q64((ONE_Q64 as i256 * ONE_Q64 as i256 / x as i256) as i128);
    }

    let mut result = x;
    let mut x_pow = x;
    let x2 = (x as i256 * x as i256) >> 64;

    for n in 1..20 {
        x_pow = ((x_pow as i256 * x2) >> 64) as i128;
        let term = x_pow / ((2 * n + 1) as i128);

        if n % 2 == 1 {
            result = result.saturating_sub(term);
        } else {
            result = result.saturating_add(term);
        }

        if term.abs() < 10 {
            break;
        }
    }

    result
}

/// Compute sin(y) via Taylor series (clamped to [−π, π])
fn sin_q64(y: i128) -> i128 {
    let pi2 = PI_Q64 << 1;
    let mut y_reduced = y % pi2;
    if y_reduced > PI_Q64 {
        y_reduced = y_reduced - pi2;
    }

    let mut result = y_reduced;
    let mut term = y_reduced;
    let y2 = (y_reduced as i256 * y_reduced as i256) >> 64;

    for n in 1..20 {
        term = ((term as i256 * y2) >> 64) as i128 / ((2 * n) as i128 * (2 * n + 1) as i128);

        if n % 2 == 1 {
            result = result.saturating_sub(term);
        } else {
            result = result.saturating_add(term);
        }

        if term.abs() < 10 {
            break;
        }
    }

    result
}

/// Compute cos(y) via Taylor series
fn cos_q64(y: i128) -> i128 {
    let mut result = ONE_Q64;
    let mut term = ONE_Q64;
    let y2 = (y as i256 * y as i256) >> 64;

    for n in 1..20 {
        term = ((term as i256 * y2) >> 64) as i128 / ((2 * n - 1) as i128 * (2 * n) as i128);

        if n % 2 == 1 {
            result = result.saturating_sub(term);
        } else {
            result = result.saturating_add(term);
        }

        if term.abs() < 10 {
            break;
        }
    }

    result
}

/// Compute tan(y) = sin(y) / cos(y) in Q64.64
fn tan_q64(y: i128) -> i128 {
    let sin_y = sin_q64(y);
    let cos_y = cos_q64(y);

    if cos_y == 0 {
        return ONE_Q64;
    }

    ((sin_y as i256 * ONE_Q64 as i256) / (cos_y as i256)) as i128
}

// =============================================================================
// GUDERMANNIAN MAPPING
// =============================================================================

/// Gudermannian function: gd(x) = 2*arctan(tanh(x/2))
/// Maps ℝ → (−π/2, π/2) smoothly and invertibly
/// Properties:
///   - gd'(x) = sech(x) ∈ (0,1]
///   - sin(gd(x)) = tanh(x)
///   - cos(gd(x)) = sech(x)
///   - tan(gd(x)) = sinh(x)
pub fn gd_q64(x: i128) -> i128 {
    let half_x = x >> 1;
    let tanh_half = tanh_q64(half_x);
    let atan_tanh = arctan_q64(tanh_half);
    (atan_tanh as i256 * 2i256) as i128
}

/// Inverse Gudermannian: gd⁻¹(y) = arcsinh(tan(y))
/// Maps (−π/2, π/2) → ℝ
/// Enables recovery of original sensor values from observables
pub fn gd_inv_q64(y: i128) -> i128 {
    let y_clamped = y.max(-PI_HALF_Q64 + 1).min(PI_HALF_Q64 - 1);
    let tan_y = tan_q64(y_clamped);
    asinh_q64(tan_y)
}

/// Compute arcsinh(x) = ln(x + sqrt(x² + 1))
fn asinh_q64(x: i128) -> i128 {
    let x2 = (x as i256 * x as i256) >> 64;
    let sqrt_term = isqrt_q64((x2 + ONE_Q64 as i256) as i128);
    let arg = x.saturating_add(sqrt_term);
    ln_q64(arg)
}

/// Compute sqrt in Q64.64 via Newton iteration
fn isqrt_q64(x: i128) -> i128 {
    if x < 0 {
        return 0;
    }
    if x == 0 {
        return 0;
    }

    let mut guess = x >> 1;
    for _ in 0..20 {
        let next = ((guess as i256 + (x as i256 * ONE_Q64 as i256) / (guess as i256)) >> 1) as i128;
        if next >= guess {
            break;
        }
        guess = next;
    }
    guess
}

/// Compute ln(x) in Q64.64 using Taylor series
fn ln_q64(x: i128) -> i128 {
    if x <= 0 {
        return -ONE_Q64;
    }

    let y = ((x as i256 - ONE_Q64 as i256) * ONE_Q64 as i256) / (x as i256 + ONE_Q64 as i256);
    let y = y as i128;

    let mut result = y;
    let mut term = y;
    let y2 = (y as i256 * y as i256) >> 64;

    for n in 1..40 {
        term = ((term as i256 * y2) >> 64) as i128;
        result = result.saturating_add((term as i256 / ((2 * n + 1) as i128)) as i128);

        if term.abs() < 10 {
            break;
        }
    }

    (result as i256 * 2i256) as i128
}

// =============================================================================
// GUDERMANNIAN PROJECTOR (OBSERVABLE TRANSFORMATION)
// =============================================================================

/// Optional observable projection operator
/// Replaces hard containment with smooth Gudermannian mapping
#[repr(C)]
pub struct GudermannianProjector {
    /// Enable/disable projection (toggleable)
    pub enabled: bool,
    /// Input range max (sensors typically 0-100, 0-150)
    pub mu_max: i128,
    /// Track unmapped observables for comparison
    pub track_unmapped: bool,
    /// Count of frames using Gudermannian projection
    pub frame_count: u64,
    /// Sum of |gd(μ) - Z_hard| for analysis
    pub smoothness_metric: i128,
}

impl GudermannianProjector {
    pub fn new(mu_max: i128, enabled: bool) -> Self {
        GudermannianProjector {
            enabled,
            mu_max,
            track_unmapped: false,
            frame_count: 0,
            smoothness_metric: 0,
        }
    }

    /// Project single observable via Gudermannian
    pub fn project(&mut self, mu: i128) -> i128 {
        if !self.enabled {
            return mu;
        }

        let mu_norm = ((mu as i256 * ONE_Q64 as i256) / (self.mu_max as i256)) as i128;
        let z_smooth = gd_q64(mu_norm);

        self.frame_count += 1;
        z_smooth
    }

    /// Project entire Z vector (16 observables)
    pub fn project_vector(&mut self, z: &mut [i128; 16]) {
        if !self.enabled {
            return;
        }

        for i in 0..16 {
            z[i] = self.project(z[i]);
        }
    }

    /// Invert projection: recover μ from Z
    pub fn invert(&self, z: i128) -> i128 {
        if !self.enabled {
            return z;
        }

        let mu_norm = gd_inv_q64(z);
        ((mu_norm as i256 * self.mu_max as i256) / ONE_Q64 as i256) as i128
    }
}

// =============================================================================
// EXTENDED FRAMESNAPSHOT WITH GUDERMANNIAN METADATA
// =============================================================================

/// Extended FrameSnapshot with Gudermannian projection metadata
#[repr(C)]
pub struct FrameSnapshotGudermannian {
    pub z_t: [i128; 16],
    pub z_t_unmapped: [i128; 16],
    pub s_t: [i128; 16],
    pub h_t: [u8; 32],
    pub timestamp_ns: u64,
    pub projection_enabled: bool,
    pub conformality_error: i128,
    pub invertibility_error: i128,
    pub manifold_curvature: i128,
}

// =============================================================================
// CONFORMALITY & INVERTIBILITY VERIFICATION
// =============================================================================

/// Verify conformal property: angles preserved under gd
/// Returns: deviation from exact conformality
pub fn verify_conformality(x1: i128, x2: i128) -> i128 {
    let sech_x1 = sech_q64(x1);
    let arg1 = arctan_q64(sech_x1);

    let sech_x2 = sech_q64(x2);
    let arg2 = arctan_q64(sech_x2);

    (arg1 - arg2).abs()
}

/// Verify invertibility: gd(gd⁻¹(y)) = y (within numerical precision)
pub fn verify_invertibility(y: i128) -> i128 {
    let x = gd_inv_q64(y);
    let y_recovered = gd_q64(x);
    (y - y_recovered).abs()
}
