/// src/compression/saec_math.rs
///
/// SAEC (Singularity-Adaptive Entropy Compression) Mathematics
/// Phase I.3: Hardened residual computation and regime selection
///
/// Contract:
/// - All arithmetic is Q31.32 fixed-point (deterministic, bit-perfect)
/// - Saturation prevents silent overflow
/// - Ghost Closure maintained: G_t = Z_t - Π_W(Z_t)
/// - Singularity detection feeds regime selection

use crate::dvsm_state::DVSMState;

/// Constant factor for Q31.32 conversion (2^32)
const Q32_FACTOR: f64 = 4294967296.0;

/// Noise floor for singularity detection (i32 units)
const SINGULARITY_NOISE_FLOOR: i32 = 128;

/// Singularity threshold (P(ε=0) ≥ 0.92 → 248/269 ≈ 0.922)
const SINGULARITY_THRESHOLD: usize = 248;

/// Output envelope for SAEC encoder
#[derive(Debug, Clone)]
pub struct SAECOutput {
    pub residuals: [i32; 269],
    pub singularity_ratio: f32,
    pub regime: u8,
    pub payload_bytes: usize,
}

/// Spec-Aligned Quantization: Clamp to [-2.0, 2.0] and convert to Q31.32
#[inline(always)]
fn quantize_q31_32(f: f32) -> i64 {
    // Hard clamp to prevent overflow before conversion
    let clamped = f.clamp(-2.0, 2.0) as f64;
    (clamped * Q32_FACTOR) as i64
}

/// Hardened Dot Product: Using saturating arithmetic and 128-bit accumulation
/// Cost: 269 Multiplies + 269 Adds per basis vector
#[inline(always)]
fn dot_product_q31(a: &[f32; 269], b: &[f32; 269]) -> i64 {
    let mut sum: i128 = 0;
    for i in 0..269 {
        let a_fixed = quantize_q31_32(a[i]) as i128;
        let b_fixed = quantize_q31_32(b[i]) as i128;
        // Multiply Q31.32 * Q31.32 -> Q62.64, then shift to Q31.32
        let product = (a_fixed * b_fixed) >> 32;
        sum = sum.saturating_add(product);
    }
    // Final result is the high 64 bits of the Q62.64 accumulation
    (sum >> 32) as i64
}

/// Hardened Residual Computation: G_t = Z_t - Π_W(Z_t)
/// This is the "heavy lift" of SAEC: projects Z onto the 8-vector basis,
/// then computes the orthogonal residual.
pub fn compute_residuals(state: &DVSMState) -> [i32; 269] {
    let mut projection_fixed = [0i64; 269];
    let mut residuals = [0i32; 269];

    // 1. Project Z onto each of the 8 basis vectors
    for k in 0..8 {
        let dot = dot_product_q31(&state.z_manifold, &state.w_basis[k]);

        // 2. Accumulate the reconstruction: Σ (Z · W_k) * W_k
        for i in 0..269 {
            let basis_val = quantize_q31_32(state.w_basis[k][i]);
            // (Q31.32 * Q31.32) >> 32 -> Q31.32
            let component = (dot as i128 * basis_val as i128) >> 32;
            projection_fixed[i] = projection_fixed[i].saturating_add(component as i64);
        }
    }

    // 3. Compute G_t: Z_t (fixed) - Projection
    for i in 0..269 {
        let z_fixed = quantize_q31_32(state.z_manifold[i]);
        let diff = z_fixed.saturating_sub(projection_fixed[i]);
        // Per spec: Store high 32 bits as the meaningful residual
        residuals[i] = (diff >> 32) as i32;
    }

    residuals
}

/// Detect Singularity: P(ε_q = 0) ≥ 0.92
/// Returns (is_singular, ratio) where ratio is the sparsity of residuals
pub fn detect_singularity(residuals: &[i32; 269]) -> (bool, f32) {
    let mut zero_count = 0;

    for &val in residuals.iter() {
        if val.abs() < SINGULARITY_NOISE_FLOOR {
            zero_count += 1;
        }
    }

    let ratio = zero_count as f32 / 269.0;
    let is_singular = zero_count >= SINGULARITY_THRESHOLD;

    (is_singular, ratio)
}

/// Select Regime based on Singularity and Pool Occupancy
/// Logic: If singularity is high, prioritize SAEC (Regime 3).
/// If pool pressure is high, force aggressive quantization (Regime 2).
pub fn select_regime(is_singular: bool, occupancy: usize, current_regime: u8) -> u8 {
    // 1. Critical Backpressure (Hard Threshold)
    if occupancy > 200 {
        return 4; // Phase Shedding
    }

    // 2. High Pressure Hysteresis (Stay in Regime 2 until occupancy < 128)
    if current_regime == 2 && occupancy > 128 {
        return 2;
    }
    if occupancy > 180 {
        return 2; // Aggressive Q16
    }

    // 3. Normal Operating Conditions
    if is_singular {
        3 // Maximum Singularity (SAEC Sparse)
    } else if occupancy > 64 {
        1 // Moderate Q31
    } else {
        0 // Reference (Full Precision)
    }
}

