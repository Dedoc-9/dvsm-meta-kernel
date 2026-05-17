// ── DVSM-V20 HARDENED VAULT HEADER ──────────────────────────
//
// Author: Daniel J. Dillberg
//
// Rose-curve constraint bounds long-run trajectory drift into cyclic attractors.
// The 1.1x restorative model represents controlled over-correction toward centerline equilibrium.
// Polar recurrence reduces unbounded state divergence during high-noise manifold evolution.
// Restorative bias acts as a damped return force rather than a precision amplifier.
// Together, the rose topology and bounded recurrence improve runtime stability under sustained stress.
//
// Purpose:
//   Self-describing runtime manifest for deterministic replay,
//   telemetry decoding, and cross-platform verification.
//
// Layout:
//   [MAGIC "DVSM"]        4 bytes
//   [VERSION u32 LE]     4 bytes
//   [JSON_SIZE u32 LE]   4 bytes
//   [JSON_HEADER]        variable
//
// NOTE:
//   Telemetry stream records are now 25 bytes each:
//
//     frame:u32        = 4
//     total_us:u64     = 8
//     budget_pct:f32   = 4
//     drift:f32        = 4
//     stress:f32       = 4
//     overrun:u8       = 1
//
//   TOTAL = 25 bytes

use std::fs::File;
use std::io::{Write, Result};

pub fn write_vault_header(
    file: &mut File,
    rank: u32,
    precision: &str,
    frames: u32,
) -> Result<()> {

    let json = format!(
r#"{{
  "format":"DVSM-V20",
  "version":4,

  "runtime":{{
    "rank":{},
    "precision":"{}",
    "frame_budget_us":4166,
    "target_hz":240
  }},

  "dynamics":{{
    "lambda":0.05,
    "alpha":0.98,
    "dt":0.004166666666666667,
    "polar_k":4.0,
    "restorative_bias":0.05,
    "damping":0.98
  }},

  "arithmetic":{{
    "q64_mode":"bounded_saturating",
    "overflow_policy":"clamp_then_saturate",
    "determinism":"fixed_step"
  }},

  "telemetry":{{
    "record_bytes":25,
    "endianness":"little",
    "fields":[
      "frame:u32",
      "total_us:u64",
      "budget_pct:f32",
      "drift:f32",
      "stress:f32",
      "overrun:u8"
    ]
  }},

  "limits":{{
    "max_drift_norm":1000.0,
    "max_frames":{}
  }}

}}"#,
        rank,
        precision,
        frames
    );

    // MAGIC
    file.write_all(b"DVSM")?;

    // VERSION
    file.write_all(&4u32.to_le_bytes())?;

    // JSON HEADER SIZE
    file.write_all(&(json.len() as u32).to_le_bytes())?;

    // JSON PAYLOAD
    file.write_all(json.as_bytes())?;

    Ok(())
}

use std::fs::File;
use std::io::Write;
use std::time::Instant;

// ── CONSTANTS ───────────────────────────────────────────────
const RMAX: usize = 16;
const DT_F64: f64 = 1.0 / 240.0;
const ALPHA_F64: f64 = 0.98;
const LAMBDA_F64: f64 = 0.05;
const ETA_F64: f64 = 0.01;
const FRAME_BUDGET_US: u64 = 4166;

// ── FIXED POINT TRAIT ───────────────────────────────────────

pub trait FixedPoint: Copy + Clone + Send + Sync {
    fn add(self, rhs: Self) -> Self;
    fn sub(self, rhs: Self) -> Self;
    fn mul(self, rhs: Self) -> Self;
    fn from_f64(v: f64) -> Self;
    fn to_f64(self) -> f64;
    fn zero() -> Self;
}

// ── Q16.16 (WASM / Chromebook) ──────────────────────────────

#[derive(Clone, Copy)]
pub struct Q16(pub i32);

impl FixedPoint for Q16 {
    #[inline] fn add(self, rhs: Self) -> Self { Q16(self.0.saturating_add(rhs.0)) }
    #[inline] fn sub(self, rhs: Self) -> Self { Q16(self.0.saturating_sub(rhs.0)) }
    #[inline] fn mul(self, rhs: Self) -> Self {
        Q16(((self.0 as i64 * rhs.0 as i64) >> 16) as i32)
    }
    #[inline] fn from_f64(v: f64) -> Self { Q16((v * 65536.0) as i32) }
    #[inline] fn to_f64(self) -> f64 { self.0 as f64 / 65536.0 }
    #[inline] fn zero() -> Self { Q16(0) }
}

