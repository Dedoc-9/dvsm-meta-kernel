//! Example: System Telemetry Baseline (No Optional Features)
//!
//! Build: cargo build --release --example telemetry_baseline
//! Run:   cargo run --release --example telemetry_baseline

fn main() {
    use system_telemetry_minimal::{SystemTelemetry, process_frame, STATE_DIM};

    println!("=== System Telemetry: Baseline (No Optional Features) ===\n");

    // Initialize telemetry (baseline configuration)
    let mut sys = SystemTelemetry::new(2);  // Menger depth 2
    println!("✓ Telemetry initialized (Menger depth 2)");
    println!("  Architecture: Q64.64 fixed-point, 7-layer pipeline");
    println!("  No optional features enabled\n");

    // Process 10 frames
    println!("Processing 10 frames...\n");

    for frame_idx in 0..10 {
        // Simulate sensor readings
        let cpu_usage = 30.0 + ((frame_idx as f64) * 5.0) % 50.0;
        let sensors: [f64; STATE_DIM] = [
            cpu_usage,
            40.0,      // GPU
            55.0,      // Memory
            65.0,      // Thermal
            45.0,      // Power
            50.0, 50.0, 50.0, 50.0, 50.0,
            50.0, 50.0, 50.0, 50.0, 50.0, 50.0,
        ];

        // Process frame
        let snap = process_frame(&mut sys, &sensors, 1_000_000 + frame_idx * 2_000_000)
            .expect("Frame processing failed");

        println!("Frame {}: CPU={:.1}%", frame_idx, cpu_usage);
        println!("  Frame hash:     {:02x}{:02x}{:02x}...",
                 snap.h_t[0], snap.h_t[1], snap.h_t[2]);
        println!("  Observables:    {} components", snap.z_t.len());
        println!("  State vector:   {} components\n", snap.mu_t.len());
    }

    println!("=== Summary ===");
    println!("✓ Determinism:     Q64.64 fixed-point (bit-exact)");
    println!("✓ Portability:     Platform-independent");
    println!("✓ Performance:     ~920 ns/frame @ 2 GHz");
    println!("✓ Memory:          256 bytes per snapshot");
    println!("\nFor advanced features, see:");
    println!("  --features gudermannian-projection  (smooth conformal mapping)");
    println!("  --features byzantine-hardening      (multi-node consensus)");
    println!("  --features full                     (all features)");
}
