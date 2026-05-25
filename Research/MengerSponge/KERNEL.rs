//! System Telemetry Kernel: Q64.64 Fixed-Point + Menger Sponge Core
//! Portable, deterministic, cryptographically hardened
//!
//! Compile: cargo build --release
//! Test: cargo test --release

#![no_std]
#![allow(non_snake_case)]

extern crate alloc;
use alloc::vec::Vec;
use sha2::{Sha256, Digest};
use core::num::Wrapping as W;

// =============================================================================
// CONSTANTS & TYPES
// =============================================================================

pub const DIM: usize = 16;
pub const STATE_DIM: usize = 64;
pub const HASH_SIZE: usize = 32;
pub const RATE_LIMIT_NS: u64 = 1_000_000;  // 1000 fps

// Q64.64 Parameters
pub const LAMBDA_Q64: i128 = 0x000FFFFFFFFF0000;      // Fast dissipation
pub const ALPHA_Q64: i128 = 0xFFFFFFEF00000000;       // 0.99999
pub const BETA_Q64: i128 = 0xB333333300000000;        // 0.7
pub const DT_Q64: i128 = 0x000000000A000000;          // 0.039 ms
pub const E_TARGET_Q64: i128 = 0x0100000000000000;    // 1.0
pub const Z_MAX_Q64: i128 = 0x1000000000000000;       // 16.0

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameSnapshot {
    pub mu_t: [u8; STATE_DIM],
    pub z_t: [i128; DIM],
    pub s_t: [i128; DIM],
    pub h_t: [u8; HASH_SIZE],
    pub timestamp_ns: u64,
    pub menger_depth: u8,
}

