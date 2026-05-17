// ============================================================
// DVSM-V20 ONEFILE RUNTIME
// ------------------------------------------------------------
// Target:
//   Portable deterministic runtime for:
//   - Ally X (R=16, Q64.64)
//   - Standard PC (R=8, Q31.32)
//   - Chromebook / WASM (R=4, Q16.16)
//
// Core Concepts:
//   - Bounded Cyclic Topology
//   - Rose-Curve Constraint Layer
//   - Deterministic Telemetry
//   - Hybrid JSON + Binary Vault
//   - Portable Fixed-Point Arithmetic
//
// GROUNDED NOTES:
//   - "1.1x" is treated as symbolic restorative bias language,
//     NOT literal >100% correction.
//   - Rose geometry constrains trajectories;
//     it does NOT magically improve arithmetic precision.
//   - Q64 improves numerical precision at increased compute cost.
// ============================================================

use std::fs::File;
use std::io::{Write, Read};
use std::time::Instant;

// ============================================================
// FIXED POINT ABSTRACTION
// ============================================================

pub trait FixedPoint:
    Copy + Clone + Send + Sync
{
    fn add(self, rhs: Self) -> Self;
    fn sub(self, rhs: Self) -> Self;
    fn mul(self, rhs: Self) -> Self;
    fn from_f64(v: f64) -> Self;
    fn to_f64(self) -> f64;
}

// ------------------------------------------------------------
// Q16.16 (portable)
// ------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Q16(pub i32);

impl FixedPoint for Q16 {
    fn add(self, rhs: Self) -> Self {
        Q16(self.0.saturating_add(rhs.0))
    }

    fn sub(self, rhs: Self) -> Self {
        Q16(self.0.saturating_sub(rhs.0))
    }

    fn mul(self, rhs: Self) -> Self {
        let v = ((self.0 as i64 * rhs.0 as i64) >> 16) as i32;
        Q16(v)
    }

    fn from_f64(v: f64) -> Self {
        Q16((v * 65536.0) as i32)
    }

    fn to_f64(self) -> f64 {
        self.0 as f64 / 65536.0
    }
}

// ------------------------------------------------------------
// Q31.32
// ------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Q31(pub i64);

impl FixedPoint for Q31 {
    fn add(self, rhs: Self) -> Self {
        Q31(self.0.saturating_add(rhs.0))
    }

    fn sub(self, rhs: Self) -> Self {
        Q31(self.0.saturating_sub(rhs.0))
    }

    fn mul(self, rhs: Self) -> Self {
        let v = ((self.0 as i128 * rhs.0 as i128) >> 32) as i64;
        Q31(v)
    }

    fn from_f64(v: f64) -> Self {
        Q31((v * ((1u64 << 32) as f64)) as i64)
    }

    fn to_f64(self) -> f64 {
        self.0 as f64 / ((1u64 << 32) as f64)
    }
}

// ------------------------------------------------------------
// Q64.64
// ------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Q64(pub i128);

impl FixedPoint for Q64 {
    fn add(self, rhs: Self) -> Self {
        Q64(self.0.saturating_add(rhs.0))
    }

    fn sub(self, rhs: Self) -> Self {
        Q64(self.0.saturating_sub(rhs.0))
    }

    fn mul(self, rhs: Self) -> Self {
        // SAFE RANGE NOTE:
        // This assumes constrained operands.
        // Full 256-bit multiply would be needed
        // for unrestricted Q64 arithmetic.
        Q64((self.0.saturating_mul(rhs.0)) >> 64)
    }

    fn from_f64(v: f64) -> Self {
        Q64((v * ((1u128 << 64) as f64)) as i128)
    }

    fn to_f64(self) -> f64 {
        self.0 as f64 / ((1u128 << 64) as f64)
    }
}

// ============================================================
// POLAR CONSTRAINT LAYER
// ============================================================

pub struct PolarConstraint {
    pub k_factor: f64,
    pub bias: f64,
    pub damping: f64,
}

impl PolarConstraint {
    pub fn new(k: f64, bias: f64, damping: f64) -> Self {
        Self {
            k_factor: k,
            bias: bias.clamp(0.0, 1.0),
            damping: damping.clamp(0.0001, 1.0),
        }
    }

    /// Rose-Curve bounded recurrence
    ///
    /// r = cos(k * theta)
    ///
    /// "1.1x" concept:
    /// interpreted as restorative recurrence bias,
    /// NOT literal over-correction.
    pub fn restorative_force(
        &self,
        r_current: f64,
        theta: f64,
    ) -> f64 {
        let r_target = (self.k_factor * theta).cos();

        let error = r_target - r_current;

        (error * self.bias) * self.damping
    }
}

// ============================================================
// DVSM CORE
// ============================================================

pub struct DvsmCore<T: FixedPoint> {
    pub z: Vec<T>,
    pub theta: f64,
    pub polar: PolarConstraint,
}

impl<T: FixedPoint> DvsmCore<T> {
    pub fn new(rank: usize, polar: PolarConstraint) -> Self {
        Self {
            z: vec![T::from_f64(0.1); rank],
            theta: 0.0,
            polar,
        }
    }

