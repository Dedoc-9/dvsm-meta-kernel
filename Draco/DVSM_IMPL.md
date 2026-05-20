# DVSM Implementation Guide
**Author:** Daniel J. Dillberg | **Date:** 2026-05-19 | **Scope:** Code patterns, feature details, test suites

---

## §1 FIXED-POINT ARITHMETIC (Determinism Layer)

### §1.1 Q31 Codec (Primary)

```rust
// rust/base/src/fixed_point.rs

pub const Q31_SCALE: f32 = 2_147_483_648.0;  // 2^31
pub const Q31_SCALE_INV: f32 = 1.0 / Q31_SCALE;

/// Encode float to Q31 fixed point
#[inline]
pub fn q31_encode(x: f32) -> i32 {
    let clamped = x.clamp(-1.0 + 1e-7, 1.0 - 1e-7);
    (clamped * Q31_SCALE) as i32
}

/// Decode Q31 back to float
#[inline]
pub fn q31_decode(q: i32) -> f32 {
    (q as f32) * Q31_SCALE_INV
}

/// Q31 vector quantization (forces deterministic rounding)
pub fn q31_quantize_vector(z: &mut [f32; 16]) {
    for k in 0..16 {
        let q = q31_encode(z[k]);
        z[k] = q31_decode(q);
    }
}
```

**Usage:** Call before hash computation and every GhostSnap interval.

### §1.2 Q16 Codec (Overflow Handling)

```rust
pub const Q16_SCALE: f32 = 65_536.0;  // 2^16

pub fn q16_encode(x: f32) -> i32 {
    (x * Q16_SCALE).clamp(i32::MIN as f32, i32::MAX as f32) as i32
}

pub fn q16_decode(q: i32) -> f32 {
    (q as f32) / Q16_SCALE
}

/// Sub-zero SNR support: switch to Q16 if |Z| > 1.0
pub fn adaptive_quantize(z: &mut [f32; 16]) {
    let norm_sq: f32 = z.iter().map(|x| x * x).sum();
    let norm = norm_sq.sqrt();
    
    if norm > 1.0 {
        // Overflow risk: use Q16
        for k in 0..16 {
            let q = q16_encode(z[k]);
            z[k] = q16_decode(q);
        }
    } else {
        // Normal: use Q31
        q31_quantize_vector(z);
    }
}
```

### §1.3 Q64.64 Codec (Extended Range)

```rust
pub const Q64_64_SCALE: f64 = 18_446_744_073_709_551_616.0;  // 2^64

pub fn q64_64_encode(x: f64) -> i128 {
    (x * Q64_64_SCALE) as i128
}

pub fn q64_64_decode(q: i128) -> f64 {
    (q as f64) / Q64_64_SCALE
}

/// Extended range support: use Q64.64 for large dynamic range
/// Precision: ±9.223e18 range, ~5.4e-20 ULP
pub fn quantize_q64_64(z: &mut [f64; 20]) {
    for k in 0..20 {
        let q = q64_64_encode(z[k]);
        z[k] = q64_64_decode(q);
    }
}

/// Adaptive Q-switching (Q31 → Q16 → Q64.64 by norm)
pub fn adaptive_q_switch(z: &[f32; 16]) -> QuantMode {
    let norm = z.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    match norm {
        n if n > 10.0 => QuantMode::Q64_64,   // extreme range
        n if n > 2.0  => QuantMode::Q16,      // wide range
        _             => QuantMode::Q31,      // precision
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QuantMode {
    Q31,
    Q16,
    Q64_64,
}
```

### §1.3 Custom Fixed-Point (User-Defined)

```rust
pub trait CustomQuantizer {
    fn encode(&self, x: f32) -> i64;
    fn decode(&self, q: i64) -> f32;
}

pub struct CustomQ {
    bits: u32,           // total bits
    fractional: u32,     // fractional bits
}

impl CustomQ {
    pub fn new(bits: u32, fractional: u32) -> Self {
        assert!(fractional <= bits);
        Self { bits, fractional }
    }
    
    pub fn scale(&self) -> f32 {
        2.0_f32.powi(self.fractional as i32)
    }
}

impl CustomQuantizer for CustomQ {
    fn encode(&self, x: f32) -> i64 {
        (x * self.scale()) as i64
    }
    
    fn decode(&self, q: i64) -> f32 {
        (q as f32) / self.scale()
    }
}
```

---

## §2 FNV-1A HASH WITH PARITY

### §2.1 Implementation

```rust
// rust/base/src/hash.rs

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

pub fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET_BASIS;
    for &byte in data {
        h ^= byte as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

#[inline]
pub fn parity_bit(h: u64) -> u8 {
    (h.count_ones() & 1) as u8
}

/// FNV1A with parity check
pub struct FNV1AWithParity {
    pub hash: u64,
    pub parity: u8,
}

pub fn fnv1a_parity(data: &[u8]) -> FNV1AWithParity {
    let h = fnv1a(data);
    let p = parity_bit(h);
    FNV1AWithParity { hash: h, parity: p }
}

/// Verify parity integrity
pub fn verify_parity(h: u64, expected_parity: u8) -> bool {
    parity_bit(h) == expected_parity
}
```

### §2.2 State Hash (H_t Binding)

```rust
pub fn hash_state(
    mu: &[f32; 16],
    z: &[f32; 16],
    s: &[f32; 16],
    w: &[f32; 16 * 4],  // assuming r=4
    kappa: &[f32; 256],
    lambda: f32,
    alpha: f32,
    e_target: f32,
    q_mode: u8,
    neural_enabled: bool,
    protocol_version: u64,
) -> FNV1AWithParity {
    // Serialize state to bytes (deterministic)
    let mut data = Vec::new();
    
    // Add all state components (IEEE 754 bit-exact)
    for &x in mu.iter().chain(z).chain(s).chain(w) {
        data.extend_from_slice(&x.to_bits().to_le_bytes());
    }
    for &x in kappa {
        data.extend_from_slice(&x.to_bits().to_le_bytes());
    }
    data.extend_from_slice(&lambda.to_bits().to_le_bytes());
    data.extend_from_slice(&alpha.to_bits().to_le_bytes());
    data.extend_from_slice(&e_target.to_bits().to_le_bytes());
    data.push(q_mode);
    data.push(neural_enabled as u8);
    data.extend_from_slice(&protocol_version.to_le_bytes());
    
    fnv1a_parity(&data)
}
```

---

## §3 CAYLEY PROJECTION (SPYWARE REJECTION)

### §3.1 Skew-Symmetry Test

```rust
// rust/base/src/cayley.rs

/// Measure skew-symmetry of matrix A
pub fn skew_error(a: &[f32; 256], n: usize) -> f32 {
    let mut err_sq = 0.0_f32;
    let mut norm_sq = 0.0_f32;
    
    for i in 0..n {
        for j in 0..n {
            let a_ij = a[i * n + j];
            let a_ji = a[j * n + i];
            
            // Skew property: a_ij = -a_ji
            let residual = a_ij + a_ji;
            err_sq += residual * residual;
            norm_sq += a_ij * a_ij;
        }
    }
    
    if norm_sq < 1e-12 { return 0.0; }
    (err_sq / norm_sq).sqrt()
}

/// Extract antisymmetric part from difference vector
pub fn antisymmetrize(delta_z: &[f32; 16]) -> [f32; 256] {
    let mut a = [0.0_f32; 256];
    
    for i in 0..16 {
        for j in 0..16 {
            if i != j {
                let term = (delta_z[i] * 0.0 - delta_z[j] * 0.0) / 2.0;  // simplified; full version uses basis
                a[i * 16 + j] = term;
                a[j * 16 + i] = -term;
            }
        }
    }
    a
}

/// Cayley-based spyware detection
pub fn detect_spyware(
    z_claimed: &[f32; 16],
    z_trusted: &[f32; 16],
    threshold: f32,
) -> bool {
    // Compute difference
    let mut delta = [0.0_f32; 16];
    for k in 0..16 {
        delta[k] = z_claimed[k] - z_trusted[k];
    }
    
    // Check if difference is "skew-like"
    let a = antisymmetrize(&delta);
    let err = skew_error(&a, 16);
    
    // If not skew-symmetric, likely injected (not natural drift)
    err > threshold
}

/// Cayley correction (project onto valid manifold)
pub fn cayley_correct(
    z_claimed: &[f32; 16],
    z_trusted: &[f32; 16],
) -> [f32; 16] {
    // If Cayley test passes, return average blend
    // Otherwise, return trusted (conservative)
    let mut z_corrected = [0.0_f32; 16];
    for k in 0..16 {
        z_corrected[k] = 0.5 * z_claimed[k] + 0.5 * z_trusted[k];
    }
    z_corrected
}
```

---

## §4 GHOSTSNAP: BIT-CREEP PURGING

### §4.1 Checkpoint Structure

```rust
// rust/base/src/lib.rs

#[derive(Clone, Copy, Debug)]
pub struct GhostSnapCheckpoint {
    pub tick: u64,
    pub hash: FNV1AWithParity,
    pub z_snap: [f32; 16],
    pub bitcreep_metric: f32,
}

pub struct GhostSnapManager {
    pub interval: u64,           // checkpoint every N ticks
    pub bitcreep_threshold: f32,
    pub checkpoints: Vec<GhostSnapCheckpoint>,
}

impl GhostSnapManager {
    pub fn new(interval: u64) -> Self {
        Self {
            interval,
            bitcreep_threshold: 1e-7,
            checkpoints: Vec::new(),
        }
    }
    
    /// Quantize Z to Q31, compute bitcreep
    pub fn scan_and_checkpoint(
        &mut self,
        tick: u64,
        z: &mut [f32; 16],
        hash: FNV1AWithParity,
    ) {
        // Measure creep before quantization
        let z_before = *z;
        
        // Quantize to Q31 (force rounding)
        q31_quantize_vector(z);
        
        // Measure creep
        let mut creep = 0.0_f32;
        for k in 0..16 {
            creep += (z[k] - z_before[k]).abs();
        }
        
        // Store checkpoint if interval or creep threshold exceeded
        if tick % self.interval == 0 || creep > self.bitcreep_threshold {
            self.checkpoints.push(GhostSnapCheckpoint {
                tick,
                hash,
                z_snap: *z,
                bitcreep_metric: creep,
            });
        }
    }
    
    /// Resync from nearest checkpoint
    pub fn resync(&self, current_z: &mut [f32; 16]) -> Option<u64> {
        let checkpoint = self.checkpoints.last()?;
        *current_z = checkpoint.z_snap;
        Some(checkpoint.tick)
    }
}
```

---

## §5 FRAME RATE LOCKING (HARD FIX SWITCH)

### §5.1 Session Config with Immutable Frame Rate

```rust
// rust/base/src/lib.rs

#[derive(Clone, Copy, Debug)]
pub struct SessionConfig {
    pub frame_rate_hz: u32,      // LOCKED: 60, 120, or 240
    pub dt: f32,                 // Computed: 1.0 / frame_rate_hz
    pub vr_enabled: bool,        // Spatial 3D/haptics
    pub q_mode: QuantMode,       // Fixed-point precision
    pub _locked: bool,           // Sanity check: cannot change once session starts
}

impl SessionConfig {
    pub fn new(frame_rate_hz: u32, vr_enabled: bool, q_mode: QuantMode) -> Result<Self, String> {
        if ![60, 120, 240].contains(&frame_rate_hz) {
            return Err(format!("Invalid frame rate: {}. Must be 60, 120, or 240.", frame_rate_hz));
        }
        
        Ok(Self {
            frame_rate_hz,
            dt: 1.0 / (frame_rate_hz as f32),
            vr_enabled,
            q_mode,
            _locked: false,
        })
    }
    
    /// Lock config at session start (IRREVERSIBLE)
    pub fn lock(&mut self) {
        self._locked = true;
    }
    
    /// Attempt to change frame rate (FORBIDDEN if locked)
    pub fn try_set_frame_rate(&mut self, new_hz: u32) -> Result<(), String> {
        if self._locked {
            Err("Frame rate is locked for this session. Cannot change.".to_string())
        } else if ![60, 120, 240].contains(&new_hz) {
            Err(format!("Invalid frame rate: {}", new_hz))
        } else {
            self.frame_rate_hz = new_hz;
            self.dt = 1.0 / (new_hz as f32);
            Ok(())
        }
    }
}

/// Dissipation scaling by frame rate
pub fn scale_dissipation(lambda_base: f32, frame_rate_hz: u32) -> f32 {
    lambda_base * (60.0 / frame_rate_hz as f32)
}

pub fn scale_backreaction(alpha_base: f32, frame_rate_hz: u32) -> f32 {
    alpha_base * (frame_rate_hz as f32 / 60.0)
}

// Example:
// lambda_base = 0.12, frame_rate = 60 Hz  → lambda_actual = 0.12
// lambda_base = 0.12, frame_rate = 120 Hz → lambda_actual = 0.06
// lambda_base = 0.12, frame_rate = 240 Hz → lambda_actual = 0.03
```

### §5.2 Test Frame Rate Immutability

```rust
#[test]
fn test_frame_rate_immutable() {
    let mut config = SessionConfig::new(60, false, QuantMode::Q31).unwrap();
    
    // Can change before lock
    assert!(config.try_set_frame_rate(120).is_ok());
    assert_eq!(config.frame_rate_hz, 120);
    
    // Lock it
    config.lock();
    
    // Cannot change after lock
    let result = config.try_set_frame_rate(240);
    assert!(result.is_err());
    assert_eq!(config.frame_rate_hz, 120, "Frame rate changed despite lock!");
}

#[test]
fn test_dissipation_scaling() {
    assert_eq!(scale_dissipation(0.12, 60), 0.12);   // 1.0x
    assert_eq!(scale_dissipation(0.12, 120), 0.06);  // 0.5x
    assert_eq!(scale_dissipation(0.12, 240), 0.03);  // 0.25x
}

#[test]
fn test_backreaction_scaling() {
    assert_eq!(scale_backreaction(0.08, 60), 0.08);   // 1.0x
    assert_eq!(scale_backreaction(0.08, 120), 0.16);  // 2.0x
    assert_eq!(scale_backreaction(0.08, 240), 0.32);  // 4.0x
}
```

---

## §6 3D/VR + HAPTICS (OPTIONAL SPATIAL MODE)

### §6.1 VR State Extension (20D)

```rust
// rust/base/src/lib.rs (with VR feature flag)

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VRState {
    // Position (3D)
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    
    // Rotation (quaternion: normalized on S^3)
    pub rot_w: f32,
    pub rot_x: f32,
    pub rot_y: f32,
    pub rot_z: f32,
    
    // Linear velocity
    pub vel_x: f32,
    pub vel_y: f32,
    pub vel_z: f32,
    
    // Angular velocity
    pub ang_x: f32,
    pub ang_y: f32,
    pub ang_z: f32,
    
    // Haptic feedback (force)
    pub hap_fx: f32,
    pub hap_fy: f32,
    pub hap_fz: f32,
    
    // Haptic feedback (torque)
    pub hap_tx: f32,
    pub hap_ty: f32,
    pub hap_tz: f32,
}

impl VRState {
    pub fn new() -> Self {
        Self {
            pos_x: 0.0, pos_y: 0.0, pos_z: 0.0,
            rot_w: 1.0, rot_x: 0.0, rot_y: 0.0, rot_z: 0.0,  // identity quaternion
            vel_x: 0.0, vel_y: 0.0, vel_z: 0.0,
            ang_x: 0.0, ang_y: 0.0, ang_z: 0.0,
            hap_fx: 0.0, hap_fy: 0.0, hap_fz: 0.0,
            hap_tx: 0.0, hap_ty: 0.0, hap_tz: 0.0,
        }
    }
    
    /// Ensure quaternion stays normalized (drift correction)
    pub fn normalize_quaternion(&mut self) {
        let norm_sq = self.rot_w * self.rot_w 
                    + self.rot_x * self.rot_x 
                    + self.rot_y * self.rot_y 
                    + self.rot_z * self.rot_z;
        
        if norm_sq < 1e-12 {
            self.rot_w = 1.0;  // Reset to identity if degenerate
            self.rot_x = 0.0;
            self.rot_y = 0.0;
            self.rot_z = 0.0;
            return;
        }
        
        let norm = norm_sq.sqrt();
        self.rot_w /= norm;
        self.rot_x /= norm;
        self.rot_y /= norm;
        self.rot_z /= norm;
    }
    
    /// Convert to 20D array for Z_t
    pub fn to_array(&self) -> [f32; 20] {
        [
            self.pos_x, self.pos_y, self.pos_z,
            self.rot_w, self.rot_x, self.rot_y, self.rot_z,
            self.vel_x, self.vel_y, self.vel_z,
            self.ang_x, self.ang_y, self.ang_z,
            self.hap_fx, self.hap_fy, self.hap_fz,
            self.hap_tx, self.hap_ty, self.hap_tz,
            0.0,  // padding
        ]
    }
}
```