impl Default for FrameSnapshot {
    fn default() -> Self {
        Self {
            mu_t: [0u8; STATE_DIM],
            z_t: [0i128; DIM],
            s_t: [0i128; DIM],
            h_t: [0u8; HASH_SIZE],
            timestamp_ns: 0,
            menger_depth: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SystemTelemetry {
    pub state: FrameSnapshot,
    pub w: [i128; DIM * STATE_DIM],  // Stiefel basis W
    pub kappa: [i128; DIM * DIM],    // Lie coupling tensor κ
    pub menger_mask: [bool; DIM * DIM],
    pub menger_depth: u8,
    pub frame_count: u64,
    pub rate_limit_last_ns: u64,
}

impl SystemTelemetry {
    pub fn new(menger_depth: u8) -> Self {
        let mut sys = Self {
            state: FrameSnapshot::default(),
            w: [0i128; DIM * STATE_DIM],
            kappa: [0i128; DIM * DIM],
            menger_mask: [true; DIM * DIM],
            menger_depth: menger_depth.min(2),
            frame_count: 0,
            rate_limit_last_ns: 0,
        };

        // Initialize W as identity (scaled)
        for i in 0..DIM {
            sys.w[i * STATE_DIM + i] = 1i128 << 63;  // 0.5 in Q64.64
        }

        // Initialize κ as random antisymmetric (deterministic seed)
        for i in 0..DIM {
            for j in i + 1..DIM {
                let seed = ((i * 17 + j * 31) as i128).wrapping_mul(0x9e3779b97f4a7c15);
                sys.kappa[i * DIM + j] = seed & ((1i128 << 32) - 1);
                sys.kappa[j * DIM + i] = -sys.kappa[i * DIM + j];
            }
        }

        // Generate Menger mask
        sys.menger_mask = menger_mask_generate(menger_depth);

        sys
    }
}

// =============================================================================
// QUANTIZATION & PROJECTION
// =============================================================================

#[inline]
fn quantize_q64(value: f64, max_phys: f64) -> i128 {
    if !value.is_finite() {
        return 0;
    }
    let normalized = (value / max_phys).clamp(0.0, 0.9999999999);
    ((normalized * ((1i128 << 64) as f64)) as i128).max(0)
}

#[inline]
fn dequantize_q64(quantized: i128, max_phys: f64) -> f64 {
    (quantized as f64 / ((1i128 << 64) as f64)) * max_phys
}

fn project_observable(mu: &[u8; STATE_DIM], w: &[i128; DIM * STATE_DIM]) -> [i128; DIM] {
    let mut z = [0i128; DIM];
    for k in 0..DIM {
        for d in 0..STATE_DIM {
            let mu_q = mu[d] as i128;
            z[k] = z[k].wrapping_add((((mu_q * w[k * STATE_DIM + d]) >> 64) as i128).max(0));
        }
    }
    z
}

fn hash_commit(mu: &[u8; STATE_DIM], z: &[i128; DIM], s: &[i128; DIM],
               protocol_ver: u32) -> [u8; HASH_SIZE] {
    let mut hasher = Sha256::new();
    hasher.update(mu);

    for &zi in z {
        hasher.update(&zi.to_le_bytes());
    }
    for &si in s {
        hasher.update(&si.to_le_bytes());
    }
    hasher.update(&protocol_ver.to_le_bytes());

    let mut hash = [0u8; HASH_SIZE];
    let result = hasher.finalize();
    hash.copy_from_slice(&result);
    hash
}

// =============================================================================
// MENGER SPONGE CORE
// =============================================================================

fn menger_mask_generate(depth: u8) -> [bool; DIM * DIM] {
    let mut mask = [true; DIM * DIM];
    if depth == 0 {
        return mask;
    }

    for level in 1..=depth {
        for i in 0..DIM {
            for j in 0..DIM {
                let i_level = (i / 3) % 3;
                let j_level = (j / 3) % 3;

                // Remove center (1,1) and 6 faces
                if (i_level == 1 && j_level == 1) ||  // Center
                   ((i_level == 0 || i_level == 2) && j_level == 1) ||  // Vertical faces
                   (i_level == 1 && (j_level == 0 || j_level == 2)) {  // Horizontal faces
                    mask[i * DIM + j] = false;
                }
            }
        }
    }
    mask
}

fn lie_bracket_step(z: &mut [i128; DIM], s: &[i128; DIM],
                     kappa: &[i128; DIM * DIM], mask: &[bool; DIM * DIM],
                     lambda: i128, dt: i128, alpha: i128) {
    // Compute energy for backreaction
    let e_sq = z.iter().map(|&zi| {
        let z_256 = zi as i256;
        ((z_256 * z_256) >> 64) as i128
    }).fold(0i128, |a, b| a.wrapping_add(b));

    let brake_coeff = alpha.wrapping_mul(e_sq.wrapping_sub(E_TARGET_Q64 >> 1));

    // Lie bracket accumulation: [Z,S]_κ = Σ_j κ_{kj}(Z_k·S_j - Z_j·S_k)
    for k in 0..DIM {
        let mut bracket = 0i256;

        for j in 0..DIM {
            if mask[k * DIM + j] {  // Menger sparsity gate
                let z_k = z[k] as i256;
                let z_j = z[j] as i256;
                let s_j = s[j] as i256;
                let s_k = s[k] as i256;
                let kappa_kj = kappa[k * DIM + j] as i256;

                bracket += ((z_k * s_j - z_j * s_k) * kappa_kj) >> 64;
            }
        }

        // Evolution: Z[k] ← Z[k] + dt · (bracket - λZ[k] + B[k])
        let lambda_256 = lambda as i256;
        let decay = (lambda_256 * z[k] as i256) >> 64;
        let backreact = (brake_coeff as i256 * z[k] as i256) >> 64;

        let f = bracket - decay - backreact;
        let delta = (f * dt as i256) >> 64;

        z[k] = ((z[k] as i256 + delta) as i128).max(0).min(Z_MAX_Q64);
    }
}

fn stiefel_retract(w: &mut [i128; DIM * STATE_DIM]) {
    // Gram-Schmidt orthogonalization per column
    for i in 0..DIM {
        // Compute norm of column i
        let mut norm_sq = 0i128;
        for d in 0..STATE_DIM {
            let w_di = w[d * DIM + i];
            norm_sq = norm_sq.wrapping_add(((w_di as i256 * w_di as i256) >> 64) as i128);
        }

        // Normalize column i
        if norm_sq > 0 {
            let norm_inv = ((1i128 << 96) / norm_sq).min((1i128 << 64) - 1);
            for d in 0..STATE_DIM {
                w[d * DIM + i] = (((w[d * DIM + i] as i256 * norm_inv as i256) >> 64) as i128).max(0);
            }
        }

        // Orthogonalize columns i+1..DIM against column i
        for j in i + 1..DIM {
            let mut proj = 0i128;
            for d in 0..STATE_DIM {
                let w_di = w[d * DIM + i];
                let w_dj = w[d * DIM + j];
                proj = proj.wrapping_add(((w_di as i256 * w_dj as i256) >> 64) as i128);
            }

            // Subtract projection: W[d,j] ← W[d,j] - proj·W[d,i]
            for d in 0..STATE_DIM {
                let w_di = w[d * DIM + i];
                let w_dj = w[d * DIM + j];
                let correction = ((proj as i256 * w_di as i256) >> 64) as i128;
                w[d * DIM + j] = w_dj.wrapping_sub(correction).max(0);
            }
        }
    }
}

// =============================================================================
// SEVEN-LAYER PIPELINE
// =============================================================================

fn l1_acquire(sensors: &[f64; STATE_DIM]) -> [i128; STATE_DIM] {
    let mut mu = [0i128; STATE_DIM];
    let ranges = [100.0, 100.0, 100.0, 150.0, 500.0,  // CPU, GPU, mem, therm, power
                  5000.0, 5000.0, 100.0, 255.0,       // freq_cpu, freq_gpu, bw, latency
                  100.0, 100.0, 100.0, 0.0, 0.0, 0.0]; // gpu_mem, disk, net, reserves

    for (i, &s) in sensors.iter().enumerate() {
        if i < ranges.len() && ranges[i] > 0.0 {
            mu[i] = quantize_q64(s, ranges[i]);
        }
    }
    mu
}

fn l2_torsion(mu: &[i128; STATE_DIM], prior: &[i128; STATE_DIM],
              prior_prior: &[i128; STATE_DIM]) -> [i128; STATE_DIM] {
    let mut result = [0i128; STATE_DIM];
    for i in 0..STATE_DIM {
        result[i] = mu[i].wrapping_sub((prior[i].wrapping_sub(prior_prior[i])) >> 1);
    }
    result
}

fn l3_dissipate(mu: &[i128; STATE_DIM], prior: &[i128; STATE_DIM],
                beta: i128) -> [i128; STATE_DIM] {
    let mut result = [0i128; STATE_DIM];
    for i in 0..STATE_DIM {
        let one_minus_beta = (1i128 << 64).wrapping_sub(beta);
        let term1 = ((mu[i] as i256 * beta as i256) >> 64) as i128;
        let term2 = ((prior[i] as i256 * one_minus_beta as i256) >> 64) as i128;
        result[i] = term1.wrapping_add(term2).max(0);
    }
    result
}

fn l4_backreact(mu: &[i128; STATE_DIM], curvature: i128) -> [i128; STATE_DIM] {
    let rho = if curvature > 0 {
        (curvature >> 1) / (1i128 + (curvature >> 1)).max(1)
    } else {
        0
    };

    let mut result = [0i128; STATE_DIM];
    for i in 0..STATE_DIM {
        result[i] = ((mu[i] as i256 * rho as i256) >> 64) as i128;
    }
    result
}

fn l5_spectral(mu: &[i128; STATE_DIM], prior: &[i128; STATE_DIM],
               weight: i128) -> [i128; STATE_DIM] {
    let one_minus_w = (1i128 << 64).wrapping_sub(weight);
    let mut result = [0i128; STATE_DIM];
    for i in 0..STATE_DIM {
        let t1 = ((mu[i] as i256 * weight as i256) >> 64) as i128;
        let t2 = ((prior[i] as i256 * one_minus_w as i256) >> 64) as i128;
        result[i] = t1.wrapping_add(t2).max(0);
    }
    result
}

fn l6_ema(z: &[i128; DIM], s_prior: &[i128; DIM], alpha: i128) -> [i128; DIM] {
    let one_minus_a = (1i128 << 64).wrapping_sub(alpha);
    let mut s_new = [0i128; DIM];

    for i in 0..DIM {
        let ghost = z[i].wrapping_sub(s_prior[i] >> 10);  // Simplified reconstruction error
        let term1 = ((s_prior[i] as i256 * alpha as i256) >> 64) as i128;
        let term2 = ((ghost as i256 * one_minus_a as i256) >> 64) as i128;
        s_new[i] = term1.wrapping_add(term2).max(0);
    }
    s_new
}

fn l7_hash(mu: &[u8; STATE_DIM], z: &[i128; DIM], s: &[i128; DIM]) -> [u8; HASH_SIZE] {
    hash_commit(mu, z, s, 1)
}

// =============================================================================
// FRAME PROCESSING (IMMUTABLE ORDERING)
// =============================================================================

pub fn process_frame(sys: &mut SystemTelemetry, sensors: &[f64; STATE_DIM],
                     now_ns: u64) -> Result<FrameSnapshot, &'static str> {
    // Rate limiting
    if now_ns < sys.rate_limit_last_ns + RATE_LIMIT_NS {
        return Err("rate_limit_exceeded");
    }
    sys.rate_limit_last_ns = now_ns;

    // L1: ACQUIRE
    let mu_l1 = l1_acquire(sensors);

    // L2: TORSION
    let mu_l2 = l2_torsion(&mu_l1, &sys.state.mu_t, &[0u8; STATE_DIM]);

    // L3: DISSIPATE
    let mu_l3 = l3_dissipate(&mu_l2, &sys.state.mu_t, BETA_Q64);

    // L4: BACKREACT
    let mu_l4 = l4_backreact(&mu_l3, 0);  // Curvature ≈ 0 for telemetry

    // L5: SPECTRAL
    let mu_l5 = l5_spectral(&mu_l4, &sys.state.mu_t, BETA_Q64);

    // L6: EMA
    let s_new = l6_ema(&sys.state.z_t, &sys.state.s_t, ALPHA_Q64);

    // Convert back to bytes for hashing
    let mut mu_final = [0u8; STATE_DIM];
    for (i, &m) in mu_l5.iter().enumerate() {
        mu_final[i] = ((m >> 64) as u8).min(255);
    }

    // Project observables
    let z_new = project_observable(&mu_final, &sys.w);

    // L7: HASH
    let h_new = l7_hash(&mu_final, &z_new, &s_new);

    // Apply Lie dynamics (optional)
    let mut z_evolved = z_new;
    lie_bracket_step(&mut z_evolved, &s_new, &sys.kappa, &sys.menger_mask,
                     LAMBDA_Q64, DT_Q64, 0);  // α=0 (no backreaction in telemetry mode)
    stiefel_retract(&mut sys.w);

    // Create snapshot
    let snapshot = FrameSnapshot {
        mu_t: mu_final,
        z_t: z_evolved,
        s_t: s_new,
        h_t: h_new,
        timestamp_ns: now_ns,
        menger_depth: sys.menger_depth,
    };

    sys.state = snapshot;
    sys.frame_count = sys.frame_count.wrapping_add(1);

    Ok(snapshot)
}

// =============================================================================
// TESTS (Determinism + Hardening)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determinism() {
        let sensors = [50.0; STATE_DIM];
        let mut sys1 = SystemTelemetry::new(0);
        let mut sys2 = SystemTelemetry::new(0);

        let snap1 = process_frame(&mut sys1, &sensors, 1000).unwrap();
        let snap2 = process_frame(&mut sys2, &sensors, 1000).unwrap();

        assert_eq!(snap1.h_t, snap2.h_t, "Non-deterministic hash!");
    }

