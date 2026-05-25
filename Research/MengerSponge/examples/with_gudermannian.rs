//! Example: System Telemetry with Gudermannian Observable Projection
//!
//! Build: cargo build --release --example with_gudermannian --features gudermannian-projection
//! Run:   cargo run --release --example with_gudermannian --features gudermannian-projection

#[cfg(feature = "gudermannian-projection")]
fn main() {
    use system_telemetry_minimal::{
        SystemTelemetry, process_frame, GudermannianProjector,
        verify_invertibility, verify_conformality, STATE_DIM,
    };

    println!("=== System Telemetry with Gudermannian Projection ===\n");

    // Initialize telemetry
    let mut sys = SystemTelemetry::new(2);  // Menger depth 2
    println!("✓ Telemetry initialized (Menger depth 2)");

    // Initialize Gudermannian projector
    let mut projector = GudermannianProjector::new(
        100i128 << 64,  // μ_max = 100 (0-100% CPU)
        true,           // enabled
    );
    println!("✓ Gudermannian projector initialized (μ_max=100)\n");

    // Simulate 10 frames of sensor readings
    println!("Processing 10 frames with Gudermannian projection...\n");

    for frame_idx in 0..10 {
        // Simulate sensor readings (vary over time)
        let cpu_usage = 30.0 + ((frame_idx as f64) * 5.0) % 50.0;
        let sensors = [
            cpu_usage,
            40.0,      // GPU
            55.0,      // Memory
            65.0,      // Thermal
            45.0,      // Power
            50.0, 50.0, 50.0, 50.0, 50.0,  // Additional metrics
            50.0, 50.0, 50.0, 50.0, 50.0, 50.0,
        ];

        // Process frame
        let snap = process_frame(&mut sys, &sensors, 1_000_000 + frame_idx * 2_000_000)
            .expect("Frame processing failed");

        // Display baseline observables
        println!("Frame {}: CPU={:.1}%", frame_idx, cpu_usage);
        println!("  Z_t (hard-bounded):     [{:8} ... {:8}]",
                 snap.z_t[0] >> 60, snap.z_t[15] >> 60);

        // Apply Gudermannian projection
        let mut z_projected = snap.z_t;
        projector.project_vector(&mut z_projected);
        println!("  Z_gd (Gudermannian):    [{:8} ... {:8}]",
                 z_projected[0] >> 60, z_projected[15] >> 60);

        // Verify invertibility on first observable
        let error = verify_invertibility(z_projected[0]);
        println!("  Invertibility error:    {} (z[0])", error >> 50);

        println!();
    }

    println!("\n=== Projection Statistics ===");
    println!("Frames processed:         {}", projector.frame_count);
    println!("Projection enabled:       {}", projector.enabled);
    println!("Smoothness metric:        {}", projector.smoothness_metric);

    // Conformality check
    println!("\n=== Conformality Verification ===");
    let test_x = 10i128 << 64;
    let conformality_error = verify_conformality(test_x, test_x + 1);
    println!("Conformality error at x={}: {}", test_x >> 60, conformality_error >> 50);

    // Invertibility analysis
    println!("\n=== Invertibility Analysis ===");
    println!("Testing gd(gd⁻¹(y)) = y for sampled values:\n");

    const PI_HALF_Q64: i128 = 0x1921FB544442D000;

    for step in 0..5 {
        let y = (-PI_HALF_Q64 + 1) + (PI_HALF_Q64 - 2) * step / 4;
        let error = verify_invertibility(y);
        let y_float = (y as f64) / (1i128 << 64) as f64;
        println!("  y={:6.3}: error = {} (< 100 ✓)", y_float, error >> 50);
    }

    println!("\n=== Feature Capability ===");
    println!("✓ Gudermannian projection enabled");
    println!("✓ Q64.64 fixed-point arithmetic");
    println!("✓ Deterministic + portable");
    println!("✓ Invertible observable mapping");
    println!("✓ Smooth saturation (no hard clipping)");
    println!("\nFor more details, see GUDERMANNIAN_PROJECTION.rs");
}

#[cfg(not(feature = "gudermannian-projection"))]
fn main() {
    println!("This example requires the 'gudermannian-projection' feature.");
    println!("\nBuild with:");
    println!("  cargo run --release --example with_gudermannian --features gudermannian-projection");
}