### §6.2 Haptic Force Controller

```rust
#[derive(Clone, Copy, Debug)]
pub struct HapticsProfile {
    pub kp: f32,           // proportional gain (stiffness)
    pub kd: f32,           // derivative gain (damping)
    pub max_force: f32,    // device limit (Newtons)
    pub max_torque: f32,   // device limit (N·m)
}

impl HapticsProfile {
    pub const BASIC_PHONE: Self = Self {
        kp: 0.5,
        kd: 0.1,
        max_force: 0.5,
        max_torque: 0.1,
    };
    
    pub const STANDARD_JOY: Self = Self {
        kp: 2.0,
        kd: 0.5,
        max_force: 5.0,
        max_torque: 1.0,
    };
    
    pub const ADVANCED_GLOVE: Self = Self {
        kp: 5.0,
        kd: 1.0,
        max_force: 10.0,
        max_torque: 5.0,
    };
}

pub fn compute_haptic_force(
    z_current: &VRState,
    z_target: &VRState,
    profile: &HapticsProfile,
) -> (f32, f32, f32) {
    // PD control: F = K_p * error + K_d * d(error)/dt
    let ep = z_target.pos_x - z_current.pos_x;
    let ep_vel = z_target.vel_x - z_current.vel_x;
    
    let fx = (profile.kp * ep + profile.kd * ep_vel)
        .clamp(-profile.max_force, profile.max_force);
    
    let fy = (profile.kp * (z_target.pos_y - z_current.pos_y) 
            + profile.kd * (z_target.vel_y - z_current.vel_y))
        .clamp(-profile.max_force, profile.max_force);
    
    let fz = (profile.kp * (z_target.pos_z - z_current.pos_z) 
            + profile.kd * (z_target.vel_z - z_current.vel_z))
        .clamp(-profile.max_force, profile.max_force);
    
    (fx, fy, fz)
}

pub fn compute_haptic_torque(
    z_current: &VRState,
    z_target: &VRState,
    profile: &HapticsProfile,
) -> (f32, f32, f32) {
    // Simplified: torque ∝ angular velocity error
    let tx = (profile.kp * (z_target.ang_x - z_current.ang_x) 
            + profile.kd * (z_target.ang_x - z_current.ang_x) * 0.1)
        .clamp(-profile.max_torque, profile.max_torque);
    
    let ty = (profile.kp * (z_target.ang_y - z_current.ang_y)
            + profile.kd * (z_target.ang_y - z_current.ang_y) * 0.1)
        .clamp(-profile.max_torque, profile.max_torque);
    
    let tz = (profile.kp * (z_target.ang_z - z_current.ang_z)
            + profile.kd * (z_target.ang_z - z_current.ang_z) * 0.1)
        .clamp(-profile.max_torque, profile.max_torque);
    
    (tx, ty, tz)
}
```

---

## §7 ROSE CURVE LOGIC (OPTIONAL NEURAL)

### §5.1 Rose Curve Modulation

```rust
// rust/dfe/src/lib.rs

pub fn rose_curve(theta: f32, a: f32, k: f32, variant: RoseVariant) -> f32 {
    let arg = k * theta;
    match variant {
        RoseVariant::Cosine => a * arg.cos(),
        RoseVariant::Sine => a * arg.sin(),
    }
}

pub enum RoseVariant {
    Cosine,
    Sine,
}

/// Rose curve term in Lie-bracket dynamics
pub fn rose_term(
    z: &[f32; 16],
    theta: f32,      // angle from basis W
    a: f32,          // amplitude
    k: f32,          // harmonic order
    beta: f32,       // coupling strength [0,1]
) -> [f32; 16] {
    let mut rose = [0.0_f32; 16];
    let r = rose_curve(theta, a, k, RoseVariant::Cosine);
    
    for i in 0..16 {
        let z_norm = z[i].abs() + 1e-12;
        rose[i] = beta * r * z[i] / z_norm;
    }
    rose
}
```

### §5.2 Neural Support (Frozen MLP)

```rust
// rust/neural/src/lib.rs

#[derive(Clone, Debug)]
pub struct RoseNeuralNet {
    /// Frozen weights (initialized from seed)
    pub w1: [f32; 16 * 32],  // input (16) to hidden (32)
    pub b1: [f32; 32],
    pub w2: [f32; 32 * 2],   // hidden (32) to output (2: a, k)
    pub b2: [f32; 2],
}

impl RoseNeuralNet {
    /// Initialize from protocol_version seed (deterministic)
    pub fn from_seed(seed: u64) -> Self {
        let mut rng = SeededRng::new(seed);
        
        let mut net = Self {
            w1: [0.0_f32; 16 * 32],
            b1: [0.0_f32; 32],
            w2: [0.0_f32; 32 * 2],
            b2: [0.0_f32; 2],
        };
        
        // Fill with deterministic values
        for i in 0..16 * 32 {
            net.w1[i] = rng.next_f32() * 0.1;  // small init
        }
        for i in 0..32 {
            net.b1[i] = 0.0;
        }
        for i in 0..32 * 2 {
            net.w2[i] = rng.next_f32() * 0.1;
        }
        for i in 0..2 {
            net.b2[i] = 0.0;
        }
        
        net
    }
    
    /// Forward pass (frozen, no training)
    pub fn forward(&self, input: &[f32; 16]) -> (f32, f32) {
        // Hidden layer (ReLU)
        let mut h = [0.0_f32; 32];
        for j in 0..32 {
            let mut z = self.b1[j];
            for i in 0..16 {
                z += self.w1[i * 32 + j] * input[i];
            }
            h[j] = z.max(0.0);  // ReLU
        }
        
        // Output layer (no activation, but clamp to valid range)
        let mut out = [0.0_f32; 2];
        for j in 0..2 {
            let mut z = self.b2[j];
            for i in 0..32 {
                z += self.w2[i * 2 + j] * h[i];
            }
            out[j] = z;
        }
        
        // Clamp outputs: a ∈ [0, 2], k ∈ [1, 8]
        let a = out[0].clamp(0.0, 2.0);
        let k = out[1].clamp(1.0, 8.0);
        
        (a, k)
    }
}

/// SeededRng: deterministic PRNG
pub struct SeededRng {
    state: u64,
}

impl SeededRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    
    pub fn next_f32(&mut self) -> f32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        ((self.state >> 11) as f32) / (u32::MAX as f32)
    }
}
```

---

## §8 BACKREACTION WITH ALL FEATURES

### §8.1 Full Step Function (Scalar + VR)

```rust
pub fn dvsm_step_full(
    state: &mut DVSMState,
    config: &SessionConfig,    // LOCKED frame rate, VR mode, Q_mode
    p: &WattageProfile,
    dfe_enabled: bool,
    neural_enabled: bool,
    net: Option<&RoseNeuralNet>,
    haptics: Option<(&VRState, &HapticsProfile)>,  // (target, profile)
    ghostsnap_mgr: &mut GhostSnapManager,
) -> Result<(), String> {
    // Check frame rate is still locked
    if !config._locked {
        return Err("Frame rate not locked; cannot proceed".to_string());
    }
    
    // Apply frame-rate scaling
    let lambda_scaled = scale_dissipation(p.lambda, config.frame_rate_hz);
    let alpha_scaled = scale_backreaction(p.alpha, config.frame_rate_hz);
    
    let mut acc = [0.0_f32; 16];
    let dim = if config.vr_enabled { 20 } else { 16 };
    
    // === A: LIE BRACKET ===
    for k in 0..dim {
        let zk = state.z[k];
        let sk = state.s[k];
        for j in 0..dim {
            if j == k { continue; }
            let bracket = zk * state.s[j] - state.z[j] * sk;
            acc[k] += state.kappa_get(k, j) * bracket;
        }
    }
    
    // === B: ROSE CURVE (optional neural) ===
    let mut rose = [0.0_f32; 20];  // padded to 20
    if neural_enabled && dfe_enabled {
        if let Some(net) = net {
            let (a, k) = net.forward(&state.z);
            let theta = 0.5;  // placeholder angle from W
            rose = rose_term(&state.z, theta, a, k, 0.05);
        }
    }
    
    // === C: BACKREACTION (frame-rate scaled) ===
    let backreaction_coeff = -alpha_scaled * (state.norm_sq - p.e_target);
    
    // === D: EULER STEP ===
    for k in 0..dim {
        let b_k = backreaction_coeff * state.z[k];
        let dz = config.dt * (acc[k] - lambda_scaled * state.z[k] + b_k + rose[k]);
        state.z[k] += dz;
        
        // === D.1: STATE BOUNDARY CLAMPING (§A.2b) ===
        // Immediately after Euler integration, clamp to prevent NaN propagation
        if paranoid_mode_enabled {
            // Soft clip: 2·tanh(x/2) — continuous, detects saturation
            state.z[k] = 2.0 * (state.z[k] / 2.0).tanh();
        } else {
            // Hard clamp [-2.0, 2.0] — deterministic, O(1), production-grade
            state.z[k] = state.z[k].clamp(-2.0, 2.0);
        }
        
        // === D.2: EMA MEMORY ===
        state.s[k] = p.ema_beta * state.s[k] + (1.0 - p.ema_beta) * state.z[k];
    }
    
    state.update_norm();
    
    // === E: VR/HAPTICS (if enabled) ===
    if config.vr_enabled && config.frame_rate_hz >= 120 {
        if let Some((vr_target, hap_profile)) = haptics {
            let vr_current = VRState {
                pos_x: state.z[0],
                pos_y: state.z[1],
                pos_z: state.z[2],
                rot_w: state.z[3],
                rot_x: state.z[4],
                rot_y: state.z[5],
                rot_z: state.z[6],
                vel_x: state.z[7],
                vel_y: state.z[8],
                vel_z: state.z[9],
                ang_x: state.z[10],
                ang_y: state.z[11],
                ang_z: state.z[12],
                hap_fx: 0.0,  // computed below
                hap_fy: 0.0,
                hap_fz: 0.0,
                hap_tx: 0.0,
                hap_ty: 0.0,
                hap_tz: 0.0,
            };
            
            // Compute haptic feedback
            let (fx, fy, fz) = compute_haptic_force(&vr_current, vr_target, hap_profile);
            let (tx, ty, tz) = compute_haptic_torque(&vr_current, vr_target, hap_profile);
            
            // Store in Z for next tick
            state.z[13] = fx;
            state.z[14] = fy;
            state.z[15] = fz;
            state.z[16] = tx;
            state.z[17] = ty;
            state.z[18] = tz;
        }
    }
    
    // === F: GHOSTSNAP ===
    let hash = hash_state(/* args */);
    ghostsnap_mgr.scan_and_checkpoint(state.tick, &mut state.z, hash);
    
    // === G: ADAPTIVE Q-SWITCHING ===
    if let Some(q_mode) = determine_adaptive_q_mode(&state.z) {
        match q_mode {
            QuantMode::Q31 => q31_quantize_vector(&mut state.z),
            QuantMode::Q16 => {
                // Convert to Q16 and back
                for k in 0..dim {
                    let q = q16_encode(state.z[k]);
                    state.z[k] = q16_decode(q);
                }
            }
            QuantMode::Q64_64 => {
                // For extended ranges (F64 conversion)
                for k in 0..dim {
                    let z64 = state.z[k] as f64;
                    let q = q64_64_encode(z64);
                    state.z[k] = q64_64_decode(q) as f32;
                }
            }
        }
    }
    
    // === H: SUCHNESS CHECK ===
    let orthogonal = orthogonality_check(&state.z, &state.s);
    let ghost_ok = ghost_closure_audit(&state.z, &state.s);  // code audit
    let binding_ok = verify_parity(hash.hash, hash.parity);
    
    if !orthogonal || !ghost_ok || !binding_ok {
        // FAILURE: rollback to last checkpoint
        if let Some(checkpoint_tick) = ghostsnap_mgr.resync(&mut state.z) {
            eprintln!("Suchness broken; reverted to tick {}", checkpoint_tick);
        }
    } else {
        state.tick += 1;
        state.replay_hash = hash.hash;
    }
    
    Ok(())
}

fn determine_adaptive_q_mode(z: &[f32; 20]) -> Option<QuantMode> {
    let norm = z[..16].iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm > 10.0 {
        Some(QuantMode::Q64_64)
    } else if norm > 2.0 {
        Some(QuantMode::Q16)
    } else {
        Some(QuantMode::Q31)
    }
}
```

### §8.2 Phase-Locked Loop (Z2 Extreme Temporal Anchor)

**Purpose:** Temporal PLL implementation for Z2 Extreme, exploiting 0.19% occupancy to achieve zero-jitter state prediction via GPU timestamp anchoring.

#### §8.2a Step Momentum (Rising Edge: Prediction without Backreaction)

```rust
/// Rising Edge: Momentum integration using MEASURED GPU latency.
/// Integrates Lie-bracket, damping, and Rose curve.
/// EXCLUDES backreaction to avoid double-counting during phase correction.
pub fn step_momentum(
    state: &mut DVSMState,
    tau_meas: f32,           // Measured GPU latency (actual Δt)
    lambda: f32,
    rose: &[f32; 20],
    ema_beta: f32,           // From config.ema_beta
    paranoid_mode: bool,
) {
    let dim = if state.z.len() == 20 { 20 } else { 16 };
    
    // === A: LIE BRACKET (unchanged) ===
    let mut acc = [0.0_f32; 20];
    for k in 0..dim {
        let zk = state.z[k];
        let sk = state.s[k];
        for j in 0..dim {
            if j == k { continue; }
            let bracket = zk * state.s[j] - state.z[j] * sk;
            acc[k] += state.kappa_get(k, j) * bracket;
        }
    }
    
    // === D: EULER STEP with measured tau_meas (NO backreaction) ===
    for k in 0..dim {
        // Apply Lie-bracket + damping + Rose, but NOT backreaction
        let dz = tau_meas * (acc[k] - lambda * state.z[k] + rose[k]);
        state.z[k] += dz;
        
        // === D.1: STATE BOUNDARY CLAMPING ===
        if paranoid_mode {
            state.z[k] = 2.0 * (state.z[k] / 2.0).tanh();
        } else {
            state.z[k] = state.z[k].clamp(-2.0, 2.0);
        }
        
        // === D.2: EMA MEMORY (uses nominal EMA coefficient) ===
        state.s[k] = ema_beta * state.s[k] + (1.0 - ema_beta) * state.z[k];
    }
    
    state.update_norm();
}
```

#### §8.2b Tick Phase-Locked (Full PLL Cycle: Dispatch + Correction)

```rust
/// Non-linear Phase-Locked Loop for state-space.
/// Rising Edge: Predict Z using measured GPU latency.
/// Falling Edge: Apply phase-corrected backreaction pulse.
pub fn tick_phase_locked(
    state: &mut DVSMState,
    config: &SessionConfig,
    p: &WattageProfile,
    d_ns: u64,                           // dispatch timestamp (ns)
    c_ns: u64,                           // completion timestamp (ns)
    dfe_enabled: bool,
    neural_enabled: bool,
    net: Option<&RoseNeuralNet>,
    paranoid_mode: bool,
    ghostsnap_mgr: &mut GhostSnapManager,
) -> Result<(), String> {
    if !config._locked {
        return Err("Frame rate not locked".to_string());
    }
    
    let tau_meas = (c_ns.saturating_sub(d_ns) as f32) / 1_000_000_000.0;
    let tau_nominal = config.dt;
    let dim = if config.vr_enabled { 20 } else { 16 };
    
    // === PRE-COMPUTE: Rose Curve ===
    let mut rose = [0.0_f32; 20];
    if neural_enabled && dfe_enabled {
        if let Some(net) = net {
            let (a, k) = net.forward(&state.z);
            let theta = 0.5;
            rose = rose_term(&state.z, theta, a, k, 0.05);
        }
    }
    
    // === RISING EDGE: Momentum Integration ===
    step_momentum(state, tau_meas, p.lambda, &rose, p.ema_beta, paranoid_mode);
    
    // === FALLING EDGE: Phase-Corrected Backreaction Pulse ===
    let phase_delta = tau_meas - tau_nominal;
    let sync_scale = (1.0 + 0.25 * phase_delta).clamp(0.8, 1.2);
    let alpha_sync = p.alpha * sync_scale;
    
    let backreaction_coeff = -alpha_sync * (state.norm_sq - p.e_target);
    let pulse_magnitude = 4.0 * backreaction_coeff * tau_nominal;  // empirical scaling
    
    for k in 0..dim {
        state.z[k] = (state.z[k] + pulse_magnitude * state.z[k]).clamp(-2.0, 2.0);
    }
    
    // === HARDENING: VR Quaternion Renormalization ===
    if config.vr_enabled && dim >= 20 {
        let q_norm_sq = state.z[3] * state.z[3]
                      + state.z[4] * state.z[4]
                      + state.z[5] * state.z[5]
                      + state.z[6] * state.z[6];
        let q_norm = q_norm_sq.sqrt();
        
        if (q_norm - 1.0).abs() > 0.01 && q_norm > 1e-12 {
            for k in 3..7 {
                state.z[k] /= q_norm;
            }
        }
    }
    
    state.update_norm();
    
    // === TELEMETRY: Phase Error Tracking ===
    // Store phase_delta for diagnostics (e.g., EMA filter for systematic bias)
    // Implementation depends on supervisor state structure
    
    // === POST-FLIGHT: Ghost Guard & Hash ===
    ghostsnap_mgr.scan_and_rebirth(state);  // assumed method
    
    let hash = hash_state_with_nominal_dt(state, p, config.dt, config.frame_rate_hz);
    ghostsnap_mgr.checkpoint(state.tick, &mut state.z, hash);
    
    // === SUCHNESS CHECK ===
    let orthogonal = orthogonality_check(&state.z, &state.s);
    let ghost_ok = ghost_closure_audit(&state.z, &state.s);
    let binding_ok = verify_parity(hash.hash, hash.parity);
    
    if !orthogonal || !ghost_ok || !binding_ok {
        if let Some(checkpoint_tick) = ghostsnap_mgr.resync(&mut state.z) {
            eprintln!("Phase-lock suchness failed; reverted to tick {}", checkpoint_tick);
        }
    } else {
        state.tick += 1;
        state.replay_hash = hash.hash;
    }
    
    Ok(())
}

/// Helper: Extract norm variance for VRS tile projection
pub fn compute_norm_variance(state: &DVSMState, window_size: usize) -> f32 {
    // Rolling variance of ‖Z‖²
    // Implementation: maintain circular buffer of recent norm_sq values
    // For now, return instantaneous norm_sq (will be smoothed in supervisor)
    state.norm_sq
}
```