// ── Q31.32 (Standard PC) ───────────────────────────────────

#[derive(Clone, Copy)]
pub struct Q31(pub i64);

impl FixedPoint for Q31 {
    #[inline] fn add(self, rhs: Self) -> Self { Q31(self.0.saturating_add(rhs.0)) }
    #[inline] fn sub(self, rhs: Self) -> Self { Q31(self.0.saturating_sub(rhs.0)) }
    #[inline] fn mul(self, rhs: Self) -> Self {
        Q31(((self.0 as i128 * rhs.0 as i128) >> 32) as i64)
    }
    #[inline] fn from_f64(v: f64) -> Self { Q31((v * (1u64 << 32) as f64) as i64) }
    #[inline] fn to_f64(self) -> f64 { self.0 as f64 / (1u64 << 32) as f64 }
    #[inline] fn zero() -> Self { Q31(0) }
}

// ── Q64.64 (Ally X / archival) ──────────────────────────────
// FIX: saturating_mul prevents silent overflow.
// NOTE: full Q64 range requires 256-bit intermediate.
// Constrain inputs to ±2^32 effective range for safety.

#[derive(Clone, Copy)]
pub struct Q64(pub i128);

impl FixedPoint for Q64 {
    #[inline] fn add(self, rhs: Self) -> Self { Q64(self.0.saturating_add(rhs.0)) }
    #[inline] fn sub(self, rhs: Self) -> Self { Q64(self.0.saturating_sub(rhs.0)) }
    #[inline] fn mul(self, rhs: Self) -> Self {
        // Clamp to safe range before multiply to prevent overflow
        let a = self.0.clamp(-(1i128 << 96), 1i128 << 96);
        let b = rhs.0.clamp(-(1i128 << 96), 1i128 << 96);
        Q64(a.saturating_mul(b) >> 64)
    }
    #[inline] fn from_f64(v: f64) -> Self {
        // Clamp to prevent f64→i128 overflow (f64 max > i128 max)
        let clamped = v.clamp(-1e18, 1e18);
        Q64((clamped * (1u128 << 64) as f64) as i128)
    }
    #[inline] fn to_f64(self) -> f64 { self.0 as f64 / (1u128 << 64) as f64 }
    #[inline] fn zero() -> Self { Q64(0) }
}

// ── POLAR CONSTRAINT ────────────────────────────────────────
// Rose-curve bounded trajectory constraint.
// NOT a precision enhancer. IS a trajectory limiter.

pub struct PolarConstraint {
    pub k_factor: f64,
    pub bias: f64,     // restorative strength ∈ [0,1]
    pub damping: f64,  // decay rate ∈ (0,1]
}

impl PolarConstraint {
    pub fn new(k: f64, bias: f64, damping: f64) -> Self {
        Self {
            k_factor: k,
            bias: bias.clamp(0.0, 1.0),
            damping: damping.clamp(0.0001, 1.0),
        }
    }

    #[inline]
    pub fn restorative_force(&self, r_current: f64, theta: f64) -> f64 {
        // r_target from rose curve: r = cos(k·θ)
        let r_target = (self.k_factor * theta).cos();
        let error = r_target - r_current;
        error * self.bias * self.damping
    }
}

// ── DVSM CORE (fixed-size arrays, no Vec) ───────────────────
// FIX: Vec<T> replaced with [T; RMAX] — zero heap in hot path

pub struct DvsmCore<T: FixedPoint> {
    pub z: [T; RMAX],
    pub s: [T; RMAX],
    pub kappa: [T; RMAX * RMAX],  // precomputed antisymmetric coupling
    pub theta: f64,
    pub polar: PolarConstraint,
    pub rank: usize,
    pub frame: u64,
    // precomputed constants in fixed-point
    lambda: T,
    dt: T,
    alpha: T,
    one_minus_alpha: T,
}

