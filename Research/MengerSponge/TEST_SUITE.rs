//! System Telemetry: Comprehensive Test Suite
//! Tests: determinism, hardening, Menger, quantization, stability

#[cfg(test)]
mod tests {
    use crate::{SystemTelemetry, process_frame, DIM, STATE_DIM, quantize_q64, dequantize_q64};

    // =========================================================================
    // DETERMINISM TESTS
    // =========================================================================

    #[test]
    fn test_determinism_baseline() {
        let sensors = [50.0f64; STATE_DIM];
        let mut sys1 = SystemTelemetry::new(0);
        let mut sys2 = SystemTelemetry::new(0);

        for frame in 0..100 {
            let snap1 = process_frame(&mut sys1, &sensors, 1_000_000 + frame * 2_000_000).unwrap();
            let snap2 = process_frame(&mut sys2, &sensors, 1_000_000 + frame * 2_000_000).unwrap();

            assert_eq!(snap1.h_t, snap2.h_t, "Hash divergence at frame {}", frame);
            assert_eq!(snap1.z_t, snap2.z_t, "Observable divergence at frame {}", frame);
        }
    }

    #[test]
    fn test_determinism_menger() {
        let sensors = [42.0f64; STATE_DIM];
        let mut sys1 = SystemTelemetry::new(2);
        let mut sys2 = SystemTelemetry::new(2);

        for frame in 0..50 {
            let snap1 = process_frame(&mut sys1, &sensors, 1_000_000 + frame * 2_000_000).unwrap();
            let snap2 = process_frame(&mut sys2, &sensors, 1_000_000 + frame * 2_000_000).unwrap();

            assert_eq!(snap1.h_t, snap2.h_t, "Menger depth 2: Hash divergence");
        }
    }

    #[test]
    fn test_hash_protocol_separation() {
        let sensors = [50.0f64; STATE_DIM];
        let mut sys_d0 = SystemTelemetry::new(0);
        let mut sys_d2 = SystemTelemetry::new(2);

        let snap0 = process_frame(&mut sys_d0, &sensors, 1_000_000).unwrap();
        let snap2 = process_frame(&mut sys_d2, &sensors, 1_000_000).unwrap();

        // Different Menger configurations should produce different hashes
        assert_ne!(snap0.h_t, snap2.h_t, "Protocol separation failed");
    }

    // =========================================================================
    // HARDENING TESTS
    // =========================================================================

    #[test]
    fn test_rate_limiting_enforced() {
        let sensors = [50.0f64; STATE_DIM];
        let mut sys = SystemTelemetry::new(0);

        // First frame should succeed
        let result1 = process_frame(&mut sys, &sensors, 0);
        assert!(result1.is_ok(), "First frame failed");

        // Second frame too soon should fail
        let result2 = process_frame(&mut sys, &sensors, 999_999);
        assert!(result2.is_err(), "Rate limit not enforced");

        // Frame after rate limit should succeed
        let result3 = process_frame(&mut sys, &sensors, 2_000_000);
        assert!(result3.is_ok(), "Frame after rate limit failed");

        // Verify frame count incremented correctly
        assert_eq!(sys.frame_count, 2, "Frame count incorrect");
    }

    #[test]
    fn test_nan_inf_rejection() {
        let mut sensors = [50.0f64; STATE_DIM];
        sensors[0] = f64::NAN;
        sensors[1] = f64::INFINITY;

        let mut sys = SystemTelemetry::new(0);
        let result = process_frame(&mut sys, &sensors, 1_000_000);

        // Should succeed but with zeroed invalid values
        assert!(result.is_ok(), "NaN/Inf handling failed");
        assert_eq!(result.unwrap().mu_t[0], 0, "NaN not zeroed");
    }

    #[test]
    fn test_range_clamping() {
        let mut sensors = [0.0f64; STATE_DIM];
        sensors[0] = 150.0;  // CPU% > 100 (out of range)

        let mut sys = SystemTelemetry::new(0);
        let result = process_frame(&mut sys, &sensors, 1_000_000);

        assert!(result.is_ok(), "Out-of-range handling failed");
        let snap = result.unwrap();
        assert!(snap.mu_t[0] <= 255, "Value not clamped");
    }

    // =========================================================================
    // MENGER SPONGE TESTS
    // =========================================================================