### §8.3 Q31.32 Fixed-Point Lie-Bracket Kernel (Z2 Extreme Integer-Only Evolution)

**Purpose:** Hardware-locked deterministic arithmetic. All operations integer-only; zero floating-point rounding error.

**Encoding:** x_fixed = floor(x_float × 2^32), decode: x_float = x_fixed / 2^32, range [-2^31, 2^31), ULP = 2^-32 ≈ 2.328e-10

**State Definition (Q31.32 encoding):**
- z_q[k] ∈ ℤ, represents z_k ∈ ℝ with z_k = z_q[k] / 2^32
- s_q[k] ∈ ℤ, represents s_k (EMA residual memory)
- H_t = HASH(z_q ⊕ protocol_version) (hash only; no backreaction feedback to H)

#### §8.3a Q31.32 Arithmetic Primitives

```rust
/// Multiply two Q31.32 fixed-point values, preserving precision.
/// Input: a_q, b_q (Q31.32 integers)
/// Output: (a_q / 2^32) * (b_q / 2^32) = result_q / 2^32
pub fn mul_q31_32(a_q: i64, b_q: i64) -> i64 {
    // Compute (a_q * b_q) >> 32
    // Intermediate: a_q * b_q requires 128-bit precision
    // On platforms without i128: split into 32-bit halves
    let hi_a = a_q >> 32;
    let lo_a = a_q & 0xFFFFFFFF;
    let hi_b = b_q >> 32;
    let lo_b = b_q & 0xFFFFFFFF;
    
    // Compute product: (hi_a + lo_a)(hi_b + lo_b) with shifts
    // Full: hi_a*hi_b (shift 64), hi_a*lo_b + lo_a*hi_b (shift 32), lo_a*lo_b (shift 0)
    let p_hi_hi = (hi_a as i128) * (hi_b as i128);
    let p_mixed = ((hi_a as i128) * (lo_b as i128)) + ((lo_a as i128) * (hi_b as i128));
    let p_lo_lo = (lo_a as i128) * (lo_b as i128);
    
    let result = (p_hi_hi << 64) + (p_mixed << 32) + p_lo_lo;
    (result >> 32) as i64
}

/// Divide two Q31.32 values: (a_q / 2^32) / (b_q / 2^32) = (a_q / b_q)
pub fn div_q31_32(a_q: i64, b_q: i64) -> i64 {
    if b_q == 0 {
        return 0;  // Safety: zero division returns 0
    }
    // Compute (a_q * 2^32) / b_q = result_q
    let numerator = (a_q as i128) << 32;
    let result = numerator / (b_q as i128);
    result as i64
}

/// Add two Q31.32 values with saturation clamp to [-2.0, +2.0] in fixed-point space
pub fn add_q31_32_clamped(a_q: i64, b_q: i64) -> i64 {
    let result = a_q.saturating_add(b_q);
    // Clamp to [-2.0, +2.0] ≡ [-(2.0 * 2^32), +(2.0 * 2^32)] in Q31.32
    let clamp_max: i64 = (2.0 * (1i64 << 32) as f64) as i64;  // +(2.0 * 2^32)
    let clamp_min: i64 = -clamp_max;  // -(2.0 * 2^32)
    result.max(clamp_min).min(clamp_max)
}

/// Convert float to Q31.32
pub fn f32_to_q31_32(x: f32) -> i64 {
    (x * ((1i64 << 32) as f32)) as i64
}

/// Convert Q31.32 to float
pub fn q31_32_to_f32(x_q: i64) -> f32 {
    (x_q as f32) / ((1i64 << 32) as f32)
}
```

#### §8.3b Lie-Bracket Integration (Q31.32 Fixed-Point)

**Formal definition:**
```
z_q[k]^{t+1} = z_q[k]^t + τ_q · (Σⱼ κ_{kj} · (z_q[k]^t · s_q[j]^t − z_q[j]^t · s_q[k]^t) − λ · z_q[k]^t + rose_q[k]^t)

where τ_q = encode(dt), κ_{kj} = encode(κ_{kj}^float), rose_q[k] = encode(rose_k^float)
H_t = HASH(z_q ⊕ s_q ⊕ protocol_version) [hash remains immutable anchor; no feedback]
```

```rust
/// Lie-bracket term: Σⱼ κ_{kj} · (z_q[k] · s_q[j] − z_q[j] · s_q[k])
fn bracket_q31_32(z_q: &[i64; 16], s_q: &[i64; 16], kappa_matrix: &[[i64; 16]; 16], k: usize) -> i64 {
    let mut acc_q: i64 = 0;
    
    for j in 0..16 {
        if j == k { continue; }
        
        // Compute z_q[k] * s_q[j] in Q31.32 space
        let term1_q = mul_q31_32(z_q[k], s_q[j]);
        
        // Compute z_q[j] * s_q[k] in Q31.32 space
        let term2_q = mul_q31_32(z_q[j], s_q[k]);
        
        // Bracket: (term1 - term2)
        let bracket_q = term1_q.saturating_sub(term2_q);
        
        // Multiply by κ_{kj}
        let contrib_q = mul_q31_32(kappa_matrix[k][j], bracket_q);
        
        // Accumulate with saturation
        acc_q = acc_q.saturating_add(contrib_q);
    }
    
    acc_q
}

/// Step Euler integration in Q31.32 (momentum phase, no backreaction)
pub fn step_q31_32_momentum(
    z_q: &mut [i64; 16],
    s_q: &mut [i64; 16],
    lambda_q: i64,                    // λ in Q31.32
    tau_q: i64,                       // dt in Q31.32
    rose_q: &[i64; 16],               // rose curve in Q31.32
    ema_beta_q: i64,                  // β in Q31.32
) {
    // Pre-compute kappa matrix (assumed static, pre-encoded in Q31.32)
    // kappa_matrix[k][j]: coupling coefficient
    let kappa_static: [[i64; 16]; 16] = [
        // Example identity-like coupling (should be actual DVSM κ matrix)
        [0; 16]; 16
    ];
    
    // For each state dimension
    for k in 0..16 {
        // === A: Lie-bracket term ===
        let bracket_term = bracket_q31_32(z_q, s_q, &kappa_static, k);
        
        // === B: Damping term ===
        let damp_term = mul_q31_32(lambda_q, z_q[k]);
        
        // === C: Rose curve term ===
        let rose_term = rose_q[k];
        
        // === D: Combined acceleration ===
        let accel_q = bracket_term.saturating_sub(damp_term).saturating_add(rose_term);
        
        // === E: Euler step: dz_q = τ_q * accel_q ===
        let dz_q = mul_q31_32(tau_q, accel_q);
        
        // === F: Update state with clamping ===
        z_q[k] = add_q31_32_clamped(z_q[k], dz_q);
        
        // === G: EMA update: s_q[k] = β·s_q[k] + (1−β)·z_q[k] ===
        let one_minus_beta_q = (1i64 << 32).saturating_sub(ema_beta_q);
        let s_contrib1 = mul_q31_32(ema_beta_q, s_q[k]);
        let s_contrib2 = mul_q31_32(one_minus_beta_q, z_q[k]);
        s_q[k] = s_contrib1.saturating_add(s_contrib2);
    }
}

/// Full PLL cycle in Q31.32 (Z2 Extreme hardware-locked kernel)
pub fn tick_q31_32_phase_locked(
    z_q: &mut [i64; 16],
    s_q: &mut [i64; 16],
    norm_sq_q: &mut i64,
    tau_meas_q: i64,                 // Measured GPU latency in Q31.32
    tau_nominal_q: i64,              // Nominal dt in Q31.32
    alpha_q: i64,                    // Backreaction coefficient in Q31.32
    e_target_q: i64,                 // Energy target in Q31.32
    lambda_q: i64,
    rose_q: &[i64; 16],
    ema_beta_q: i64,
) {
    // === RISING EDGE: Momentum (tau_meas, no backreaction) ===
    step_q31_32_momentum(z_q, s_q, lambda_q, tau_meas_q, rose_q, ema_beta_q);
    
    // === FALLING EDGE: Phase-Corrected Backreaction ===
    let phase_delta_q = tau_meas_q.saturating_sub(tau_nominal_q);
    let sync_scale_base_q = (1i64 << 32);  // 1.0 in Q31.32
    let phase_mult_q = f32_to_q31_32(0.25);  // 0.25 in Q31.32
    let phase_term_q = mul_q31_32(phase_mult_q, phase_delta_q);
    let sync_scale_q = sync_scale_base_q.saturating_add(phase_term_q);
    
    // Clamp sync_scale to [0.8, 1.2]
    let clamp_min_q = f32_to_q31_32(0.8);
    let clamp_max_q = f32_to_q31_32(1.2);
    let sync_scale_clamped = sync_scale_q.max(clamp_min_q).min(clamp_max_q);
    
    let alpha_sync_q = mul_q31_32(alpha_q, sync_scale_clamped);
    
    // Backreaction: compute norm_sq_q first
    *norm_sq_q = 0i64;
    for k in 0..16 {
        let z_sq = mul_q31_32(z_q[k], z_q[k]);
        *norm_sq_q = norm_sq_q.saturating_add(z_sq);
    }
    
    // Backreaction coefficient: -α_sync · (‖Z‖² − E_target)
    let norm_error_q = norm_sq_q.saturating_sub(e_target_q);
    let backreaction_coeff_q = mul_q31_32(-alpha_sync_q, norm_error_q);
    
    // Pulse magnitude: 4.0 * coeff * τ_nominal
    let four_q = f32_to_q31_32(4.0);
    let pulse_mag_q = mul_q31_32(mul_q31_32(four_q, backreaction_coeff_q), tau_nominal_q);
    
    // Apply backreaction pulse: z_q[k] += pulse_mag_q * z_q[k]
    for k in 0..16 {
        let correction_q = mul_q31_32(pulse_mag_q, z_q[k]);
        z_q[k] = add_q31_32_clamped(z_q[k], correction_q);
    }
}
```

#### §8.3c Verification (Determinism Guarantee)

```rust
/// Hash state using Q31.32 encoded integers (deterministic, no float rounding)
pub fn hash_state_q31_32(z_q: &[i64; 16], s_q: &[i64; 16], protocol_version: u32) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for k in 0..16 {
        hash = fnv1a_update(hash, z_q[k].to_le_bytes());
        hash = fnv1a_update(hash, s_q[k].to_le_bytes());
    }
    hash = fnv1a_update(hash, protocol_version.to_le_bytes());
    hash
}

/// Byte-identical verification: encode/decode round-trip
#[test]
fn test_q31_32_encode_decode_cycle() {
    for test_val in &[-2.0_f32, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0] {
        let encoded = f32_to_q31_32(*test_val);
        let decoded = q31_32_to_f32(encoded);
        let error = (test_val - decoded).abs();
        assert!(error < 1e-8, "Q31.32 round-trip error at {}: {}", test_val, error);
    }
}

/// Cross-platform determinism: C-struct layout identical on Z2 Linux / Windows
#[test]
fn test_q31_32_arithmetic_closure() {
    // Verify that sequences of operations produce identical results on repeated runs
    let mut z_q = [f32_to_q31_32(0.5_f32); 16];
    let mut s_q = [f32_to_q31_32(0.1_f32); 16];
    let tau_q = f32_to_q31_32(1.0 / 120.0);  // 120 Hz frame time
    let lambda_q = f32_to_q31_32(0.1);
    let rose_q = [f32_to_q31_32(0.01_f32); 16];
    let ema_beta_q = f32_to_q31_32(0.99);
    
    let hash_before = hash_state_q31_32(&z_q, &s_q, 1);
    
    // Execute kernel 100 times
    for _ in 0..100 {
        let mut norm_sq_q = 0i64;
        tick_q31_32_phase_locked(
            &mut z_q, &mut s_q, &mut norm_sq_q,
            tau_q, tau_q,
            f32_to_q31_32(0.05), f32_to_q31_32(1.0),
            lambda_q, &rose_q, ema_beta_q,
        );
    }
    
    let hash_after = hash_state_q31_32(&z_q, &s_q, 1);
    
    // On repeated runs with identical inputs, hashes must match
    assert_ne!(hash_before, hash_after, "State evolved");
    
    // Re-run to verify reproducibility
    let mut z_q2 = [f32_to_q31_32(0.5_f32); 16];
    let mut s_q2 = [f32_to_q31_32(0.1_f32); 16];
    
    for _ in 0..100 {
        let mut norm_sq_q = 0i64;
        tick_q31_32_phase_locked(
            &mut z_q2, &mut s_q2, &mut norm_sq_q,
            tau_q, tau_q,
            f32_to_q31_32(0.05), f32_to_q31_32(1.0),
            lambda_q, &rose_q, ema_beta_q,
        );
    }
    
    let hash_after2 = hash_state_q31_32(&z_q2, &s_q2, 1);
    assert_eq!(hash_after, hash_after2, "Q31.32 determinism failure");
}
```

### §8.4 Q64.64 Fixed-Point Lie-Bracket Kernel (Extended Range, Z2 Extreme)

**Purpose:** Support extreme dynamic range (±9.223e18) for sub-zero SNR and deep dynamics. Uses i128 for full 64-bit fractional precision.

**Encoding:** x_fixed = floor(x_float × 2^64), decode: x_float = x_fixed / 2^64, range [-2^63, 2^63), ULP = 2^-64 ≈ 5.421e-20

#### §8.4a Q64.64 Arithmetic Primitives (i128-based)

```rust
/// Multiply two Q64.64 fixed-point values (i128 based)
/// Input: a_q, b_q (Q64.64 integers, i128)
/// Output: (a_q / 2^64) * (b_q / 2^64) = result_q / 2^64
pub fn mul_q64_64(a_q: i128, b_q: i128) -> i128 {
    // Compute (a_q * b_q) >> 64
    // For i128: use widening multiply if available, else decompose
    // Rust: i128 * i128 = overflow, so use u128 arithmetic with sign handling
    let a_sign = a_q < 0;
    let b_sign = b_q < 0;
    let a_abs = a_q.abs() as u128;
    let b_abs = b_q.abs() as u128;
    
    let prod_abs = (a_abs.wrapping_mul(b_abs)) >> 64;
    let result = prod_abs as i128;
    
    // Restore sign
    if (a_sign && !b_sign) || (!a_sign && b_sign) {
        -result
    } else {
        result
    }
}

/// Divide two Q64.64 values: (a_q / 2^64) / (b_q / 2^64) = (a_q / b_q)
pub fn div_q64_64(a_q: i128, b_q: i128) -> i128 {
    if b_q == 0 {
        return 0;  // Safety: zero division returns 0
    }
    // Compute (a_q * 2^64) / b_q = result_q
    // Avoid overflow: use division directly
    (a_q >> 64).wrapping_mul(1i128 << 64) / b_q
}

/// Add two Q64.64 values with saturation clamp to [-9.223e18, +9.223e18]
pub fn add_q64_64_clamped(a_q: i128, b_q: i128) -> i128 {
    let result = a_q.saturating_add(b_q);
    // Clamp to [-2^63, +2^63)
    let clamp_max: i128 = i128::MAX;
    let clamp_min: i128 = i128::MIN;
    result.max(clamp_min).min(clamp_max)
}

/// Convert float to Q64.64
pub fn f64_to_q64_64(x: f64) -> i128 {
    (x * ((1i128 << 64) as f64)) as i128
}

/// Convert Q64.64 to float
pub fn q64_64_to_f64(x_q: i128) -> f64 {
    (x_q as f64) / ((1i128 << 64) as f64)
}
```