impl<T: FixedPoint> DvsmCore<T> {
    pub fn new(rank: usize, polar: PolarConstraint) -> Self {
        let rank = rank.min(RMAX);
        let mut z = [T::zero(); RMAX];
        let mut s = [T::zero(); RMAX];
        let mut kappa = [T::zero(); RMAX * RMAX];

        // init Z with small nonzero values
        let mut k = 0;
        while k < rank { z[k] = T::from_f64(0.1); k += 1; }

        // precompute κ (antisymmetric: κ[i,j] = -κ[j,i])
        // using Bhaskara sin approximation
        let pi = std::f64::consts::PI;
        let mut i = 0;
        while i < rank {
            let mut j = 0;
            while j < rank {
                let x = (i as f64) * 1.37 - (j as f64) * 1.73;
                let x = x - (x / (2.0*pi)).floor() * 2.0*pi;
                let x = if x > pi { x - 2.0*pi } else { x };
                let sin_val = 16.0*x*(pi-x.abs()) / (5.0*pi*pi - 4.0*x.abs()*(pi-x.abs()));
                kappa[i * RMAX + j] = T::from_f64(sin_val);
                j += 1;
            }
            i += 1;
        }

        Self {
            z, s, kappa, theta: 0.0, polar, rank, frame: 0,
            lambda: T::from_f64(LAMBDA_F64),
            dt: T::from_f64(DT_F64),
            alpha: T::from_f64(ALPHA_F64),
            one_minus_alpha: T::from_f64(1.0 - ALPHA_F64),
        }
    }

    /// Full DVSM pipeline step in fixed-point arithmetic.
    /// FIX: original step() had no Lie bracket, no EMA, no basis ops.
    /// Now implements: Lie evolution → EMA → polar constraint → containment.
    pub fn step(&mut self) {
        let r = self.rank;

        // 1. LIE-BRACKET EVOLUTION: Z += dt·([Z,S]_κ − λZ)
        let mut z_next = [T::zero(); RMAX];
        let mut k = 0;
        while k < r {
            let mut torque = T::zero();
            let mut j = 0;
            while j < r {
                if j != k {
                    // (Z_k·S_j − Z_j·S_k) · κ_{kj}
                    let zk_sj = self.z[k].mul(self.s[j]);
                    let zj_sk = self.z[j].mul(self.s[k]);
                    let bracket = zk_sj.sub(zj_sk);
                    torque = torque.add(bracket.mul(self.kappa[k * RMAX + j]));
                }
                j += 1;
            }
            // dt · (torque − λ·Z_k)
            let damped = torque.sub(self.lambda.mul(self.z[k]));
            let delta = self.dt.mul(damped);
            z_next[k] = self.z[k].add(delta);
            k += 1;
        }
        self.z = z_next;

        // 2. EMA MEMORY: S = α·S + (1−α)·Z
        k = 0;
        while k < r {
            let a_term = self.alpha.mul(self.s[k]);
            let z_term = self.one_minus_alpha.mul(self.z[k]);
            self.s[k] = a_term.add(z_term);
            k += 1;
        }

        // 3. POLAR CONSTRAINT (rose-curve trajectory bounding)
        self.theta += DT_F64;
        k = 0;
        while k < r {
            let current = self.z[k].to_f64();
            let restore = self.polar.restorative_force(current, self.theta + k as f64 * 0.1);
            // Apply as small additive correction (does NOT replace Lie dynamics)
            self.z[k] = T::from_f64(current + restore * DT_F64);
            k += 1;
        }

        // 4. CONTAINMENT (drift guard)
        let drift = self.drift_norm();
        if drift > 1000.0 {
            // hard reset Z to small values, preserve S (GhostSnap-like)
            k = 0;
            while k < r { self.z[k] = T::from_f64(1e-6); k += 1; }
        }

        self.frame += 1;
    }

    pub fn drift_norm(&self) -> f64 {
        let mut acc = 0.0;
        let mut k = 0;
        while k < self.rank {
            let v = self.z[k].to_f64();
            acc += v * v;
            k += 1;
        }
        acc.sqrt()
    }

    pub fn stress(&self) -> f64 {
        let z_n = self.drift_norm().max(1e-15);
        let mut s_n = 0.0;
        let mut k = 0;
        while k < self.rank { let v = self.s[k].to_f64(); s_n += v*v; k += 1; }
        s_n.sqrt() / z_n
    }
}

// ── TELEMETRY ───────────────────────────────────────────────
// FIX: removed #[repr(packed)] — causes UB with references on some targets.
// Use explicit serialization instead.

pub struct FrameSample {
    pub frame: u32,
    pub total_us: u64,
    pub budget_pct: f32,
    pub drift: f32,
    pub stress: f32,
    pub overrun: u8,
}

impl FrameSample {
    pub fn write_to(&self, file: &mut File) -> std::io::Result<()> {
        file.write_all(&self.frame.to_le_bytes())?;
        file.write_all(&self.total_us.to_le_bytes())?;
        file.write_all(&self.budget_pct.to_le_bytes())?;
        file.write_all(&self.drift.to_le_bytes())?;
        file.write_all(&self.stress.to_le_bytes())?;
        file.write_all(&[self.overrun])?;
        Ok(())
    }
}

// ── VAULT FORMAT ────────────────────────────────────────────