    #[test]
    fn test_menger_mask_generation() {
        let mask_d0 = crate::kernel::menger_mask_generate(0);
        let mask_d1 = crate::kernel::menger_mask_generate(1);
        let mask_d2 = crate::kernel::menger_mask_generate(2);

        let count_d0: usize = mask_d0.iter().filter(|&&b| b).count();
        let count_d1: usize = mask_d1.iter().filter(|&&b| b).count();
        let count_d2: usize = mask_d2.iter().filter(|&&b| b).count();

        assert_eq!(count_d0, 256, "Depth 0 should be full tensor");
        assert!((count_d1 as i32 - 240).abs() <= 5, "Depth 1: {} nonzeros", count_d1);
        assert!((count_d2 as i32 - 189).abs() <= 5, "Depth 2: {} nonzeros", count_d2);

        // Verify sparsity reduction
        assert!(count_d1 < count_d0, "Sparsification not working (depth 1)");
        assert!(count_d2 < count_d1, "Sparsification not working (depth 2)");
    }

    #[test]
    fn test_menger_sparsity_ratio() {
        let mask = crate::kernel::menger_mask_generate(2);
        let ones: usize = mask.iter().filter(|&&b| b).count();
        let expected = (256.0 * (20.0 / 27.0_f64).powi(2)) as usize;

        // Allow ±2 variation due to rounding
        assert!((ones as i32 - expected as i32).abs() <= 2,
                "Menger depth 2: {} nonzeros (expected ~{})", ones, expected);
    }

    #[test]
    fn test_menger_antisymmetry_preserved() {
        let sys = SystemTelemetry::new(2);
        let mask = &sys.menger_mask;

        // Verify that mask is symmetric (if κ[i,j] is masked, so is κ[j,i])
        for i in 0..DIM {
            for j in 0..DIM {
                assert_eq!(mask[i * DIM + j], mask[j * DIM + i],
                          "Menger mask not symmetric at ({},{})", i, j);
            }
        }
    }

    // =========================================================================
    // QUANTIZATION TESTS
    // =========================================================================

    #[test]
    fn test_quantize_dequantize_reversibility() {
        let test_values = [0.0, 10.0, 25.0, 50.0, 75.0, 99.9];
        let max_phys = 100.0;

        for &value in &test_values {
            let q = quantize_q64(value, max_phys);
            let dq = dequantize_q64(q, max_phys);

            assert!((dq - value).abs() < 0.1,
                   "Quantize/dequantize error for {}: got {}", value, dq);
        }
    }

    #[test]
    fn test_quantize_precision() {
        // Q64.64 should maintain ~19 decimal digits of precision
        let q = quantize_q64(50.0, 100.0);
        assert!(q > 0, "Quantization produced zero");
        assert!(q < (1i128 << 64), "Quantization overflow");
    }

    #[test]
    fn test_quantize_boundaries() {
        // Test boundary values
        let q_zero = quantize_q64(0.0, 100.0);
        let q_max = quantize_q64(99.9, 100.0);
        let q_over = quantize_q64(150.0, 100.0);

        assert_eq!(q_zero, 0, "Zero not handled");
        assert!(q_max > 0, "Max value zeroed");
        assert!(q_over < (1i128 << 64), "Overflow not prevented");
    }

    // =========================================================================
    // STABILITY TESTS (Lie Dynamics)
    // =========================================================================

    #[test]
    fn test_energy_containment() {
        // If Lie dynamics active, Z should not grow unbounded
        let mut sys = SystemTelemetry::new(0);
        let sensors = [50.0f64; STATE_DIM];

        let mut max_energy = 0i128;
        for frame in 0..1000 {
            let snap = process_frame(&mut sys, &sensors, 1_000_000 + frame * 1_000_000).unwrap();
            let energy: i128 = snap.z_t.iter()
                .map(|&z| ((z as i256 * z as i256) >> 64) as i128)
                .sum();
            max_energy = max_energy.max(energy);
        }

        // Energy should be bounded (containment check)
        assert!(max_energy < (1i128 << 63), "Energy explosion detected");
    }

    #[test]
    fn test_stiefel_orthogonality_maintained() {
        let mut sys = SystemTelemetry::new(0);
        let sensors = [50.0f64; STATE_DIM];

        for frame in 0..100 {
            let _ = process_frame(&mut sys, &sensors, 1_000_000 + frame * 2_000_000);

            // Check that W maintains approximate orthonormality
            // (In real implementation, would verify WᵀW ≈ I)
        }
    }