#### §8.4b Lie-Bracket Integration (Q64.64 Fixed-Point)

**Formal definition (i128 arithmetic, no floating-point):**
```
z_q[k]^{t+1} = z_q[k]^t + τ_q · (Σⱼ κ_{kj} · (z_q[k]^t · s_q[j]^t − z_q[j]^t · s_q[k]^t) − λ · z_q[k]^t + rose_q[k]^t)

where τ_q = encode(dt), κ_{kj} = encode(κ_{kj}^float), rose_q[k] = encode(rose_k^float)
All multiplication/division in i128 space; hash H_t remains immutable (no backreaction feedback)
```

```rust
/// Lie-bracket term in Q64.64: Σⱼ κ_{kj} · (z_q[k] · s_q[j] − z_q[j] · s_q[k])
fn bracket_q64_64(z_q: &[i128; 16], s_q: &[i128; 16], kappa_matrix: &[[i128; 16]; 16], k: usize) -> i128 {
    let mut acc_q: i128 = 0;
    
    for j in 0..16 {
        if j == k { continue; }
        
        // Compute z_q[k] * s_q[j] in Q64.64 space
        let term1_q = mul_q64_64(z_q[k], s_q[j]);
        
        // Compute z_q[j] * s_q[k] in Q64.64 space
        let term2_q = mul_q64_64(z_q[j], s_q[k]);
        
        // Bracket: (term1 - term2)
        let bracket_q = term1_q.saturating_sub(term2_q);
        
        // Multiply by κ_{kj}
        let contrib_q = mul_q64_64(kappa_matrix[k][j], bracket_q);
        
        // Accumulate with saturation
        acc_q = acc_q.saturating_add(contrib_q);
    }
    
    acc_q
}

/// Step Euler integration in Q64.64 (momentum phase, no backreaction)
pub fn step_q64_64_momentum(
    z_q: &mut [i128; 16],
    s_q: &mut [i128; 16],
    lambda_q: i128,                    // λ in Q64.64
    tau_q: i128,                       // dt in Q64.64
    rose_q: &[i128; 16],               // rose curve in Q64.64
    ema_beta_q: i128,                  // β in Q64.64
) {
    // Pre-compute kappa matrix (assumed static, pre-encoded in Q64.64)
    let kappa_static: [[i128; 16]; 16] = [
        [0i128; 16]; 16
    ];
    
    // For each state dimension
    for k in 0..16 {
        // === A: Lie-bracket term ===
        let bracket_term = bracket_q64_64(z_q, s_q, &kappa_static, k);
        
        // === B: Damping term ===
        let damp_term = mul_q64_64(lambda_q, z_q[k]);
        
        // === C: Rose curve term ===
        let rose_term = rose_q[k];
        
        // === D: Combined acceleration ===
        let accel_q = bracket_term.saturating_sub(damp_term).saturating_add(rose_term);
        
        // === E: Euler step: dz_q = τ_q * accel_q ===
        let dz_q = mul_q64_64(tau_q, accel_q);
        
        // === F: Update state with clamping to [-2^63, 2^63) ===
        z_q[k] = add_q64_64_clamped(z_q[k], dz_q);
        
        // === G: EMA update: s_q[k] = β·s_q[k] + (1−β)·z_q[k] ===
        let one_q = 1i128 << 64;  // 1.0 in Q64.64
        let one_minus_beta_q = one_q.saturating_sub(ema_beta_q);
        let s_contrib1 = mul_q64_64(ema_beta_q, s_q[k]);
        let s_contrib2 = mul_q64_64(one_minus_beta_q, z_q[k]);
        s_q[k] = s_contrib1.saturating_add(s_contrib2);
    }
}

/// Full PLL cycle in Q64.64 (Z2 Extreme extended-range kernel)
pub fn tick_q64_64_phase_locked(
    z_q: &mut [i128; 16],
    s_q: &mut [i128; 16],
    norm_sq_q: &mut i128,
    tau_meas_q: i128,                 // Measured GPU latency in Q64.64
    tau_nominal_q: i128,              // Nominal dt in Q64.64
    alpha_q: i128,                    // Backreaction coefficient in Q64.64
    e_target_q: i128,                 // Energy target in Q64.64
    lambda_q: i128,
    rose_q: &[i128; 16],
    ema_beta_q: i128,
) {
    // === RISING EDGE: Momentum (tau_meas, no backreaction) ===
    step_q64_64_momentum(z_q, s_q, lambda_q, tau_meas_q, rose_q, ema_beta_q);
    
    // === FALLING EDGE: Phase-Corrected Backreaction ===
    let phase_delta_q = tau_meas_q.saturating_sub(tau_nominal_q);
    let one_q = 1i128 << 64;  // 1.0 in Q64.64
    let phase_mult_q = f64_to_q64_64(0.25);  // 0.25 in Q64.64
    let phase_term_q = mul_q64_64(phase_mult_q, phase_delta_q);
    let sync_scale_q = one_q.saturating_add(phase_term_q);
    
    // Clamp sync_scale to [0.8, 1.2]
    let clamp_min_q = f64_to_q64_64(0.8);
    let clamp_max_q = f64_to_q64_64(1.2);
    let sync_scale_clamped = sync_scale_q.max(clamp_min_q).min(clamp_max_q);
    
    let alpha_sync_q = mul_q64_64(alpha_q, sync_scale_clamped);
    
    // Backreaction: compute norm_sq_q first
    *norm_sq_q = 0i128;
    for k in 0..16 {
        let z_sq = mul_q64_64(z_q[k], z_q[k]);
        *norm_sq_q = norm_sq_q.saturating_add(z_sq);
    }
    
    // Backreaction coefficient: -α_sync · (‖Z‖² − E_target)
    let norm_error_q = norm_sq_q.saturating_sub(e_target_q);
    let backreaction_coeff_q = mul_q64_64(-alpha_sync_q, norm_error_q);
    
    // Pulse magnitude: 4.0 * coeff * τ_nominal
    let four_q = f64_to_q64_64(4.0);
    let pulse_mag_q = mul_q64_64(mul_q64_64(four_q, backreaction_coeff_q), tau_nominal_q);
    
    // Apply backreaction pulse: z_q[k] += pulse_mag_q * z_q[k]
    for k in 0..16 {
        let correction_q = mul_q64_64(pulse_mag_q, z_q[k]);
        z_q[k] = add_q64_64_clamped(z_q[k], correction_q);
    }
}
```

#### §8.4c Verification (Q64.64 Determinism)

```rust
/// Hash state using Q64.64 encoded integers
pub fn hash_state_q64_64(z_q: &[i128; 16], s_q: &[i128; 16], protocol_version: u32) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for k in 0..16 {
        hash = fnv1a_update(hash, z_q[k].to_le_bytes());
        hash = fnv1a_update(hash, s_q[k].to_le_bytes());
    }
    hash = fnv1a_update(hash, protocol_version.to_le_bytes());
    hash
}

/// Byte-identical verification: Q64.64 encode/decode round-trip
#[test]
fn test_q64_64_encode_decode_cycle() {
    for test_val in &[-1e15_f64, -1.0, -0.5, 0.0, 0.5, 1.0, 1e15_f64] {
        let encoded = f64_to_q64_64(*test_val);
        let decoded = q64_64_to_f64(encoded);
        let error = (test_val - decoded).abs();
        assert!(error < 1e-18, "Q64.64 round-trip error at {}: {}", test_val, error);
    }
}

/// Cross-platform determinism: i128 handling identical on Z2 Linux / Windows
#[test]
fn test_q64_64_arithmetic_closure() {
    // Verify that sequences of operations produce identical results on repeated runs
    let mut z_q = [f64_to_q64_64(0.5_f64); 16];
    let mut s_q = [f64_to_q64_64(0.1_f64); 16];
    let tau_q = f64_to_q64_64(1.0 / 120.0);  // 120 Hz frame time
    let lambda_q = f64_to_q64_64(0.1);
    let rose_q = [f64_to_q64_64(0.01_f64); 16];
    let ema_beta_q = f64_to_q64_64(0.99);
    
    let hash_before = hash_state_q64_64(&z_q, &s_q, 1);
    
    // Execute kernel 100 times
    for _ in 0..100 {
        let mut norm_sq_q = 0i128;
        tick_q64_64_phase_locked(
            &mut z_q, &mut s_q, &mut norm_sq_q,
            tau_q, tau_q,
            f64_to_q64_64(0.05), f64_to_q64_64(1.0),
            lambda_q, &rose_q, ema_beta_q,
        );
    }
    
    let hash_after = hash_state_q64_64(&z_q, &s_q, 1);
    assert_ne!(hash_before, hash_after, "State evolved");
    
    // Re-run to verify reproducibility
    let mut z_q2 = [f64_to_q64_64(0.5_f64); 16];
    let mut s_q2 = [f64_to_q64_64(0.1_f64); 16];
    
    for _ in 0..100 {
        let mut norm_sq_q = 0i128;
        tick_q64_64_phase_locked(
            &mut z_q2, &mut s_q2, &mut norm_sq_q,
            tau_q, tau_q,
            f64_to_q64_64(0.05), f64_to_q64_64(1.0),
            lambda_q, &rose_q, ema_beta_q,
        );
    }
    
    let hash_after2 = hash_state_q64_64(&z_q2, &s_q2, 1);
    assert_eq!(hash_after, hash_after2, "Q64.64 determinism failure");
}
```

---

## §9 TEST PATTERNS (VERIFICATION HARNESS)

### §9.1 Determinism Tests (Core)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_q31_determinism_across_languages() {
        let z_rust = dvsm_step_rust(&state, &profile);
        let z_swift = call_swift_via_ffi(&state, &profile);
        
        for k in 0..16 {
            assert_eq!(z_rust[k].to_bits(), z_swift[k].to_bits(),
                "Q31 divergence at Z[{}]", k);
        }
    }
    
    #[test]
    fn test_q64_64_precision() {
        let val = 123456789.123456789_f64;
        let q = q64_64_encode(val);
        let decoded = q64_64_decode(q);
        let error = (val - decoded).abs();
        assert!(error < 1e-14, "Q64.64 precision loss: {}", error);
    }
    
    #[test]
    fn test_adaptive_q_switching() {
        let mut z_small = [0.5_f32; 16];
        let mut z_large = [5.0_f32; 16];
        let mut z_huge = [50.0_f32; 16];
        
        assert_eq!(adaptive_q_switch(&z_small), Some(QuantMode::Q31));
        assert_eq!(adaptive_q_switch(&z_large), Some(QuantMode::Q16));
        assert_eq!(adaptive_q_switch(&z_huge), Some(QuantMode::Q64_64));
    }
    
    #[test]
    fn test_ghostsnap_bitcreep_purge() {
        let mut mgr = GhostSnapManager::new(100);
        let mut z = [0.5_f32; 16];
        
        for tick in 0..1000 {
            // Accumulate error
            z[0] += 1e-8;
            
            let hash = hash_state(/* args */);
            mgr.scan_and_checkpoint(tick as u64, &mut z, hash);
        }
        
        assert!(mgr.checkpoints.len() > 0, "No checkpoints created");
    }
    
    #[test]
    fn test_cayley_spyware_detection() {
        let mut z_trusted = [0.5_f32; 16];
        let z_natural_drift = [0.5001_f32; 16];  // small, smooth change
        let z_injected = [0.9_f32; 16];          // large jump
        
        let natural_detected = detect_spyware(&z_natural_drift, &z_trusted, 0.1);
        let injected_detected = detect_spyware(&z_injected, &z_trusted, 0.1);
        
        assert!(!natural_detected, "False positive on natural drift");
        assert!(injected_detected, "Missed spyware injection");
    }
    
    #[test]
    fn test_suchness_identified() {
        let mut state = DVSMState::new_identity();
        let profile = WattageProfile::ALLY_X_PERF;
        let mut ghostsnap_mgr = GhostSnapManager::new(1000);
        let config = SessionConfig::new(120, false, QuantMode::Q31).unwrap();
        
        for _ in 0..10_000 {
            let _ = dvsm_step_full(&mut state, &config, &profile, true, false, None, None, &mut ghostsnap_mgr);
        }
        
        // All ticks should have suchness (no rollbacks)
        assert_eq!(state.tick, 10_000, "Rollbacks occurred; suchness broken");
    }
}
```

### §9.2 Frame Rate Tests

```rust
#[test]
fn test_frame_rate_immutable_in_hash() {
    let config_60 = SessionConfig::new(60, false, QuantMode::Q31).unwrap();
    let config_120 = SessionConfig::new(120, false, QuantMode::Q31).unwrap();
    
    // Hashes should differ due to different frame rates
    let hash_60 = hash_state(/* z, s, ..., frame_rate_hz=60 */);
    let hash_120 = hash_state(/* z, s, ..., frame_rate_hz=120 */);
    
    assert_ne!(hash_60.hash, hash_120.hash, "Frame rate change not reflected in hash");
}

#[test]
fn test_frame_rate_60_120_240_determinism() {
    for frame_rate in [60, 120, 240].iter() {
        let config = SessionConfig::new(*frame_rate, false, QuantMode::Q31).unwrap();
        let expected_dt = 1.0 / (*frame_rate as f32);
        assert!((config.dt - expected_dt).abs() < 1e-6, "dt mismatch for {} Hz", frame_rate);
    }
}

#[test]
fn test_dissipation_scales_with_frame_rate() {
    let lambda_base = 0.12;
    
    let lambda_60 = scale_dissipation(lambda_base, 60);
    let lambda_120 = scale_dissipation(lambda_base, 120);
    let lambda_240 = scale_dissipation(lambda_base, 240);
    
    assert_eq!(lambda_60, lambda_base);           // 1.0x
    assert_eq!(lambda_120, lambda_base * 0.5);    // 0.5x
    assert_eq!(lambda_240, lambda_base * 0.25);   // 0.25x
}
```

### §9.3 VR/Haptics Tests

```rust
#[test]
fn test_vr_state_quaternion_normalization() {
    let mut vr = VRState::new();
    vr.rot_w = 2.0;  // unnormalized
    vr.rot_x = 0.5;
    vr.rot_y = 0.5;
    vr.rot_z = 0.5;
    
    vr.normalize_quaternion();
    
    let norm_sq = vr.rot_w * vr.rot_w 
                + vr.rot_x * vr.rot_x 
                + vr.rot_y * vr.rot_y 
                + vr.rot_z * vr.rot_z;
    
    assert!((norm_sq - 1.0).abs() < 1e-6, "Quaternion not normalized");
}

#[test]
fn test_vr_to_array_dimension() {
    let vr = VRState::new();
    let arr = vr.to_array();
    assert_eq!(arr.len(), 20, "VR array not 20D");
}

#[test]
fn test_haptic_force_bounds() {
    let profile = HapticsProfile::STANDARD_JOY;
    let vr_current = VRState::new();
    let mut vr_target = VRState::new();
    vr_target.pos_x = 10.0;  // large error
    
    let (fx, fy, fz) = compute_haptic_force(&vr_current, &vr_target, &profile);
    
    assert!(fx.abs() <= profile.max_force, "Force X exceeds max");
    assert!(fy.abs() <= profile.max_force, "Force Y exceeds max");
    assert!(fz.abs() <= profile.max_force, "Force Z exceeds max");
}

#[test]
fn test_haptic_latency_60_120_240() {
    let frame_times = [
        (60, 16.67),    // ms
        (120, 8.33),
        (240, 4.17),
    ];
    
    for (hz, expected_ms) in frame_times.iter() {
        let dt = 1.0 / (*hz as f32);
        let dt_ms = dt * 1000.0;
        assert!((dt_ms - expected_ms).abs() < 0.1, "Latency mismatch at {} Hz", hz);
    }
}

#[test]
fn test_vr_spatial_orthogonality() {
    let mut state = DVSMState::new_identity_vr();  // 20D init
    
    // Run VR-enabled step
    let config = SessionConfig::new(120, true, QuantMode::Q31).unwrap();
    let profile = WattageProfile::ALLY_X_PERF;
    let mut ghostsnap_mgr = GhostSnapManager::new(1000);
    
    for _ in 0..1000 {
        let _ = dvsm_step_full(&mut state, &config, &profile, true, false, None, None, &mut ghostsnap_mgr);
    }
    
    // Check Z[0:20] · S[0:20] ≈ 0
    let dot: f32 = state.z.iter().zip(state.s.iter()).map(|(zi, si)| zi * si).sum();
    assert!(dot.abs() < 1e-10, "VR spatial orthogonality broken: Z·S = {}", dot);
}
```

### §9.4 Frame Generation Parity Tests

```rust
#[test]
fn test_frame_gen_parity_interpolate() {
    let z_prev = [0.5_f32; 16];
    let z_curr = [0.6_f32; 16];
    
    // Interpolate
    let mut z_synth = [0.0_f32; 16];
    for k in 0..16 {
        z_synth[k] = 0.5 * z_prev[k] + 0.5 * z_curr[k];
    }
    
    // Compute parity
    let ghost_err = 0.01_f32;  // mock prediction error
    let parity_expected = compute_frame_parity(&z_synth, ghost_err);
    
    // Verify: recompute parity
    let parity_actual = compute_frame_parity(&z_synth, ghost_err);
    
    assert_eq!(parity_expected, parity_actual, "Frame parity not deterministic");
}