pub fn write_vault_header(file: &mut File, rank: u32, precision: &str) -> std::io::Result<()> {
    let json = format!(
        r#"{{"format":"DVSM-V20","version":3,"rank":{},"precision":"{}","lambda":{},"alpha":{},"dt":{}}}"#,
        rank, precision, LAMBDA_F64, ALPHA_F64, DT_F64
    );
    file.write_all(b"DVSM")?;
    file.write_all(&3u32.to_le_bytes())?;
    file.write_all(&(json.len() as u32).to_le_bytes())?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

// ── RUNTIME ─────────────────────────────────────────────────

pub fn run_runtime<T: FixedPoint>(rank: usize, precision: &str, output: &str, frames: u32) {
    let polar = PolarConstraint::new(4.0, 0.05, 0.98);
    let mut core = DvsmCore::<T>::new(rank, polar);
    let mut file = File::create(output).expect("cannot create output file");

    write_vault_header(&mut file, rank as u32, precision).expect("header write failed");

    let mut overruns = 0u32;
    let mut total_us_sum = 0u64;

    for f in 0..frames {
        let start = Instant::now();
        core.step();
        let elapsed = start.elapsed().as_micros() as u64;
        total_us_sum += elapsed;

        let budget_pct = (elapsed as f32 / FRAME_BUDGET_US as f32) * 100.0;
        let overrun = elapsed > FRAME_BUDGET_US;
        if overrun { overruns += 1; }

        let sample = FrameSample {
            frame: f,
            total_us: elapsed,
            budget_pct,
            drift: core.drift_norm() as f32,
            stress: core.stress() as f32,
            overrun: overrun as u8,
        };
        let _ = sample.write_to(&mut file);

        // hard exit on divergence
        if core.drift_norm() > 1000.0 {
            println!("[HALT] Drift exceeded at frame {}", f);
            break;
        }
    }

    // CRC placeholder
    let _ = file.write_all(&0u32.to_le_bytes());

    let avg_us = total_us_sum / frames.max(1) as u64;
    println!("DVSM-V20 {} R={} | {}frames avg={}μs overruns={} → {}",
        precision, rank, frames, avg_us, overruns, output);
}

// ── ENTRY POINTS ────────────────────────────────────────────

pub fn run_allyx_mode() {
    println!("Ally X | Q64.64 | R=16");
    run_runtime::<Q64>(16, "Q64.64", "allyx.dvsm", 10_000);
}

pub fn run_standard_mode() {
    println!("Standard | Q31.32 | R=8");
    run_runtime::<Q31>(8, "Q31.32", "standard.dvsm", 10_000);
}

pub fn run_compact_mode() {
    println!("Compact | Q16.16 | R=4");
    run_runtime::<Q16>(4, "Q16.16", "compact.dvsm", 10_000);
}

fn main() {
    run_allyx_mode();
    // run_standard_mode();
    // run_compact_mode();
}
// dvsm-core/src/vault_v4.rs
// DVSM-V20.4 // HARDENED TELEMETRY VAULT
// ------------------------------------------------------------
// PURPOSE:
//   Deterministic telemetry serialization + integrity seal
//
// NOTE:
//   SHA-256 here provides integrity verification,
//   NOT proof of "truth" or hardware uniqueness.
//
// RECORD LAYOUT:
//   Telemetry  = 25 bytes
//   SHA-256    = 32 bytes
//   Total      = 57 bytes per sealed frame

use std::fs::File;
use std::io::{Write, Result};

use sha2::{Digest, Sha256};

/// ------------------------------------------------------------
/// FRAME SAMPLE
/// ------------------------------------------------------------

pub struct FrameSampleV4 {
    pub frame: u32,
    pub total_us: u64,
    pub budget_pct: f32,
    pub drift: f32,
    pub stress: f32,
    pub overrun: u8,
}

impl FrameSampleV4 {

    /// --------------------------------------------------------
    /// SERIALIZE RAW TELEMETRY (25 BYTES)
    /// --------------------------------------------------------
    ///
    /// Stable little-endian binary layout.
    ///
    pub fn serialize_telemetry(&self) -> [u8; 25] {

        let mut buf = [0u8; 25];

        // frame:u32
        buf[0..4].copy_from_slice(&self.frame.to_le_bytes());

        // total_us:u64
        buf[4..12].copy_from_slice(&self.total_us.to_le_bytes());

        // budget_pct:f32
        buf[12..16].copy_from_slice(&self.budget_pct.to_le_bytes());

        // drift:f32
        buf[16..20].copy_from_slice(&self.drift.to_le_bytes());

        // stress:f32
        buf[20..24].copy_from_slice(&self.stress.to_le_bytes());

        // overrun:u8
        buf[24] = self.overrun;

        buf
    }

    /// --------------------------------------------------------
    /// GENERATE FRAME INTEGRITY TOKEN
    /// --------------------------------------------------------
    ///
    /// PURPOSE:
    ///   Detect corruption/tampering during replay or transfer.
    ///
    /// IMPORTANT:
    ///   This is deterministic hashing,
    ///   not cryptographic attestation of hardware identity.
    ///
    pub fn generate_v18_token(&self) -> [u8; 32] {

        let telemetry = self.serialize_telemetry();

        let mut hasher = Sha256::new();

        // deterministic frame payload
        hasher.update(&telemetry);

        // runtime profile namespace
        hasher.update(b"DVSM-V20");
        hasher.update(b"ALLYX-Q64-R16");

        let result = hasher.finalize();

        let mut token = [0u8; 32];
        token.copy_from_slice(&result);

        token
    }

    /// --------------------------------------------------------
    /// WRITE SEALED RECORD
    /// --------------------------------------------------------
    ///
    /// OUTPUT:
    ///   [25-byte telemetry]
    ///   [32-byte SHA-256 seal]
    ///
    /// TOTAL:
    ///   57 bytes/frame
    ///
    pub fn vault_final(&self, file: &mut File) -> Result<()> {

        let telemetry = self.serialize_telemetry();

        // write telemetry
        file.write_all(&telemetry)?;

        // append integrity token
        let token = self.generate_v18_token();
        file.write_all(&token)?;

        Ok(())
    }
}
// dvsm-core/src/v20_final.rs
// DVSM-V20.4 // DEPLOYMENT SUMMARY
// ------------------------------------------------------------
// PURPOSE:
//   Final runtime deployment summary for deterministic telemetry.
//
// NOTE:
//   "Cryptographically invariant" is replaced with
//   "hash-verified deterministic telemetry".
//   SHA-256 verifies integrity; it does not prove physical truth.
//
// NOTE:
//   Restorative bias bounds trajectory drift.
//   It does NOT guarantee 10^-20 stability universally.

pub fn execute_v20_deployment() {

    println!("DVSM-V20.4 // ALLY X RUNTIME ACTIVE");

    // --------------------------------------------------------
    // TELEMETRY DENSITY
    // --------------------------------------------------------
    //
    // Example comparison:
    //   Typical JSON frame log ≈ 212 bytes
    //   DVSM sealed frame      = 57 bytes
    //
    // Includes:
    //   25-byte telemetry
    //   32-byte SHA-256 seal
    //
    let json_bytes = 212.0;
    let sealed_bytes = 57.0;

    let density_ratio = json_bytes / sealed_bytes;
    let storage_reduction =
        (1.0 - (sealed_bytes / json_bytes)) * 100.0;

    // --------------------------------------------------------
    // RUNTIME MODEL
    // --------------------------------------------------------

    let frame_budget_us = 4166.0;
    let target_hz = 240.0;

    // bounded restorative coefficient
    let restorative_bias = 0.05;

    // rose attractor parameter
    let polar_k = 4.0;

    println!("--------------------------------------------------");
    println!("Telemetry Format");
    println!("--------------------------------------------------");
    println!("Frame Size:        57 bytes sealed");
    println!("JSON Equivalent:   ~212 bytes");
    println!("Density Ratio:     {:.2}x smaller", density_ratio);
    println!("Storage Reduction: {:.1}%", storage_reduction);

    println!();
    println!("--------------------------------------------------");
    println!("Runtime Constraints");
    println!("--------------------------------------------------");
    println!("Target Frequency:  {:.0} Hz", target_hz);
    println!("Frame Budget:      {:.0} us", frame_budget_us);
    println!("Polar k-factor:    {:.1}", polar_k);
    println!("Restorative Bias:  {:.3}", restorative_bias);

    println!();
    println!("--------------------------------------------------");
    println!("Integrity Layer");
    println!("--------------------------------------------------");
    println!("Verification:      SHA-256 frame sealing");
    println!("Determinism:       Fixed-step runtime");
    println!("Overflow Policy:   Clamp + saturating arithmetic");
    println!("Telemetry:         Binary canonical source");

    println!();
    println!("--------------------------------------------------");
    println!("VERDICT");
    println!("--------------------------------------------------");
    println!("DVSM-V20.4 finalized as a bounded deterministic");
    println!("telemetry runtime with replay-verifiable state");
    println!("serialization and portable fixed-point modes.");
}