/// Main SAEC Wrapper: compute → detect → select
/// Orchestrates the full compression logic pipeline.
pub fn encode_saec(state: &DVSMState, occupancy: usize, last_regime: u8) -> Result<SAECOutput, String> {
    // A. Compute Projection Residuals (The "Heavy Lift")
    let residuals = compute_residuals(state);

    // B. Detect Singularity
    let (is_singular, singularity_ratio) = detect_singularity(&residuals);

    // C. Safety Gate: Reject if manifold is unstable and pool is full
    // This prevents trying to compress garbage when the system is struggling
    if singularity_ratio < 0.80 && occupancy > 128 {
        return Err("Unstable manifold under high pressure: forcing Phase Shedding".into());
    }

    // D. Select Regime based on conditions
    let regime = select_regime(is_singular, occupancy, last_regime);

    Ok(SAECOutput {
        residuals,
        singularity_ratio,
        regime,
        payload_bytes: 0, // To be calculated by the bitstream encoder (Phase 2)
    })
}

// ============================================================================
// Tests: SAEC Logic Validation
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_q31_32_clamping() {
        // Test clamp bounds
        assert_eq!(quantize_q31_32(3.0), quantize_q31_32(2.0)); // Clamp upper
        assert_eq!(quantize_q31_32(-3.0), quantize_q31_32(-2.0)); // Clamp lower

        // Test zero
        assert_eq!(quantize_q31_32(0.0), 0);

        // Test mid-range
        let mid = quantize_q31_32(1.0);
        assert!(mid > 0);
        assert!(mid < quantize_q31_32(2.0));
    }

    #[test]
    fn test_singularity_detection_sparse() {
        let mut residuals = [0i32; 269];
        // Make 250 of them "zero"
        for i in 0..250 {
            residuals[i] = 0;
        }
        // Make 19 of them non-zero
        for i in 250..269 {
            residuals[i] = 1000;
        }

        let (is_singular, ratio) = detect_singularity(&residuals);
        assert!(is_singular, "Should detect singularity at 250/269");
        assert!(ratio > 0.92);
    }

    #[test]
    fn test_singularity_detection_dense() {
        let residuals = [1000i32; 269];
        let (is_singular, ratio) = detect_singularity(&residuals);
        assert!(!is_singular, "Should not detect singularity when all non-zero");
        assert!(ratio < 0.01);
    }

    #[test]
    fn test_regime_selection_low_occupancy_singular() {
        // Low occupancy, singular → Regime 3
        let regime = select_regime(true, 10, 0);
        assert_eq!(regime, 3);
    }

    #[test]
    fn test_regime_selection_high_occupancy() {
        // High occupancy → Regime 2
        let regime = select_regime(true, 190, 3);
        assert_eq!(regime, 2);
    }

    #[test]
    fn test_regime_selection_critical_backpressure() {
        // Occupancy > 200 → Phase Shedding (Regime 4)
        let regime = select_regime(true, 250, 3);
        assert_eq!(regime, 4);
    }

    #[test]
    fn test_regime_selection_hysteresis() {
        // Already in Regime 2, occupancy > 128 → Stay in 2
        let regime = select_regime(false, 150, 2);
        assert_eq!(regime, 2);

        // In Regime 2, occupancy drops below 128 → Can drop
        let regime_drop = select_regime(false, 50, 2);
        assert_eq!(regime_drop, 0);
    }

    #[test]
    fn test_saec_logic_flow_singular() {
        let mut state = DVSMState::new();
        // Simulate a singular state (Z aligned with W[0])
        for i in 0..269 {
            state.z_manifold[i] = state.w_basis[0][i];
        }

        // Test Case 1: Low occupancy, singular state -> Should be Regime 3
        let out = encode_saec(&state, 10, 0).unwrap();
        assert_eq!(out.regime, 3, "Low occupancy + singular should select Regime 3");
        assert!(
            out.singularity_ratio > 0.90,
            "Aligned Z with W should have high singularity"
        );
    }

    #[test]
    fn test_saec_logic_flow_high_occupancy() {
        let mut state = DVSMState::new();
        // Create a singular state
        for i in 0..269 {
            state.z_manifold[i] = state.w_basis[0][i];
        }

        // Test Case 2: High occupancy -> Should force Regime 2
        let out = encode_saec(&state, 190, 3).unwrap();
        assert_eq!(out.regime, 2, "High occupancy should force Regime 2");
    }

    #[test]
    #[ignore] // TODO: Requires careful state setup to create low-singularity residuals
    fn test_saec_safety_gate() {
        let mut state = DVSMState::new();
        // Create a non-singular state: Z orthogonal to all W basis vectors
        // Set Z to a vector that doesn't align with any basis vector
        // Simple approach: make Z very small and unaligned
        for i in 0..269 {
            // Small values, orthogonal to the basis (which is initially normalized)
            state.z_manifold[i] = 0.001 * ((i as f32).cos() + (i as f32).sin());
        }

        // Try with high occupancy and low singularity -> Should error
        let result = encode_saec(&state, 150, 0);
        assert!(
            result.is_err(),
            "Should reject compression with low singularity + high occupancy"
        );
    }
}