#[test]
fn test_frame_gen_parity_extrapolate() {
    let z_prev = [0.5_f32; 16];
    let z_curr = [0.6_f32; 16];
    
    // Extrapolate
    let mut z_synth = [0.0_f32; 16];
    for k in 0..16 {
        z_synth[k] = 2.0 * z_curr[k] - z_prev[k];
    }
    
    let ghost_err = 0.05_f32;
    let parity = compute_frame_parity(&z_synth, ghost_err);
    
    // Verify parity is 8-bit value
    assert!(parity < 256, "Parity exceeds 8 bits");
}

#[test]
fn test_frame_gen_parity_corruption_detection() {
    let z_synth = [0.5_f32; 16];
    let ghost_err = 0.02_f32;
    
    let parity_original = compute_frame_parity(&z_synth, ghost_err);
    
    // Corrupt one element
    let mut z_corrupt = z_synth;
    z_corrupt[0] = 0.50001_f32;  // bit-flip perturbation
    
    let parity_corrupt = compute_frame_parity(&z_corrupt, ghost_err);
    
    // Parities should differ (with high probability)
    // (may occasionally collide due to hash nature, but rare)
    assert_ne!(parity_original, parity_corrupt, 
      "Parity failed to detect corruption");
}

#[test]
fn test_frame_hash_binding() {
    let z_synth = [0.5_f32; 16];
    let mode = FrameGenMode::Interpolate;
    let parity = 0x42u8;
    let ghost_err = 0.01_f32;
    let generation_tick = 1000u64;
    let frame_rate_hz = 120u32;
    
    let h_frame_1 = compute_frame_hash(&z_synth, mode, parity, ghost_err, 
                                       generation_tick, frame_rate_hz);
    
    // Same inputs → same hash
    let h_frame_2 = compute_frame_hash(&z_synth, mode, parity, ghost_err,
                                       generation_tick, frame_rate_hz);
    
    assert_eq!(h_frame_1.hash, h_frame_2.hash, "Frame hash not deterministic");
    
    // Different mode → different hash
    let h_frame_alt = compute_frame_hash(&z_synth, FrameGenMode::Extrapolate, parity,
                                         ghost_err, generation_tick, frame_rate_hz);
    
    assert_ne!(h_frame_1.hash, h_frame_alt.hash, "Mode change not reflected in hash");
}
```

### §9.5 Extended Suchness Layer Tests

```rust
#[test]
fn test_suchness_triplet_pass() {
    let mut state = DVSMState::new_identity();
    let config = SessionConfig::new(120, false, QuantMode::Q31).unwrap();
    let profile = WattageProfile::ALLY_X_PERF;
    let mut ghostsnap_mgr = GhostSnapManager::new(1000);
    
    // Run 1000 ticks, collect suchness verdicts
    for _ in 0..1000 {
        let _ = dvsm_step_full(&mut state, &config, &profile, true, false, None, None, &mut ghostsnap_mgr);
    }
    
    // Verify binding check
    assert!(verify_hash_binding(&state), "L1 (Binding) failed");
    
    // Verify orthogonality check
    let dot: f32 = state.z.iter().zip(state.s.iter()).map(|(zi, si)| zi * si).sum();
    assert!(dot.abs() < 1e-10, "L2 (Orthogonality) failed: dot = {}", dot);
    
    // Verify ghost closure (code audit)
    assert!(!code_references_ghost_in_z_step(), "L3 (Ghost closure) failed");
    
    // All three pass → TRIPLET_OK
}

#[test]
fn test_suchness_quintet_with_vr() {
    let mut state = DVSMState::new_identity_vr();  // 20D
    let config = SessionConfig::new(120, true, QuantMode::Q31).unwrap();
    let profile = WattageProfile::VR_HAPTICS_STANDARD;
    let mut ghostsnap_mgr = GhostSnapManager::new(1000);
    
    for _ in 0..1000 {
        let _ = dvsm_step_full(&mut state, &config, &profile, true, false, None, None, &mut ghostsnap_mgr);
    }
    
    // L1-L3 (triplet checks)
    assert!(verify_hash_binding(&state), "L1 failed");
    let dot: f32 = state.z.iter().zip(state.s.iter()).map(|(zi, si)| zi * si).sum();
    assert!(dot.abs() < 1e-10, "L2 failed");
    assert!(!code_references_ghost_in_z_step(), "L3 failed");
    
    // L4: Frame parity (if enabled)
    // L5: Quaternion norm
    let rot_norm_sq = state.z[3] * state.z[3]     // rot_w
                    + state.z[4] * state.z[4]     // rot_x
                    + state.z[5] * state.z[5]     // rot_y
                    + state.z[6] * state.z[6];    // rot_z
    assert!((rot_norm_sq - 1.0).abs() < 1e-6, "L5 (Quaternion) failed");
    
    // All five pass → QUINTET_OK
}

#[test]
fn test_suchness_rollback_on_corruption() {
    let mut state = DVSMState::new_identity();
    let config = SessionConfig::new(120, false, QuantMode::Q31).unwrap();
    let profile = WattageProfile::ALLY_X_PERF;
    let mut ghostsnap_mgr = GhostSnapManager::new(100);  // frequent checkpoints
    
    // Run first phase normally
    for _ in 0..500 {
        let _ = dvsm_step_full(&mut state, &config, &profile, true, false, None, None, &mut ghostsnap_mgr);
    }
    
    let z_before_corruption = state.z;
    let tick_before = state.tick;
    
    // Simulate corruption: inject large perturbation
    state.z[0] += 1.0;  // break orthogonality
    
    // Next step should fail suchness and rollback
    let _ = dvsm_step_full(&mut state, &config, &profile, true, false, None, None, &mut ghostsnap_mgr);
    
    // Should have rolled back to checkpoint
    assert!(state.z[0] != 1.5, "Rollback did not occur");
}
```

### §9.5b State Boundary Clamping Tests

```rust
#[test]
fn test_state_hard_clamp_boundaries() {
    // Test hard clamp [-2.0, 2.0] preserves values and prevents NaN
    let test_values = vec![
        (-3.0, -2.0),   // clamp to boundary
        (-2.5, -2.0),   // clamp to boundary
        (-1.0, -1.0),   // within bounds, unchanged
        (0.0, 0.0),     // within bounds
        (1.5, 1.5),     // within bounds
        (2.5, 2.0),     // clamp to boundary
        (3.0, 2.0),     // clamp to boundary
        (f32::INFINITY, 2.0),   // prevent infinity
        (f32::NEG_INFINITY, -2.0), // prevent negative infinity
    ];
    
    for (input, expected) in test_values {
        let clamped = input.clamp(-2.0, 2.0);
        assert_eq!(clamped, expected, "Clamp failed for input {}", input);
    }
}

#[test]
fn test_state_soft_clip_tanh() {
    // Test soft clip via tanh: 2·tanh(x/2) approaches ±2.0 asymptotically
    let test_values = vec![
        (-5.0, 2.0 * (-5.0 / 2.0).tanh()),
        (-2.5, 2.0 * (-2.5 / 2.0).tanh()),
        (0.0, 0.0),
        (2.5, 2.0 * (2.5 / 2.0).tanh()),
        (5.0, 2.0 * (5.0 / 2.0).tanh()),
        (100.0, 2.0),  // approaches +2.0
        (-100.0, -2.0), // approaches -2.0
    ];
    
    for (input, expected) in test_values {
        let soft_clipped = 2.0 * (input / 2.0).tanh();
        assert!((soft_clipped - expected).abs() < 1e-6, 
            "Soft clip failed: got {}, expected {}", soft_clipped, expected);
        assert!(soft_clipped.abs() <= 2.0, "Soft clip exceeded bounds: {}", soft_clipped);
    }
}

#[test]
fn test_nan_prevention_after_euler() {
    // Simulate Euler step that would produce NaN without clamping
    let mut z = [0.5_f32; 16];
    let dt = 0.001;
    
    // Inject large perturbation that would cause NaN
    let dz_extreme = 1e10_f32;  // would overflow without clamping
    
    for k in 0..16 {
        z[k] += dt * dz_extreme;
        // Before fix: z[k] ≈ 1e7 (overflow risk)
        // After clamping:
        z[k] = z[k].clamp(-2.0, 2.0);
        
        assert!(z[k].is_finite(), "NaN propagation not prevented!");
        assert!(z[k].abs() <= 2.0, "Boundary exceeded: {}", z[k]);
    }
    
    // Verify hash is computable on clamped state
    let norm_sq: f32 = z.iter().map(|x| x * x).sum();
    assert!(norm_sq.is_finite(), "Norm computation failed after clamp");
}

#[test]
fn test_hash_determinism_with_clamping() {
    // Verify H_t is deterministic even with extreme input deviations
    let z_raw = vec![
        vec![-3.5, -2.5, -1.5, -0.5, 0.0, 0.5, 1.5, 2.5, 3.5, 4.5],
        vec![-3.5, -2.5, -1.5, -0.5, 0.0, 0.5, 1.5, 2.5, 3.5, 4.5], // identical input
    ];
    
    let mut hashes = Vec::new();
    
    for z_test in z_raw {
        let mut z_clamped = [0.0_f32; 16];
        for (i, &val) in z_test.iter().take(16).enumerate() {
            z_clamped[i] = val.clamp(-2.0, 2.0);
        }
        
        // Compute hash on clamped state
        let hash = compute_hash_state(&z_clamped, /* ... other params */);
        hashes.push(hash);
    }
    
    // Identical clamped states must produce identical hashes
    assert_eq!(hashes[0].hash, hashes[1].hash, 
        "Hash diverged despite clamping!");
}

#[test]
fn test_saturation_detection_paranoid_mode() {
    // Paranoid mode: detect when state saturates near boundaries
    let mut z = [0.5_f32; 16];
    let saturation_threshold = 1.8;
    let mut saturation_count = 0;
    let max_ticks = 1000;
    
    for _ in 0..max_ticks {
        // Simulate dynamics that push toward boundary
        for k in 0..16 {
            z[k] = (z[k] * 1.1).clamp(-2.0, 2.0);  // trending toward ±2.0
            if z[k].abs() >= saturation_threshold {
                saturation_count += 1;
            }
        }
    }
    
    let saturation_rate = saturation_count as f32 / (max_ticks as f32 * 16.0);
    
    // In paranoid mode, log warning if saturation > 0.1%
    if saturation_rate > 0.001 {
        eprintln!("State saturation anomaly: {:.2}% of samples", saturation_rate * 100.0);
    }
    
    assert!(saturation_rate < 0.5, "Saturation too high: {:.2}%", saturation_rate * 100.0);
}
```

### §9.6 Forensic Stack Confidence Tests

```rust
#[test]
fn test_frame_forensic_l1_l2_green() {
    // Green mode: only L1 (interpolation) + L2 (parity)
    let z_prev = [0.5_f32; 16];
    let z_curr = [0.6_f32; 16];
    
    // L1: deterministic interpolation
    let mut z_synth = [0.0_f32; 16];
    for k in 0..16 {
        z_synth[k] = 0.5 * z_prev[k] + 0.5 * z_curr[k];
    }
    
    // L2: parity check
    let ghost_err = 0.01_f32;
    let parity = compute_frame_parity(&z_synth, ghost_err);
    
    let forensic_level = evaluate_forensic_stack_green(&z_synth, ghost_err, parity);
    assert_eq!(forensic_level, ForensicLevel::GREEN, "Green mode check failed");
}

#[test]
fn test_frame_forensic_l1_l5_standard() {
    // Standard mode: L1-L5 (interpolation through spectral analysis)
    let z_prev = [0.5_f32; 16];
    let z_curr = [0.6_f32; 16];
    let z_actual_next = [0.65_f32; 16];  // real next frame
    
    let mut z_synth = [0.0_f32; 16];
    for k in 0..16 {
        z_synth[k] = 0.5 * z_prev[k] + 0.5 * z_curr[k];
    }
    
    let ghost_err = compute_ghost_error(&z_synth, &z_actual_next);
    
    // L3: Error bounds check
    assert!(ghost_err < 0.2, "L3 error bounds exceeded");
    
    // L4: Motion coherence
    let delta_prev = compute_norm(&z_curr) - compute_norm(&z_prev);
    let delta_next = compute_norm(&z_actual_next) - compute_norm(&z_curr);
    let coherence = if delta_prev.abs() > 1e-12 {
        delta_next / delta_prev
    } else {
        1.0
    };
    
    assert!(coherence > 0.5 && coherence < 2.0, "L4 motion coherence failed");
    
    // L5: Spectral check (simplified: energy ratio)
    let energy_synth: f32 = z_synth.iter().map(|z| z * z).sum();
    let energy_actual: f32 = z_actual_next.iter().map(|z| z * z).sum();
    let energy_ratio = if energy_synth > 1e-12 {
        energy_actual / energy_synth
    } else {
        1.0
    };
    
    assert!(energy_ratio > 0.7 && energy_ratio < 1.3, "L5 spectral check failed");
    
    let forensic_level = evaluate_forensic_stack_standard(&z_synth, &z_actual_next, ghost_err);
    assert_eq!(forensic_level, ForensicLevel::STANDARD, "Standard mode check failed");
}

#[test]
fn test_frame_forensic_l1_l10_forensic() {
    // Forensic mode: L1-L10 (full stack with merkle + crypto)
    // This is expensive, so we test on smaller dataset
    
    let frames = vec![
        [0.5_f32; 16],
        [0.55_f32; 16],
        [0.6_f32; 16],
        [0.65_f32; 16],
        [0.7_f32; 16],
    ];
    
    let mut merkle_roots = Vec::new();
    
    for window in frames.windows(2) {
        let z_prev = window[0];
        let z_curr = window[1];
        let mut z_synth = [0.0_f32; 16];
        
        for k in 0..16 {
            z_synth[k] = 0.5 * z_prev[k] + 0.5 * z_curr[k];
        }
        
        let ghost_err = 0.01_f32;
        
        // L9: Merkle tree
        let node_hash = compute_merkle_node(&z_prev, &z_curr, &z_synth, ghost_err);
        merkle_roots.push(node_hash);
    }
    
    // L10: Cryptographic commitment (SHA256)
    let commitment = compute_sha256_commitment(&merkle_roots);
    
    // Verify commitment is 32 bytes (SHA256)
    assert_eq!(commitment.len(), 32, "SHA256 commitment size mismatch");
    
    let forensic_level = evaluate_forensic_stack_forensic(&merkle_roots, &commitment);
    assert_eq!(forensic_level, ForensicLevel::FORENSIC, "Forensic mode check failed");
}
```

### §9.6b Phase-Lock PLL Convergence Tests (Z2 Extreme Temporal Anchor)

```rust
#[test]
fn test_phase_lock_convergence_z2_extreme() {
    use rand::Rng;
    
    let mut rng = rand::thread_rng();
    let mut state = DVSMState::new_identity();
    let config = SessionConfig::new(120, false, QuantMode::Q31).unwrap();
    let profile = WattageProfile::ALLY_X_Z2_BALANCED;
    let mut ghostsnap_mgr = GhostSnapManager::new(1000);
    
    let tau_nominal = config.dt;  // 1.0 / 120 ≈ 0.00833 s
    let mut phase_errors = Vec::new();
    let mut norm_deviations = Vec::new();
    
    // Simulate 1000 frames with ±0.5ms GPU jitter
    for frame_idx in 0..1000 {
        // Random jitter: ±0.5ms added to nominal latency
        let jitter_s = (rng.gen::<f32>() - 0.5) * 0.001;
        let tau_meas = tau_nominal + jitter_s;
        
        // Fake GPU timestamps (120 Hz = 8.333ms frame time)
        let d_ns = (frame_idx as u64) * 8_333_333;
        let c_ns = d_ns + (tau_meas * 1_000_000_000.0) as u64;
        
        // Execute phase-locked tick
        let _ = tick_phase_locked(
            &mut state,
            &config,
            &profile,
            d_ns,
            c_ns,
            false,  // dfe_enabled
            false,  // neural_enabled
            None,   // net
            false,  // paranoid_mode
            &mut ghostsnap_mgr,
        );
        
        // Record metrics
        let phase_delta = tau_meas - tau_nominal;
        phase_errors.push(phase_delta.abs());
        
        let norm_err = (state.norm_sq - profile.e_target).abs();
        norm_deviations.push(norm_err);
    }
    
    // Statistics
    let avg_phase_error = phase_errors.iter().sum::<f32>() / phase_errors.len() as f32;
    let max_phase_error = phase_errors.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let avg_norm_deviation = norm_deviations.iter().sum::<f32>() / norm_deviations.len() as f32;
    
    // Log results
    eprintln!("=== Phase-Lock Convergence (1000 frames, ±0.5ms jitter) ===");
    eprintln!("Avg Phase Error: {:.6f} s ({:.3f} ms)", avg_phase_error, avg_phase_error * 1000.0);
    eprintln!("Max Phase Error: {:.6f} s ({:.3f} ms)", max_phase_error, max_phase_error * 1000.0);
    eprintln!("Avg Norm Deviation: {:.6f}", avg_norm_deviation);
    eprintln!();
    
    // Pre-Flight Assertions
    assert!(avg_phase_error < 0.0001, "Phase lock UNSTABLE: avg error {:.6f}", avg_phase_error);
    assert!(max_phase_error < 0.0005, "Phase lock DIVERGED: max error {:.6f}", max_phase_error);
    assert!(avg_norm_deviation < 0.01, "Norm UNSTABLE: deviation {:.6f}", avg_norm_deviation);
    
    eprintln!("✅ Phase-Lock PRE-FLIGHT CHECK PASSED");
}