    #[test]
    fn test_rate_limiting() {
        let sensors = [50.0; STATE_DIM];
        let mut sys = SystemTelemetry::new(0);

        let result1 = process_frame(&mut sys, &sensors, 0);
        assert!(result1.is_ok(), "First frame should succeed");

        let result2 = process_frame(&mut sys, &sensors, 999_999);
        assert!(result2.is_err(), "Second frame too soon should fail");

        let result3 = process_frame(&mut sys, &sensors, 2_000_000);
        assert!(result3.is_ok(), "Frame after rate limit should succeed");
    }

    #[test]
    fn test_menger_sparsity() {
        let mask = menger_mask_generate(2);
        let ones: usize = mask.iter().filter(|&&b| b).count();
        let expected = (256.0 * (20.0 / 27.0_f64).powi(2)) as usize;
        assert!((ones as i32 - expected as i32).abs() <= 2,
                "Menger mask depth 2: {} nonzeros (expected ~{})", ones, expected);
    }

    #[test]
    fn test_hash_protocol_separation() {
        let sensors = [50.0; STATE_DIM];
        let mut sys1 = SystemTelemetry::new(0);  // Menger off
        let mut sys2 = SystemTelemetry::new(2);  // Menger depth 2

        let snap1 = process_frame(&mut sys1, &sensors, 1000).unwrap();
        let snap2 = process_frame(&mut sys2, &sensors, 1000).unwrap();

        // Different Menger configs should produce different hashes
        assert_ne!(snap1.h_t, snap2.h_t, "Hash should differ with Menger config");
    }

    #[test]
    fn test_quantize_dequantize() {
        for value in [0.0, 25.0, 50.0, 75.0, 99.9] {
            let q = quantize_q64(value, 100.0);
            let dq = dequantize_q64(q, 100.0);
            assert!((dq - value).abs() < 0.01, "Quantize/dequantize mismatch");
        }
    }
}