    pub fn step(&mut self) {
        self.theta += 0.01;

        for i in 0..self.z.len() {
            let current = self.z[i].to_f64();

            let restore =
                self.polar
                    .restorative_force(current, self.theta);

            let next = current + restore;

            self.z[i] = T::from_f64(next);
        }
    }

    pub fn drift_norm(&self) -> f64 {
        let mut acc = 0.0;

        for z in &self.z {
            let v = z.to_f64();
            acc += v * v;
        }

        acc.sqrt()
    }
}

// ============================================================
// TELEMETRY
// ============================================================

#[repr(C, packed)]
pub struct FrameSampleV1 {
    pub frame: u32,
    pub total_us: u64,
    pub budget_pct: f32,
    pub overrun: u8,
}

impl FrameSampleV1 {
    pub fn new(
        frame: u32,
        total_us: u64,
        budget_us: u64,
    ) -> Self {
        let budget_pct =
            (total_us as f32 / budget_us as f32) * 100.0;

        Self {
            frame,
            total_us,
            budget_pct,
            overrun: (total_us > budget_us) as u8,
        }
    }

    pub fn write(
        &self,
        file: &mut File,
    ) -> std::io::Result<()> {
        file.write_all(&self.frame.to_le_bytes())?;
        file.write_all(&self.total_us.to_le_bytes())?;
        file.write_all(&self.budget_pct.to_le_bytes())?;
        file.write_all(&[self.overrun])?;
        Ok(())
    }
}

// ============================================================
// VAULT FORMAT
// ============================================================

pub struct VaultHeader {
    pub rank: u32,
    pub polar_k: f64,
    pub bias: f64,
    pub damping: f64,
    pub precision: &'static str,
}

impl VaultHeader {
    pub fn to_json(&self) -> String {
        format!(
r#"{{
  "format":"DVSM-POLAR-VAULT",
  "version":2,
  "header":{{
    "rank":{},
    "polar_k":{},
    "bias":{},
    "damping":{},
    "precision":"{}"
  }}
}}"#,
            self.rank,
            self.polar_k,
            self.bias,
            self.damping,
            self.precision
        )
    }
}

// ============================================================
// RUNTIME
// ============================================================

pub fn run_runtime<T: FixedPoint>(
    rank: usize,
    precision_name: &'static str,
    output: &str,
) {
    const FRAME_BUDGET_US: u64 = 4166;

    let polar = PolarConstraint::new(
        4.0,
        0.05,
        0.98,
    );

    let mut core =
        DvsmCore::<T>::new(rank, polar);

    let mut file =
        File::create(output).unwrap();

    // --------------------------------------------------------
    // MAGIC
    // --------------------------------------------------------

    file.write_all(b"DVSM").unwrap();

    // VERSION
    file.write_all(&2u32.to_le_bytes()).unwrap();

    // HEADER
    let header = VaultHeader {
        rank: rank as u32,
        polar_k: 4.0,
        bias: 0.05,
        damping: 0.98,
        precision: precision_name,
    };

    let json = header.to_json();

    file.write_all(
        &(json.len() as u32).to_le_bytes()
    ).unwrap();

    file.write_all(json.as_bytes()).unwrap();

    // --------------------------------------------------------
    // TELEMETRY STREAM
    // --------------------------------------------------------

    for frame in 0..1000u32 {
        let start = Instant::now();

        core.step();

        let elapsed =
            start.elapsed().as_micros() as u64;

        let sample =
            FrameSampleV1::new(
                frame,
                elapsed,
                FRAME_BUDGET_US,
            );

        sample.write(&mut file).unwrap();

        // drift guard
        let drift = core.drift_norm();

        if drift > 1000.0 {
            println!(
                "[WARN] Drift limit exceeded at frame {}",
                frame
            );
            break;
        }
    }

    // --------------------------------------------------------
    // CRC PLACEHOLDER
    // --------------------------------------------------------

    file.write_all(&0u32.to_le_bytes()).unwrap();

    println!("DVSM-V20 Runtime Complete");
    println!("Output: {}", output);
}

// ============================================================
// API ENTRY POINTS
// ============================================================

pub fn run_allyx_mode() {
    println!("Mode: Ally X");
    println!("Precision: Q64.64");
    println!("Rank: R=16");

    run_runtime::<Q64>(
        16,
        "Q64.64",
        "allyx.dvsm",
    );
}

pub fn run_standard_mode() {
    println!("Mode: Standard PC");
    println!("Precision: Q31.32");
    println!("Rank: R=8");

    run_runtime::<Q31>(
        8,
        "Q31.32",
        "standard.dvsm",
    );
}

pub fn run_compact_mode() {
    println!("Mode: Compact");
    println!("Precision: Q16.16");
    println!("Rank: R=4");

    run_runtime::<Q16>(
        4,
        "Q16.16",
        "compact.dvsm",
    );
}

// ============================================================
// MAIN
// ============================================================

fn main() {
    // Reference node:
    // Ally X / high precision

    run_allyx_mode();

    // Optional:
    // run_standard_mode();
    // run_compact_mode();
}