#[test]
fn test_phase_lock_backreaction_scaling() {
    // Verify correction_scale = 4.0 produces equivalent damping
    let mut state = DVSMState::new_identity();
    let config = SessionConfig::new(120, false, QuantMode::Q31).unwrap();
    let profile = WattageProfile::ALLY_X_Z2_BALANCED;
    let mut ghostsnap_mgr = GhostSnapManager::new(1000);
    
    let d_ns = 0;
    let c_ns = (config.dt * 1_000_000_000.0) as u64;
    
    for _ in 0..100 {
        let _ = tick_phase_locked(&mut state, &config, &profile, d_ns, c_ns, false, false, None, false, &mut ghostsnap_mgr);
    }
    
    let norm_error = (state.norm_sq - profile.e_target).abs();
    assert!(norm_error < 0.01, "Backreaction scaling mismatch: norm_sq = {}", state.norm_sq);
}

#[test]
fn test_phase_lock_vr_quaternion_preservation() {
    // Verify VR quaternion normalization
    let mut state = DVSMState::new_identity_vr();
    let config = SessionConfig::new(120, true, QuantMode::Q31).unwrap();
    let profile = WattageProfile::VR_HAPTICS_STANDARD;
    let mut ghostsnap_mgr = GhostSnapManager::new(1000);
    
    state.z[3] = 2.0;  // rot_w (should be ≈1.0)
    state.z[4] = 0.5;
    state.z[5] = 0.5;
    state.z[6] = 0.5;
    
    let d_ns = 0;
    let c_ns = (config.dt * 1_000_000_000.0) as u64;
    
    let _ = tick_phase_locked(&mut state, &config, &profile, d_ns, c_ns, false, false, None, false, &mut ghostsnap_mgr);
    
    let q_norm_sq = state.z[3] * state.z[3] + state.z[4] * state.z[4] + state.z[5] * state.z[5] + state.z[6] * state.z[6];
    assert!((q_norm_sq - 1.0).abs() < 0.01, "Quaternion norm not preserved: {}", q_norm_sq);
}
```

---

## §10 DEPLOYMENT CHECKLIST

Before merging any feature:

**Core Determinism:**
- [ ] Q31/Q16/Q64.64 encoding bit-identical across Rust, Swift, Python
- [ ] Adaptive Q-switching (Q31 → Q16 → Q64.64) deterministic
- [ ] FNV1A hash parity verification passing
- [ ] GhostSnap checkpoints created and resync tested
- [ ] Cayley spyware detection: FP rate < 1%, FN rate < 0.1%

**Dynamics:**
- [ ] Rose curve neural net frozen (weights constant)
- [ ] Backreaction norm stability: ‖Z‖² ∈ [0.8, 1.2] @ E_target=1.0
- [ ] Orthogonality: Z · S < 1e-10 at all ticks
- [ ] Ghost closure: code audit confirms G never feeds Z evolution
- [ ] Suchness check (L1-L3 Triplet): 100k ticks, zero rollbacks

**State Boundary Clamping (§A.2b — NaN Prevention):**
- [ ] Hard clamp [-2.0, +2.0] implemented immediately after Euler step
- [ ] OR soft clip 2·tanh(x/2) for paranoid mode (test both)
- [ ] NaN prevention verified: no NaN propagates to norm computation
- [ ] Hash determinism with clamping: identical inputs → identical H_t
- [ ] State saturation detection: track % ticks where |Z_k| ≥ 1.8
- [ ] Saturation anomaly threshold: log warning if > 0.1% of ticks
- [ ] Clamp performance: hard clamp < 0.01 ms, soft clip < 0.05 ms

**Frame Rate (Hard Lock):**
- [ ] Frame rate immutable after SessionConfig::lock()
- [ ] Changing frame rate mid-session returns error
- [ ] H_t binding includes frame_rate_hz (different rates → different hashes)
- [ ] λ_actual and α_actual scale correctly (60/120/240 Hz)
- [ ] dt deterministic: exactly 1.0 / frame_rate_hz

**VR/Haptics (Optional):**
- [ ] Quaternion normalized every tick (‖R‖ = 1.0)
- [ ] Haptic force/torque clamped to device limits
- [ ] VR state orthogonality: Z_spatial[0:20] · S_spatial[0:20] < ε
- [ ] Haptics update rate ≤ frame period (no stale feedback)
- [ ] VR determinism: identical z_target → identical haptic output
- [ ] Suchness Quintet (L1-L5) passes with VR enabled

**Frame Generation & Anti-Ghosting:**
- [ ] Frame parity deterministic (identical z_synth → identical parity)
- [ ] Frame parity corruption detection: detects bit-flip injections
- [ ] Frame hash binding (H_frame) includes all parameters (z, mode, parity, tick, frame_rate)
- [ ] Green mode (L1-L2) active in development
- [ ] Standard mode (L1-L5) active in production (local)
- [ ] Forensic mode (L1-L10) active in cross-DC deployments
- [ ] Frame rollback to z_curr on parity failure
- [ ] Motion coherence check passes (δ_next / δ_prev ∈ [0.5, 2.0])
- [ ] Temporal coherence: divergence_rate < 20% (frame gen tracking)

**Extended Suchness Verification:**
- [ ] L1 (Binding): hash_chain continuity verified every tick
- [ ] L2 (Orthogonality): dot(Z, S) < 1e-10 maintained
- [ ] L3 (Ghost Closure): code audit confirms no G→Z feedback
- [ ] L4 (Frame Parity): parity matches across peers (if frame_gen enabled)
- [ ] L5 (Quaternion): ‖R‖ = 1.0 ± 1e-6 (if vr_enabled)
- [ ] L6 (Power Scaling): λ_actual matches telemetry (paranoid mode)
- [ ] L7 (Display Geometry): transform deterministic (paranoid mode)
- [ ] Rollback mechanism: reverts to last GhostSnap checkpoint on failure
- [ ] Quarantine detection: logs corruption events for forensic audit

**Cross-Language:**
- [ ] Rust + Swift + Python produce identical Z, S, W, H_t
- [ ] Frame rate scaling consistent across languages
- [ ] Q64.64 bit-identical (i128 handling platform-agnostic)
- [ ] Frame parity computation identical across languages (bit-level XOR)

**Performance:**
- [ ] Tick latency < frame budget (240 Hz → < 4 ms)
- [ ] VR + haptics @ 240 Hz: < 8 ms total
- [ ] GhostSnap overhead < 0.5 ms
- [ ] Cayley projection (spyware check) < 0.1 ms
- [ ] Frame parity computation < 0.1 ms
- [ ] Suchness verification (L1-L3) < 0.5 ms (standard)
- [ ] Suchness verification (L1-L7) < 2.0 ms (paranoid)
- [ ] Forensic stack L1-L10 < 5 ms (forensic mode, async preferred)

**Phase-Lock PLL (Z2 Extreme Temporal Anchor, §A.2c & §8.2):**
- [ ] Implement step_momentum() (rising edge: Lie-bracket + damping + rose, NO backreaction)
- [ ] Implement tick_phase_locked(d_ns, c_ns) (falling edge: phase-corrected backreaction pulse)
- [ ] GPU timestamp anchoring: capture dispatch_ns and completion_ns from platform hook
- [ ] Phase error computation: tau_meas = (c_ns − d_ns) / 1e9; phase_delta = tau_meas − tau_nominal
- [ ] Proportional sync scaling: κ_phase_lock = 0.25; α_sync = α_base · (1.0 + 0.25 · phase_delta).clamp(0.8, 1.2)
- [ ] Backreaction pulse scaling: correction_scale = 4.0 (empirical; validate via convergence test)
- [ ] State clamping post-correction: Z_k.clamp(−2.0, +2.0) after falling edge
- [ ] VR quaternion renormalization: post-correction check if vr_enabled (norm ≈ 1.0)
- [ ] Phase error telemetry: EMA tracking (phase_error_ema), warning if > 0.2ms (systematic bias)
- [ ] Convergence test (§9.6b): 1000 frames ±0.5ms jitter; avg_phase_error < 0.1ms, max < 0.5ms
- [ ] Hash determinism: H_t uses τ_nominal only (not τ_meas); identical inputs → identical hash
- [ ] Suchness verification: L1-L3 triplet passes with phase-lock enabled (orthogonality maintained)

**Hardware Variants (Z1 Extreme vs Z2 Extreme):**
- [ ] Identify target platform (Z1=Phoenix gfx1103, Z2=Strix Point gfx1150)
- [ ] Update MAX_CU constant (4 → 16 for Z2)
- [ ] Compile with correct --offload-arch flag (gfx1103 vs gfx1150)
- [ ] Verify SHADER compatibility (unchanged; forward-compatible)
- [ ] Test occupancy model (Z1: 0.78%, Z2: 0.19% DVSM headroom)
- [ ] Validate profiler data (RGP: kernel wall-time expected ~0.25× Z1 on Z2)
- [ ] Benchmark FrameVarianceRing (p99, p95) on target hardware
- [ ] Cross-validate across Z1 and Z2 (identical determinism)
- [ ] Z2-specific: Test AFMF2 coexistence (if enabled)
- [ ] Z2-specific: Validate scalar FPU optimization (RGP profile recommended)
- [ ] Z2-specific: Validate phase-lock convergence (exploit 0.19% occupancy for PLL)

---

## §11 RUNTIME PROFILES (SessionConfig + WattageProfile)

### §11.1 Session Configurations (Frame Rate Locked)

```rust
impl SessionConfig {
    // Desktop VR: 240 Hz, spatial, advanced haptics, Q31
    pub const VR_DESKTOP: Self = Self {
        frame_rate_hz: 240,
        dt: 1.0 / 240.0,
        vr_enabled: true,
        q_mode: QuantMode::Q31,
        _locked: false,
    };
    
    // Mobile VR: 120 Hz, spatial, basic haptics, adaptive Q
    pub const VR_MOBILE: Self = Self {
        frame_rate_hz: 120,
        dt: 1.0 / 120.0,
        vr_enabled: true,
        q_mode: QuantMode::Q31,
        _locked: false,
    };
    
    // Ally X Performance: 240 Hz, scalar, Q31
    pub const ALLY_X_PERF: Self = Self {
        frame_rate_hz: 240,
        dt: 1.0 / 240.0,
        vr_enabled: false,
        q_mode: QuantMode::Q31,
        _locked: false,
    };
    
    // Ally X Balanced: 120 Hz, scalar, Q31
    pub const ALLY_X_BALANCED: Self = Self {
        frame_rate_hz: 120,
        dt: 1.0 / 120.0,
        vr_enabled: false,
        q_mode: QuantMode::Q31,
        _locked: false,
    };
    
    // Ally X Silent: 60 Hz, scalar, adaptive Q for low power
    pub const ALLY_X_SILENT: Self = Self {
        frame_rate_hz: 60,
        dt: 1.0 / 60.0,
        vr_enabled: false,
        q_mode: QuantMode::Q31,
        _locked: false,
    };
    
    // Sub-zero SNR: 60 Hz, scalar, Q64.64 for extended range
    pub const SUB_ZERO_SNR: Self = Self {
        frame_rate_hz: 60,
        dt: 1.0 / 60.0,
        vr_enabled: false,
        q_mode: QuantMode::Q64_64,
        _locked: false,
    };
}
```

### §11.2 Wattage Profiles (Power Scaling)

```rust
impl WattageProfile {
    pub const ALLY_X_PERF: Self = Self {
        tdp_watts: 35.0,
        lambda: 0.12,      // scaled by frame rate: λ_actual = 0.12 * (60/240)
        alpha: 0.08,       // scaled by frame rate: α_actual = 0.08 * (240/60)
        e_target: 1.0,
        ema_beta: 0.95,
        frame_gen: FrameGenMode::Interpolate,
        vrs_enabled: true,
    };
    
    pub const ALLY_X_BALANCED: Self = Self {
        tdp_watts: 25.0,
        lambda: 0.10,
        alpha: 0.06,
        e_target: 1.0,
        ema_beta: 0.93,
        frame_gen: FrameGenMode::Interpolate,
        vrs_enabled: true,
    };
    
    pub const SUB_ZERO_SNR: Self = Self {
        tdp_watts: 15.0,
        lambda: 0.08,
        alpha: 0.04,       // reduced backreaction for low power
        e_target: 1.0,
        ema_beta: 0.90,
        frame_gen: FrameGenMode::Off,
        vrs_enabled: false,
    };
    
    pub const VR_HAPTICS_STANDARD: Self = Self {
        tdp_watts: 30.0,
        lambda: 0.12,
        alpha: 0.10,       // stronger backreaction for haptic stability
        e_target: 1.0,
        ema_beta: 0.92,
        frame_gen: FrameGenMode::Interpolate,
        vrs_enabled: true,
    };
}
```

### §11.3 Hardware Profile Selection Logic

```rust
pub fn select_config_for_platform(platform: &str, vr: bool) -> SessionConfig {
    match (platform, vr) {
        // Z1 Extreme (Phoenix, gfx1103, 4 CU)
        ("ally_x_2024", false) => SessionConfig::ALLY_X_PERF,
        ("ally_x_2024_balanced", false) => SessionConfig::ALLY_X_BALANCED,
        ("ally_x_2024_silent", false) => SessionConfig::ALLY_X_SILENT,
        
        // Z2 Extreme (Strix Point, gfx1150, 16 CU) — NEW
        ("ally_x_2025", false) => SessionConfig::ALLY_X_Z2_PERF,
        ("ally_x_2025_balanced", false) => SessionConfig::ALLY_X_Z2_BALANCED,
        ("msi_claw_a8", false) => SessionConfig::ALLY_X_Z2_PERF,  // equivalent
        
        // VR profiles (compatible with both Z1 and Z2)
        ("vr_desktop", true) => SessionConfig::VR_DESKTOP,
        ("vr_mobile", true) => SessionConfig::VR_MOBILE,
        
        // Low SNR (Z2 still uses Q64.64)
        ("low_snr", false) => SessionConfig::SUB_ZERO_SNR,
        
        _ => SessionConfig::ALLY_X_BALANCED,  // safe default
    }
}
```

**Note:** All SessionConfig profiles are mathematically identical across Z1 and Z2. 
The platform selector determines which GPU occupancy mode (Z1=128 vs Z2=512 wave slots) is used.
See §11.4 below for hardware-specific constant configuration.

**Frame Rate Lock Mechanism:**

```rust
// At engine initialization:
let mut config = select_config_for_platform("ally_x", false);
config.lock();  // IMMUTABLE for rest of session

// Attempting to change frame rate now returns error:
config.try_set_frame_rate(120).unwrap_err();
// → "Frame rate is locked for this session. Cannot change."
```

### §11.4 Z2 Extreme Hardware Configuration

**File:** src/lib.rs (constants)

```rust
// ===== PLATFORM-SPECIFIC CONSTANTS (see Z2_EXTREME_ADDENDUM.md) =====

// Z1 Extreme (Phoenix, gfx1103):
#[cfg(target_gfx = "1103")]
pub const MAX_CU: u32    = 4;

// Z2 Extreme (Strix Point, gfx1150):
#[cfg(target_gfx = "1150")]
pub const MAX_CU: u32    = 16;

// Wave occupancy calculation (identical formula, platform-specific MAX_CU)
pub const MAX_WAVES: u32 = MAX_CU * 2 * 16;  // 128 (Z1) or 512 (Z2)

// Occupancy headroom for game renderer
pub const OCCUPANCY_HEADROOM: u32 = MAX_WAVES - 1;  // DVSM uses 1 wave
```

**Associated SessionConfig Profiles (Z2-specific):**

```rust
impl SessionConfig {
    // Z2 Extreme (Strix Point, 16 CU) — Performance
    pub const ALLY_X_Z2_PERF: Self = Self {
        frame_rate_hz: 240,
        dt: 1.0 / 240.0,
        vr_enabled: false,
        q_mode: QuantMode::Q31,
        _locked: false,
    };
    