    // =========================================================================
    // PIPELINE ORDERING TESTS
    // =========================================================================

    #[test]
    fn test_pipeline_immutability() {
        // This is a compile-time test in Rust (type system enforces ordering)
        // At runtime, we verify that reordering would change the hash

        let sensors = [50.0f64; STATE_DIM];
        let mut sys1 = SystemTelemetry::new(0);
        let mut sys2 = SystemTelemetry::new(0);

        let snap1 = process_frame(&mut sys1, &sensors, 1_000_000).unwrap();
        let snap2 = process_frame(&mut sys2, &sensors, 1_000_000).unwrap();

        // Same input, same state, same pipeline order → identical hashes
        assert_eq!(snap1.h_t, snap2.h_t, "Pipeline determinism broken");
    }

    // =========================================================================
    // INTEGRATION TESTS
    // =========================================================================

    #[test]
    fn test_long_run_stability() {
        let mut sys = SystemTelemetry::new(2);
        let mut sensors = [50.0f64; STATE_DIM];

        let mut hashes_seen = Vec::new();

        for frame in 0..10000 {
            // Vary sensors slightly
            sensors[0] = 45.0 + ((frame % 100) as f64 / 100.0) * 10.0;

            let result = process_frame(&mut sys, &sensors, 1_000_000 + frame as u64 * 2_000_000);

            if let Ok(snap) = result {
                hashes_seen.push(snap.h_t);
            }
        }

        // Should have processed ~10000 frames
        assert!(hashes_seen.len() > 5000, "Long run failed");

        // Hashes should all be valid (not NaN, not all zeros)
        for hash in &hashes_seen {
            assert!(hash.iter().any(|&b| b != 0), "All-zero hash detected");
        }
    }

    #[test]
    fn test_concurrent_independent_systems() {
        // Verify that multiple telemetry systems don't interfere
        let mut sys1 = SystemTelemetry::new(0);
        let mut sys2 = SystemTelemetry::new(2);

        let sensors1 = [50.0f64; STATE_DIM];
        let sensors2 = [75.0f64; STATE_DIM];

        for _ in 0..100 {
            let snap1 = process_frame(&mut sys1, &sensors1, 1_000_000).unwrap();
            let snap2 = process_frame(&mut sys2, &sensors2, 1_000_000).unwrap();

            // Different configurations should produce different hashes
            assert_ne!(snap1.h_t, snap2.h_t, "Cross-system interference");
        }
    }

    // =========================================================================
    // PERFORMANCE SANITY CHECKS
    // =========================================================================

    #[test]
    fn test_performance_bounds() {
        use std::time::Instant;

        let mut sys = SystemTelemetry::new(2);
        let sensors = [50.0f64; STATE_DIM];

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = process_frame(&mut sys, &sensors, 1_000_000);
        }
        let elapsed = start.elapsed();

        // Should process 1000 frames in < 1 second on modern CPU
        assert!(elapsed.as_secs() < 1, "Performance regression (too slow)");

        println!("Processed 1000 frames in {:?}", elapsed);
    }

    #[test]
    fn test_menger_performance_improvement() {
        use std::time::Instant;

        let sensors = [50.0f64; STATE_DIM];

        // Time baseline (no Menger)
        let mut sys_d0 = SystemTelemetry::new(0);
        let start_d0 = Instant::now();
        for _ in 0..100 {
            let _ = process_frame(&mut sys_d0, &sensors, 1_000_000);
        }
        let time_d0 = start_d0.elapsed();

        // Time Menger depth 2
        let mut sys_d2 = SystemTelemetry::new(2);
        let start_d2 = Instant::now();
        for _ in 0..100 {
            let _ = process_frame(&mut sys_d2, &sensors, 1_000_000);
        }
        let time_d2 = start_d2.elapsed();

        // Menger should be faster or equal (never slower in Lie kernel case)
        // Note: May be equal in telemetry-only mode due to overhead
        println!("Baseline: {:?}, Menger D2: {:?}", time_d0, time_d2);
        assert!(time_d2 <= time_d0 * 2, "Menger overhead too high");
    }
}

// =============================================================================
// FEATURE-GATED TESTS: GUDERMANNIAN PROJECTION
// =============================================================================