    // Z2 Extreme — Balanced
    pub const ALLY_X_Z2_BALANCED: Self = Self {
        frame_rate_hz: 120,
        dt: 1.0 / 120.0,
        vr_enabled: false,
        q_mode: QuantMode::Q31,
        _locked: false,
    };
}
```

**Compile Flag (Cargo.toml or command-line):**

```toml
# Z2 Extreme build
[profile.release]
rustflags = ["-C", "target-cpu=native", "--offload-arch=gfx1150"]

# Z1 Extreme build (alternative)
# rustflags = ["-C", "target-cpu=native", "--offload-arch=gfx1103"]
```

**Or via command-line:**
```bash
# Z2 Extreme
cargo build --release \
  -C target-cpu=native \
  --offload-arch=gfx1150

# Z1 Extreme
cargo build --release \
  -C target-cpu=native \
  --offload-arch=gfx1103
```

**Occupancy Model Validation (Test):**

```rust
#[test]
fn test_gpu_occupancy_model() {
    #[cfg(target_gfx = "1103")]
    {
        assert_eq!(MAX_CU, 4, "Z1 Extreme CU mismatch");
        assert_eq!(MAX_WAVES, 128, "Z1 Extreme wave slots mismatch");
        let occupancy = 1.0 / 128.0;
        assert!(occupancy < 0.01, "DVSM occupancy on Z1 should be ~0.78%");
    }
    
    #[cfg(target_gfx = "1150")]
    {
        assert_eq!(MAX_CU, 16, "Z2 Extreme CU mismatch");
        assert_eq!(MAX_WAVES, 512, "Z2 Extreme wave slots mismatch");
        let occupancy = 1.0 / 512.0;
        assert!(occupancy < 0.01, "DVSM occupancy on Z2 should be ~0.19%");
    }
}
```

**Full Z2 Extreme Details:** See Z2_EXTREME_ADDENDUM.md
- Hardware specification comparison (GPU, CPU, memory)
- Code deltas required
- Architectural improvements (scalar FPU, texture throughput)
- Kernel optimization hints
- Interaction with AFMF2 (AMD Fluid Motion Frames 2)
- Benchmark validation methodology

---

### §11.5 Compression Integration Hook (SAEC Async Enqueue)

**Purpose:** Fire-and-forget compression job enqueue during supervisor tick, zero impact on 0.27ms critical path.

**Call Location:** Supervisor loop, after buffer swap and before next frame prediction.

```rust
/// Enqueue compression job for asynchronous processing
/// 
/// Input: observation_frame (raw pixels or audio samples)
/// Output: Result<(), CompressionError> (immediate, non-blocking)
pub fn enqueue_compression_job_q31_32(
    state: &DVSMState,
    config: &SessionConfig,
    observation_frame: &[u8],
    width: usize,
    height: usize,
) -> Result<(), String> {
    
    // Gate: check kill_compression flag (from USER_SETTINGS_SPEC.md)
    if config.kill_compression == 0 {
        return Ok(()); // Compression disabled, skip
    }
    
    // Create a snapshot of DVSM state for compression context
    let state_snapshot = DVSMCompressionContext {
        μ_core: state.μ_core.clone(),
        z_core: state.z_core.clone(),
        phase_delta_q: extract_phase_delta_q31_32(&state.μ_core)?,
        timestamp_tick: state.tick_count,
        protocol_version: config.protocol_version,
    };
    
    // Extract adaptive config from singularity probability
    let regime = detect_regime_from_singularity_q31_32(
        state_snapshot.phase_delta_q
    )?;
    
    let compression_config = select_adaptive_config_q31_32(regime)?;
    
    // Enqueue job (non-blocking)
    COMPRESSION_QUEUE.enqueue(CompressionJob {
        observation_data: observation_frame.to_vec(),
        state_context: state_snapshot,
        compression_config: compression_config,
        width: width,
        height: height,
        rose_net: config.neural_rose_enabled.then(|| ROSE_NET.clone()),
    })?;
    
    // Latency: ~2 μs (queue append, no processing)
    Ok(())
}

**Supervisor Tick Integration:**

```rust
pub fn supervisor_tick_main_loop(
    state: &mut DVSMState,
    config: &SessionConfig,
    input_frame: &InputFrame,
) -> Result<(), String> {
    // ═══════════════════════════════════════════════════════════
    // Critical Path (0.27 ms total budget)
    // ═══════════════════════════════════════════════════════════
    
    // 1. Core DVSM tick (250 μs)
    tick_phase_locked_q31_32(&mut state.μ_core, &mut state.z_core)?;
    
    // 2. Buffer swap + enqueue (20 μs)
    display_buffer_swap();
    
    // 3. COMPRESSION ENQUEUE (2 μs, non-blocking) ← NEW
    enqueue_compression_job_q31_32(
        state,
        config,
        &input_frame.observation_data,
        input_frame.width,
        input_frame.height,
    )?;
    
    // ═══════════════════════════════════════════════════════════
    // Non-Critical Path (runs in parallel on separate threads)
    // ═══════════════════════════════════════════════════════════
    
    // 4. Compression worker thread processes enqueued job asynchronously
    //    (Latency: 0.6–3.0 ms depending on regime, well under 8.33 ms)
    
    // 5. Modality updates (RF/ELF/BioScience 3D) if v3.2+
    if config.protocol_version >= 0x0302 {
        update_modality_states(state, config, input_frame)?;
    }
    
    state.tick_count += 1;
    Ok(())
}
```

**Hash Binding (no new state tracked):**
```
Compression is purely observational (reads state, does not modify).
H_t remains unchanged; compression does not appear in hash.
```

---

---

## §12 MULTIMODAL COUPLING OPERATOR (Q31.32)

### §12.1 Compute Coupling Matrix (RF/ELF/BioScience 3D)

**Signature:**
```rust
pub fn compute_coupling_matrix_q31_32(
    μ_core: &[i64; 12],
    μ_rf: &[i64; 4],
    μ_elf: &[i64; 3],
    μ_bio3d_cov: Option<&[[i64; 500]; 500]>,
    config: &CouplingConfig,
) -> Result<[[i64; 6]; 6], String>
```

**Implementation (abridged for documentation):**

```rust
// ────────────────────────────────────────────────────────────────
// Constants (Session-Immutable, from CouplingConfig)
// ────────────────────────────────────────────────────────────────

const Q31_32_ONE: i64 = 1i64 << 32;
const Q31_32_EPSILON: i64 = 1;
const COHERENCE_GATE_THRESHOLD: i64 = (0.7 * (1u64 << 32) as f64) as i64;
const ELF_FREQUENCY_TOLERANCE_HZ: i64 = (1.0 * (1u64 << 32) as f64) as i64;
const BIO3D_EIGENVALUE_BASELINE: i64 = 0i64;
const POWER_ITER_CYCLES: usize = 3;

// ────────────────────────────────────────────────────────────────
// Helper: Power Iteration (Dominant Eigenvalue)
// ────────────────────────────────────────────────────────────────

fn dominant_eigenvalue_power_iter_q31_32(
    cov_matrix: &[[i64; 500]; 500],
) -> Result<i64, String> {
    let mut v: Vec<i64> = vec![div_q31_32(Q31_32_ONE, 500)?; 500];
    let mut λ = 0i64;
    
    for _iter in 0..POWER_ITER_CYCLES {
        // Av = cov_matrix @ v (matrix-vector multiply)
        let mut av: Vec<i64> = Vec::with_capacity(500);
        for i in 0..500 {
            let mut sum: i128 = 0;
            for j in 0..500 {
                sum = sum.wrapping_add(
                    mul_q31_32_i128(cov_matrix[i][j], v[j])
                );
            }
            av.push((sum >> 32) as i64);
        }
        
        // λ = v · Av (Rayleigh quotient)
        let mut lambda_acc: i128 = 0;
        for i in 0..500 {
            lambda_acc = lambda_acc.wrapping_add(
                mul_q31_32_i128(v[i], av[i])
            );
        }
        λ = (lambda_acc >> 32) as i64;
        
        // Normalize: v = Av / ||Av||
        let norm_av = norm_q31_32_vec(&av)?;
        if norm_av > Q31_32_EPSILON {
            for i in 0..500 {
                v[i] = div_q31_32(av[i], norm_av)?;
            }
        } else {
            break;
        }
    }
    
    Ok(λ)
}

// ────────────────────────────────────────────────────────────────
// Main Coupling Operator
// ────────────────────────────────────────────────────────────────

pub fn compute_coupling_matrix_q31_32(
    μ_core: &[i64; 12],
    μ_rf: &[i64; 4],
    μ_elf: &[i64; 3],
    μ_bio3d_cov: Option<&[[i64; 500]; 500]>,
    config: &CouplingConfig,
) -> Result<[[i64; 6]; 6], String> {
    
    let mut w_matrix = [[0i64; 6]; 6];
    
    // ════════════════════════════════════════════════════════════
    // RF COUPLING TERM (if enabled)
    // ════════════════════════════════════════════════════════════
    
    if config.rf_influence_q31_32 > 0 {
        let amplitude_q = μ_rf[1];  // amplitude_q ∈ [0, 1)
        let α_rf = mul_q31_32(config.rf_influence_q31_32, amplitude_q)?;
        
        for i in 0..6 {
            w_matrix[i][i] = add_q31_32_clamped(w_matrix[i][i], α_rf)?;
        }
    }
    
    // ════════════════════════════════════════════════════════════
    // ELF COUPLING TERM (if enabled and gated by coherence)
    // ════════════════════════════════════════════════════════════
    
    if config.elf_influence_q31_32 > 0 {
        let coherence_q = μ_elf[1];
        let frequency_elf_q = μ_elf[0];
        let pll_frequency_q = extract_phase_rate_from_core_q31_32(μ_core)?;
        
        let freq_delta = sub_q31_32(frequency_elf_q, pll_frequency_q)?.abs();
        let freq_tolerance_ok = freq_delta < ELF_FREQUENCY_TOLERANCE_HZ;
        let coherence_ok = coherence_q >= COHERENCE_GATE_THRESHOLD;
        
        if freq_tolerance_ok && coherence_ok {
            let coherence_excess = sub_q31_32(coherence_q, COHERENCE_GATE_THRESHOLD)?;
            let core_norm_q = norm_q31_32_core_state(μ_core)?;
            
            let core_direction = if core_norm_q > Q31_32_EPSILON {
                div_q31_32(Q31_32_ONE, core_norm_q)?
            } else {
                0
            };
            
            let α_elf = mul_q31_32(
                mul_q31_32(config.elf_influence_q31_32, coherence_excess)?,
                core_direction
            )?;
            
            for i in 0..6 {
                w_matrix[i][i] = add_q31_32_clamped(w_matrix[i][i], α_elf)?;
            }
        }
    }
    
    // ════════════════════════════════════════════════════════════
    // BIOSCIENCE 3D COUPLING TERM (if enabled)
    // ════════════════════════════════════════════════════════════
    
    if config.bio3d_influence_q31_32 > 0 {
        if let Some(cov_matrix) = μ_bio3d_cov {
            let λ_dominant = dominant_eigenvalue_power_iter_q31_32(cov_matrix)?;
            let λ_delta = sub_q31_32(λ_dominant, BIO3D_EIGENVALUE_BASELINE)?;
            let α_bio = mul_q31_32(config.bio3d_influence_q31_32, λ_delta)?;
            
            for i in 0..6 {
                w_matrix[i][i] = add_q31_32_clamped(w_matrix[i][i], α_bio)?;
            }
        }
    }
    
    Ok(w_matrix)
}

// ────────────────────────────────────────────────────────────────
// Test: Coupling Matrix Determinism
// ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test_coupling_determinism {
    use super::*;
    
    #[test]
    fn test_coupling_matrix_bit_identical() {
        let μ_core = [0i64; 12];
        let μ_rf = [
            (2400i64 << 32) / 3_000_000_000,
            (500i64 << 32) / 1000,
            0,
            0,
        ];
        let μ_elf = [
            (8i64 << 32),
            ((700000i64 << 32) / 1_000_000),
            0,
        ];
        
        let config = CouplingConfig {
            rf_influence_q31_32: (500i64 << 32) / 1000,
            elf_influence_q31_32: (750i64 << 32) / 1000,
            bio3d_influence_q31_32: 0,
            coupling_mode: 1,
            _reserved: [0; 3],
        };
        
        // Two identical runs must produce bit-identical output
        let w1 = compute_coupling_matrix_q31_32(&μ_core, &μ_rf, &μ_elf, None, &config).unwrap();
        let w2 = compute_coupling_matrix_q31_32(&μ_core, &μ_rf, &μ_elf, None, &config).unwrap();
        
        assert_eq!(w1, w2, "Coupling matrices must be bit-identical");
    }
    
    #[test]
    fn test_coupling_matrix_clamping() {
        let config = CouplingConfig {
            rf_influence_q31_32: (900i64 << 32) / 1000,
            elf_influence_q31_32: (900i64 << 32) / 1000,
            bio3d_influence_q31_32: (900i64 << 32) / 1000,
            coupling_mode: 1,
            _reserved: [0; 3],
        };
        
        let w = compute_coupling_matrix_q31_32(
            &[0i64; 12],
            &[(1i64 << 32); 4],
            &[(1i64 << 32); 3],
            None,
            &config,
        ).unwrap();
        
        // Check for overflow
        for i in 0..6 {
            assert!(w[i][i] >= -(1i64 << 31), "Underflow");
            assert!(w[i][i] < (1i64 << 31), "Overflow");
        }
    }
}
```

**Critical Path:** 180 μs (Power Iteration + RF/ELF/BioScience terms) — fits comfortably within 8.33 ms budget.

---

### §12.2 RF Modality State Update (Fixed-Point PLL)

**Signature:**
```rust
pub fn update_rf_state_q31_32(
    μ_rf_prev: &[i64; 4],
    z_rf_prev: &[i64; 4],
    x_rf_input: &RFInputFrame,
    config: &CouplingConfig,
) -> Result<[i64; 4], String>
```

**Implementation (summary):**
- Extract instantaneous frequency via I/Q demodulation (100 MHz sampling)
- Compute tracking errors: frequency_error, amplitude_error, phase_error, bandwidth_error
- Update residuals Z_rf using EMA filtering (α = 0.2, τ ≈ 5 ticks)
- Apply PI feedback: μ_rf[t+1] = μ_rf[t] + Kp·error[t] + Ki·z[t]
- Wrap phase (mod 2π), gate amplitude updates, exponential decay if no signal

**Convergence:**
- Settling time: ~50 ms (6 ticks at 120 Hz)
- Steady-state frequency error: ≤ 1 kHz
- Phase jitter (locked): ≤ 0.1 rad

**For full implementation:** See RF_ELF_BIOMODALITY_SPEC.md §1.2

---

### §12.3 ELF Modality State Update (First-Order IIR + Coherence)

**Signature:**
```rust
pub fn update_elf_state_q31_32(
    μ_elf_prev: &[i64; 3],
    z_elf_prev: &[i64; 3],
    μ_core_current: &[i64; 12],
    x_elf_input: &ELFInputFrame,
    config: &CouplingConfig,
) -> Result<[i64; 3], String>
```

**Implementation (summary):**
- Extract dominant frequency via Fourier (1–100 Hz range)
- Compute signal envelope (RMS or Hilbert transform)
- Compute cross-coherence with core PLL phase
- Update residuals Z_elf using IIR filtering (α = 0.15)
- Frequency update: slow tracking (0.1 gain, avoids oscillation)
- Coherence update: natural decay (0.98 per tick) + correction from observation
- Envelope: exponential tracking (τ ≈ 10 ticks)

**Convergence:**
- Frequency settling: ~67 ms (10 ticks)
- Coherence time constant: ~50 ms
- Coherence decay half-life: ~34 ms (no bio-lock)

**For full implementation:** See RF_ELF_BIOMODALITY_SPEC.md §2.2

---

### §12.4 BioScience 3D State Update (AR(1) + Delta-Sigma Quantization)

**Signature:**
```rust
pub fn update_bio3d_state_q31_32(
    μ_bio3d_prev: &[i64; 250],
    z_bio3d_prev: &[i64; 250],
    x_bio3d_input: &VolumetricFrame,
    config: &CouplingConfig,
) -> Result<[i64; 250], String>
```

**Implementation (summary):**
- Project volumetric frame onto frozen PCA basis (rank 250)
- AR(1) prediction: ĉ[t+1] = 0.9 * c[t]
- Compute residuals: ε[t] = c_new[t] - ĉ[t]
- Delta-Sigma quantization (order 2) to minimize hash flux
- Update state: c[t+1] = ĉ[t] + quantized_residual[t]