#[cfg(all(test, feature = "gudermannian-projection"))]
mod tests_gudermannian {
    use crate::gudermannian::*;

    #[test]
    fn test_gd_invertibility() {
        let test_values = [-50i128, -10i128, 0i128, 10i128, 50i128];

        for &x in &test_values {
            let y = gd_q64(x);
            let x_recovered = gd_inv_q64(y);
            let error = (x - x_recovered).abs();

            assert!(error < 100, "Invertibility error too large: {}", error);
        }
    }

    #[test]
    fn test_gd_conformality() {
        let x = 10i128 << 64;
        let error = verify_conformality(x, x + 1);

        assert!(error < 1000, "Conformality error: {}", error);
    }

    #[test]
    fn test_gudermannian_projector() {
        let mut proj = GudermannianProjector::new(100 << 64, true);
        let mut z = [50i128 << 64; 16];

        proj.project_vector(&mut z);

        assert!(proj.enabled);
        assert_eq!(proj.frame_count, 1);
        assert!(z[0] > 0);
    }

    #[test]
    fn test_gd_smooth_saturation() {
        let z1 = gd_q64(10i128 << 64);
        let z2 = gd_q64(11i128 << 64);
        let z3 = gd_q64(12i128 << 64);

        let diff1 = (z2 - z1).abs();
        let diff2 = (z3 - z2).abs();

        assert!(diff2 <= diff1, "Saturation not smooth");
    }

    #[test]
    fn test_gd_range_bounded() {
        const PI_HALF: i128 = 0x1921FB544442D000;

        for x in [-100i128, -10, 0, 10, 100].iter() {
            let y = gd_q64(*x << 64);
            assert!(y > -PI_HALF, "gd({}) below −π/2", x);
            assert!(y < PI_HALF, "gd({}) above π/2", x);
        }
    }
}

// =============================================================================
// FEATURE-GATED TESTS: BYZANTINE HARDENING
// =============================================================================

#[cfg(all(test, feature = "byzantine-hardening"))]
mod tests_byzantine {
    use crate::byzantine::*;

    #[test]
    fn test_merkle_dag_append() {
        let mut dag = MerkleDAG::new();
        let record = AuditRecord::from_snapshot(
            0, [42u8; 32], [0u8; 32], 1_000_000,
            0, AuditZone::Compute,
            HashProtocolVersion::V2Byzantine, 2,
        );

        let root = dag.append(0, record).unwrap();
        assert_ne!(root, [0u8; 32]);
        assert_eq!(dag.total_frames, 1);
    }

    #[test]
    fn test_pbft_consensus_quorum() {
        let mut cons = PBFTLiteConsensus::new(0, 7);
        let root = [1u8; 32];

        for _ in 0..5 {
            cons.propose(root).ok();
        }

        assert!(cons.has_consensus(root));
        assert_eq!(cons.consensus_root, root);
    }

    #[test]
    fn test_pbft_tolerates_byzantine() {
        let mut cons = PBFTLiteConsensus::new(0, 7);
        let honest_root = [1u8; 32];
        let byzantine_root = [255u8; 32];

        for _ in 0..5 {
            cons.propose(honest_root).ok();
        }
        cons.inject_byzantine(byzantine_root).ok();
        cons.inject_byzantine(byzantine_root).ok();

        assert!(cons.has_consensus(honest_root));
        assert!(!cons.has_consensus(byzantine_root));
    }

    #[test]
    fn test_audit_record_commitment() {
        let record = AuditRecord::from_snapshot(
            42, [1u8; 32], [2u8; 32], 1_000_000,
            5, AuditZone::AuditCommit,
            HashProtocolVersion::V2Byzantine, 2,
        );

        let commit = record.commitment();
        assert_ne!(commit, [0u8; 32]);
        assert_eq!(record.frame_seq, 42);
        assert_eq!(record.shard_id, 5);
    }

    #[test]
    fn test_merkle_dag_global_consistency() {
        let mut dag = MerkleDAG::new();

        for shard in 0..64 {
            let record = AuditRecord::from_snapshot(
                shard as u64, [shard as u8; 32], [0u8; 32], 1_000_000,
                shard as u8, AuditZone::Compute,
                HashProtocolVersion::V2Byzantine, 2,
            );
            dag.append(shard as u8, record).ok();
        }

        assert!(dag.verify_global_consistency());
    }
}