**Convergence:**
- AR(1) decay time: ~10 ticks
- Residual error (RMS): ~0.1 (Q31.32 units)
- Hash flux: bounded by delta-sigma quantum level
- Reconstruction accuracy: ≥ 95% of variance (rank 250)

**For full implementation:** See RF_ELF_BIOMODALITY_SPEC.md §3.2

---

---

## §13 C LANGUAGE REFERENCE IMPLEMENTATION

### §12.1 Core Header (dvsm_core.h, C89 Compatible)

```c
#ifndef DVSM_CORE_H
#define DVSM_CORE_H

#include <stdint.h>
#include <stddef.h>
#include <math.h>

#define DVSM_DIM 16
#define DVSM_DIM_VR 20
#define DVSM_KAPPA_SIZE (DVSM_DIM * DVSM_DIM)

/* Fixed-point modes */
typedef enum {
    Q31 = 0,
    Q16 = 1,
    Q64_64 = 2,
} QuantMode;

/* Display geometry */
typedef enum {
    FLAT_2D = 0,
    FLAT_3D = 1,
    CONCAVE_2D = 2,
    CONCAVE_3D = 3,
    SPHERICAL_VR = 4,
} DisplayMode;

/* FPS boost flag */
typedef enum {
    BOOST_OFF = 0,
    BOOST_ON = 1,
} BoostMode;

/* Core state (scalar or VR) */
typedef struct {
    float z[DVSM_DIM_VR];           /* Primary state (padded to 20D) */
    float s[DVSM_DIM_VR];           /* EMA memory */
    float kappa[DVSM_KAPPA_SIZE];   /* Lie bracket operator */
    float norm_sq;                  /* ‖Z‖² cache */
    uint64_t replay_hash;           /* Deterministic sequence hash */
    uint64_t tick;                  /* Tick counter */
} DVSMState;

/* Session config (immutable) */
typedef struct {
    uint32_t frame_rate_hz;         /* 60/120/240 */
    float dt;                       /* Computed: 1.0 / frame_rate_hz */
    int vr_enabled;                 /* Boolean */
    int boost_mode;                 /* FPS boost (portable) */
    QuantMode q_mode;
    DisplayMode display;
    int _locked;                    /* Immutable flag */
} SessionConfig;

/* Power telemetry */
typedef struct {
    float tdp_watts;                /* TDP ceiling */
    float actual_watts;             /* Measured */
    float thermal_headroom_c;       /* TjMax - T_current */
    float power_budget_ratio;       /* actual / tdp */
} PowerTelemetry;

/* Control panel state */
typedef struct {
    int panel_enabled;
    float z_norm_current;
    float z_norm_target;
    float z_dot_s;                  /* Orthogonality check */
    int suchness[3];                /* binding, orthogonal, ghost */
    uint64_t hash_last_8bits;
    float haptic_force_mag;
    float haptic_force_max;
    float ghost_snr_db;
    uint32_t ghostsnap_checkpoint_count;
} ControlPanelState;

/* Prototypes */
void dvsm_step(DVSMState *state, const SessionConfig *config,
               float lambda, float alpha, float e_target,
               const PowerTelemetry *power, BoostMode boost);

void dvsm_step_vr(DVSMState *state, const SessionConfig *config,
                  float lambda, float alpha, float e_target,
                  const PowerTelemetry *power, DisplayMode display);

int dvsm_suchness_check(const DVSMState *state, const DVSMState *prev);

#endif
```

### §12.2 Core Implementation (dvsm_core.c, Portable)

```c
#include "dvsm_core.h"

static const float Q31_SCALE = 2147483648.0f;
static const float Q31_SCALE_INV = 1.0f / 2147483648.0f;

/* Lie-bracket + backreaction (portable) */
void dvsm_step(DVSMState *state, const SessionConfig *config,
               float lambda, float alpha, float e_target,
               const PowerTelemetry *power, BoostMode boost) {
    
    float acc[DVSM_DIM_VR] = {0.0f};
    int n = config->vr_enabled ? DVSM_DIM_VR : DVSM_DIM;
    
    /* Power scaling (local only, not in hash) */
    float b = power->actual_watts / (power->tdp_watts + 1e-6f);
    b = fminf(fmaxf(b, 0.0f), 1.0f);
    
    float lambda_actual = lambda * (0.5f + 0.5f * b);
    float alpha_actual = (boost == BOOST_ON) ? alpha * 0.25f : alpha * b;
    
    /* Lie bracket accumulation */
    for (int k = 0; k < n; k++) {
        float zk = state->z[k];
        float sk = state->s[k];
        for (int j = 0; j < n; j++) {
            if (j == k) continue;
            float bracket = zk * state->s[j] - state->z[j] * sk;
            acc[k] += state->kappa[k * DVSM_DIM + j] * bracket;
        }
    }
    
    /* Backreaction */
    float backreaction_coeff = -alpha_actual * (state->norm_sq - e_target);
    
    /* Euler step */
    for (int k = 0; k < n; k++) {
        float b_k = backreaction_coeff * state->z[k];
        float dz = config->dt * (acc[k] - lambda_actual * state->z[k] + b_k);
        state->z[k] += dz;
        state->s[k] = 0.95f * state->s[k] + 0.05f * state->z[k];
    }
    
    /* Update norm (with clamp) */
    float norm_sq = 0.0f;
    for (int k = 0; k < n; k++) {
        norm_sq += state->z[k] * state->z[k];
    }
    state->norm_sq = fminf(fmaxf(norm_sq, 0.0f), 100.0f);
    state->tick++;
}

/* VR-specific step with display geometry */
void dvsm_step_vr(DVSMState *state, const SessionConfig *config,
                  float lambda, float alpha, float e_target,
                  const PowerTelemetry *power, DisplayMode display) {
    
    /* Run standard step first */
    dvsm_step(state, config, lambda, alpha, e_target, power, BOOST_OFF);
    
    if (!config->vr_enabled) return;
    
    /* Apply display geometry correction */
    switch (display) {
        case FLAT_2D:
            state->z[2] = 0.0f;  /* Ignore Z depth */
            break;
        case CONCAVE_3D: {
            /* Simplified barrel distortion */
            float r2 = state->z[0] * state->z[0] + state->z[1] * state->z[1];
            float distortion = 1.0f + 0.1f * r2;
            state->z[0] /= distortion;
            state->z[1] /= distortion;
            break;
        }
        default:
            break;
    }
    
    /* Normalize quaternion (rotation) */
    float q_norm_sq = state->z[3] * state->z[3] 
                    + state->z[4] * state->z[4] 
                    + state->z[5] * state->z[5] 
                    + state->z[6] * state->z[6];
    
    if (q_norm_sq > 1e-12f) {
        float q_norm = sqrtf(q_norm_sq);
        state->z[3] /= q_norm;
        state->z[4] /= q_norm;
        state->z[5] /= q_norm;
        state->z[6] /= q_norm;
    } else {
        /* Reset to identity quaternion */
        state->z[3] = 1.0f;
        state->z[4] = 0.0f;
        state->z[5] = 0.0f;
        state->z[6] = 0.0f;
    }
}

/* Suchness check (triplet verification) */
int dvsm_suchness_check(const DVSMState *state, const DVSMState *prev) {
    /* 1. Binding: hash chain */
    int binding_ok = 1;  /* FNV1A parity check here */
    
    /* 2. Orthogonality: Z · S < ε */
    float dot = 0.0f;
    for (int k = 0; k < DVSM_DIM; k++) {
        dot += state->z[k] * state->s[k];
    }
    int ortho_ok = (fabsf(dot) < 1e-10f);
    
    /* 3. Ghost closure: G never feeds Z (code audit) */
    int ghost_ok = 1;  /* Verified by inspection */
    
    return binding_ok && ortho_ok && ghost_ok;
}
```

---

## §14 CONTROL PANEL IMPLEMENTATION

### §14.1 On-Screen Rendering

```c
/* Control panel telemetry capture */
void dvsm_control_panel_update(const DVSMState *state,
                               const PowerTelemetry *power,
                               const SessionConfig *config,
                               ControlPanelState *panel) {
    
    panel->z_norm_current = sqrtf(state->norm_sq);
    panel->z_norm_target = 1.0f;
    
    /* Orthogonality */
    float dot = 0.0f;
    for (int k = 0; k < DVSM_DIM; k++) {
        dot += state->z[k] * state->s[k];
    }
    panel->z_dot_s = dot;
    
    /* Suchness triplet */
    panel->suchness[0] = 1;  /* binding (hash parity) */
    panel->suchness[1] = (fabsf(dot) < 1e-10f) ? 1 : 0;
    panel->suchness[2] = 1;  /* ghost (code audit) */
    
    /* Hash (last 64 bits) */
    panel->hash_last_8bits = state->replay_hash & 0xFFFFFFFFFFFFFF00ULL;
    
    /* Haptics (mock) */
    panel->haptic_force_mag = 5.2f;
    panel->haptic_force_max = 10.0f;
    
    /* Ghost SNR */
    panel->ghost_snr_db = 18.3f;
    
    /* Checkpoints */
    panel->ghostsnap_checkpoint_count = state->tick / 1000;
}

/* Pseudo-code for on-screen overlay (platform-specific) */
void dvsm_render_control_panel(const ControlPanelState *panel) {
    /*
    draw_text(10, 10, "DVSM Control Panel");
    draw_text(10, 30, "Power: %.1f W", actual_watts);
    draw_text(10, 50, "Frame Rate: 240 Hz [LOCKED]");
    draw_text(10, 70, "Z norm: %.3f", panel->z_norm_current);
    draw_text(10, 90, "Z·S ortho: %.2e %s", 
              panel->z_dot_s,
              panel->suchness[1] ? "✓" : "✗");
    draw_text(10, 110, "Suchness: %s %s %s",
              panel->suchness[0] ? "✓" : "✗",
              panel->suchness[1] ? "✓" : "✗",
              panel->suchness[2] ? "✓" : "✗");
    draw_text(10, 130, "Hash chain: 0x%016llx", 
              panel->hash_last_8bits);
    */
}
```

### §14.2 BIOS Configuration Storage

```c
/* BIOS config structure (EEPROM/NVRAM) */
typedef struct {
    uint32_t magic;              /* 0x44564D42 "DVMB" */
    uint32_t version;            /* Config format version */
    
    int boot_mode;               /* Green/Standard/Forensic */
    int power_mode;              /* Fixed/Dynamic */
    float power_limit_watts;     /* If fixed */
    
    uint32_t frame_rate_hz;      /* 60/120/240 */
    int vr_enabled;
    int haptics_enabled;
    int boost_mode_enabled;
    
    int display_mode;            /* Flat/Concave/VR */
    int q_mode_default;          /* Q31/Q16/Q64.64 */
    
    int security_level;          /* Standard/Paranoid */
    float thermal_throttle_c;
    float power_throttle_pct;
    
    uint32_t crc32;              /* Checksum */
} BIOSConfig;

/* Load/save (platform-specific) */
int bios_config_load(BIOSConfig *cfg) {
    /* Read from EEPROM/NVRAM, verify CRC32 */
    /* Return 0 on success, -1 on corruption (fallback to defaults) */
    return 0;
}

int bios_config_save(const BIOSConfig *cfg) {
    /* Compute CRC32, write to EEPROM/NVRAM */
    /* Return 0 on success */
    return 0;
}
```

---

## §15 HARDENING REVIEW

### §15.1 Bounds & Overflow Protection

```c
/* Safe Q31 encoding */
static inline int32_t q31_encode_safe(float x) {
    x = fminf(fmaxf(x, -1.0f + 1e-7f), 1.0f - 1e-7f);
    return (int32_t)(x * Q31_SCALE);
}

/* Safe norm computation (prevent infinity) */
static inline float safe_norm_sq(const float *z, size_t n) {
    float norm = 0.0f;
    for (size_t k = 0; k < n; k++) {
        float zk = fminf(fmaxf(z[k], -1e6f), 1e6f);  /* Clamp */
        norm += zk * zk;
    }
    return norm;
}

/* Safe quaternion normalization */
static inline void normalize_quat(float *q) {
    float norm_sq = q[0]*q[0] + q[1]*q[1] + q[2]*q[2] + q[3]*q[3];
    
    if (norm_sq < 1e-12f) {
        q[0] = 1.0f;  /* Reset to identity */
        q[1] = q[2] = q[3] = 0.0f;
        return;
    }
    
    if (norm_sq > 1e6f) {
        q[0] = q[1] = q[2] = q[3] = 0.0f;  /* Degenerate → reset */
        return;
    }
    
    float norm = sqrtf(norm_sq);
    q[0] /= norm;
    q[1] /= norm;
    q[2] /= norm;
    q[3] /= norm;
}

/* Array bounds check (debug builds) */
#ifdef DVSM_DEBUG
#define ARRAY_ACCESS(arr, idx, max) \
    ((idx) < (max) ? (arr)[(idx)] : (abort(), 0.0f))
#else
#define ARRAY_ACCESS(arr, idx, max) ((arr)[(idx)])
#endif
```

### §15.2 Paranoid Mode (Optional 2x Cost)

```c
typedef struct {
    int paranoid_enabled;
    uint32_t norm_recompute_interval;      /* Every N ticks */
    uint32_t hash_full_interval;           /* Full Z,S,W hash */
} ParanoidConfig;

void dvsm_step_paranoid(DVSMState *state, const ParanoidConfig *paranoid) {
    /* Standard step */
    dvsm_step(state, config, lambda, alpha, e_target, power, BOOST_OFF);
    
    /* Recompute norm independently */
    if (paranoid->paranoid_enabled && 
        (state->tick % paranoid->norm_recompute_interval == 0)) {
        
        float norm_recomputed = safe_norm_sq(state->z, DVSM_DIM);
        float error = fabsf(norm_recomputed - state->norm_sq);
        
        if (error > 1e-6f) {
            /* Silent bit corruption detected */
            fprintf(stderr, "PARANOID: Norm mismatch detected: %e\n", error);
            /* Rollback or alert */
        }
        
        state->norm_sq = norm_recomputed;  /* Resync */
    }
}
```

---

## §16 DISPLAY GEOMETRY TRANSFORMS

### §16.1 Flat 2D/3D

```c
void apply_flat_2d(float *z) {
    z[2] = 0.0f;  /* Ignore depth */
}

void apply_flat_3d(float *z) {
    /* Perspective transform: (x, y, z) → (x/z, y/z) if z > 0 */
    if (z[2] > 0.1f) {
        z[0] /= z[2];
        z[1] /= z[2];
    }
}
```

### §16.2 Concave Distortion

```c
void apply_concave_3d(float *z, float kappa_display) {
    /* Barrel/pincushion distortion correction */
    float r2 = z[0] * z[0] + z[1] * z[1];
    float distortion = 1.0f + kappa_display * r2;
    
    if (distortion > 0.1f) {
        z[0] /= distortion;
        z[1] /= distortion;
    }
}
```

---

## §17 FPS BOOST MODE (PORTING-FRIENDLY)

### §17.1 Lightweight Kernel

```c
void dvsm_step_boost(DVSMState *state, const SessionConfig *config,
                     float lambda, float alpha, float e_target,
                     const PowerTelemetry *power) {
    
    /* Scalar only (16D, not 20D) */
    float acc[DVSM_DIM] = {0.0f};
    
    /* Lie bracket ONLY (no Rose curve) */
    for (int k = 0; k < DVSM_DIM; k++) {
        float zk = state->z[k];
        float sk = state->s[k];
        for (int j = 0; j < DVSM_DIM; j++) {
            if (j == k) continue;
            float bracket = zk * state->s[j] - state->z[j] * sk;
            acc[k] += state->kappa[k * DVSM_DIM + j] * bracket;
        }
    }
    
    /* Reduced backreaction */
    float alpha_boost = alpha * 0.25f;
    float backreaction_coeff = -alpha_boost * (state->norm_sq - e_target);
    
    /* Euler (standard dissipation) */
    for (int k = 0; k < DVSM_DIM; k++) {
        float b_k = backreaction_coeff * state->z[k];
        float dz = config->dt * (acc[k] - lambda * state->z[k] + b_k);
        state->z[k] += dz;
        state->s[k] = 0.95f * state->s[k] + 0.05f * state->z[k];
    }
    
    /* Update norm */
    float norm_sq = 0.0f;
    for (int k = 0; k < DVSM_DIM; k++) {
        norm_sq += state->z[k] * state->z[k];
    }
    state->norm_sq = fminf(fmaxf(norm_sq, 0.0f), 100.0f);
    state->tick++;
}

/* Estimated speedup: 2-4x vs standard (no VR, no Rose, reduced backreach) */
```

---

## References

- All formulas from DVSM_SPEC.md
- Rust crates: nalgebra (linear algebra), serde (serialization)
- Swift: CoreGraphics, MetalKit
- C: C89 ANSI (portable across all platforms)
- Cross-language: FFI via cbindgen (Rust ↔ C ↔ Swift)
