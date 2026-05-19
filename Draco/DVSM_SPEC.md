# DVSM-Core Specification (Consolidated)
**Author:** Daniel J. Dillberg | **Date:** 2026-05-19 | **Status:** IMMUTABLE CONTRACT

---

## §A MATHEMATICAL CONTRACT

### §A.1 State Tuple (Fully Specified)

```
σ_t = (μ_t, Z_t, S_t, W_t, κ, λ, α, E_target, Q_mode, frame_rate_hz, 
       vr_enabled, neural_enabled, protocol_version)

Immutable (protocol-frozen, session-locked):
  κ ∈ R^{n×n}                  antisymmetric Lie-bracket operator
  λ ∈ R⁺                       dissipation coefficient (base)
  α ∈ R⁺                       backreaction strength (base)
  E_target ∈ R⁺                target norm (typically 1.0)
  Q_mode ∈ {Q31, Q16, Q64.64, custom}  fixed-point mode
  frame_rate_hz ∈ {60, 120, 240}      HARD FIX (immutable for session)
  vr_enabled ∈ {bool}          3D/VR spatial state toggle
  neural_enabled ∈ {bool}      Rose curve + neural support toggle
  protocol_version ∈ u64       immutable version tag

Mutable (state-evolved):
  μ_t                          input commands (observed, causal)
  Z_t ∈ R^n                    primary state (Lie-bracket evolved)
    - n=16 standard (scalar state)
    - n=20 with VR (spatial+haptics)
  S_t ∈ R^n                    EMA memory (causal, irreversible)
  W_t ∈ St(n,r)                Stiefel manifold basis (QR-orthonormal)

Session-computed (derived):
  dt = 1.0 / frame_rate_hz     time step (immutable after init)
  λ_actual = λ · (60 / frame_rate_hz)    scaled dissipation
  α_actual = α · (frame_rate_hz / 60)    scaled backreaction
```

### §A.2 Core Equation with Backreaction

```
dZ_k/dt = Σⱼ κ_{kj}(Z_k S_j − Z_j S_k) − λZ_k − α(‖Z‖² − E_target)·Z_k + Rose_k(Z, W_neural)

where:
  [·,·]_κ                  Lie bracket (antisymmetric coupling)
  λ                        dissipation (always decay norm)
  α                        backreaction (restores toward E_target)
  Rose_k(Z, W_neural)      optional Rose curve logic (see §A.5)

Euler Integration with State Boundary Clamping (§A.2b):
  Z_k^{raw} = Z_k + dt·(dZ_k/dt)  [raw Euler step]
  
  CLAMP (immediately after integration):
    Z_k^{clamped} = CLAMP(Z_k^{raw}, −2.0, +2.0)    [hard boundary, OR]
    Z_k^{clamped} = 2·tanh(Z_k^{raw}/2)             [soft clip, continuous]
  
  Z_k := Z_k^{clamped}  [update state with bounded value]

Purpose: Prevent NaN propagation, enforce state actor integrity, preserve H_t binding
```

### §A.2b State Boundary Clamping (NaN Prevention & H_t Integrity)

**Problem:** Euler integration can produce raw values outside physically valid range, leading to:
- NaN in backreaction computation (norm → infinity)
- Hash divergence (Z contains NaN → H_t unpredictable)
- Unrecoverable state corruption

**Solution:** Clamp Z_k immediately after Euler step, before norm/hash computation.

**Two Strategies:**

```
STRATEGY 1: Hard Boundary ([-2.0, +2.0])
  Z_k^{clamped} = max(-2.0, min(+2.0, Z_k^{raw}))
  
  Properties:
    ✓ O(1) operation (no transcendental functions)
    ✓ Deterministic (identical across platforms, languages)
    ✓ Preserves sign (natural direction of state)
    ✓ Prevents overflow in Q31 fixed-point (range [-1, 1) mapped to [-2, 2])
    ✗ Discontinuous derivative at ±2.0 (but acceptable for state boundary)
  
  Recommended for: Production (low cost, predictable)

STRATEGY 2: Soft Clip via tanh (Differentiable)
  Z_k^{clamped} = 2·tanh(Z_k^{raw} / 2)
  
  Properties:
    ✓ Smooth (continuous, C∞)
    ✓ Approaches ±2.0 asymptotically (never exceeds bounds)
    ✓ Naturally handles large deviations (e^x terms compress)
    ✓ Compatible with gradient-based learning (if neural extension added)
    ✗ O(log n) cost (transcendental function)
    ✗ Platform variance (tanh implementations differ by ~1e-7)
  
  Recommended for: Development, neural variants

Boundary Threshold Justification:
  ‖Z‖_∞ ≤ 2.0 implies ‖Z‖₂ ≤ √n·2.0
  For n=16: ‖Z‖₂ ≤ 8.0 (reasonable margin above E_target=1.0)
  For n=20 (VR): ‖Z‖₂ ≤ 8.94 (accommodates spatial + angular state)
```

**H_t Binding Preservation:**

```
Before clamp: Z_k^{raw} could be NaN
              → H_t = FNV1A(... ⊕ NaN ⊕ ...) = undefined or zero-filled
              → Hash unpredictable; peer divergence

After clamp:  Z_k ∈ [-2.0, +2.0] (bounded)
              → norm(Z) ∈ [0, √(n·4)] (finite, computable)
              → H_t = FNV1A_PARITY(...) = deterministic
              → Hash identical across peers
```

**Integration Location:**
```
Euler step:           Z_k += dt·(dZ_k/dt)
↓
NaN PREVENTION ← [CLAMP Z_k here]  ← Must be IMMEDIATELY after integration
↓
Norm computation:     norm_sq = Σ_k Z_k²
↓
Backreaction coeff:   b_coeff = -α·(norm_sq - E_target)
↓
Hash computation:     H_t = FNV1A_PARITY(μ_t ⊕ Z_t ⊕ ...)
↓
Suchness check:       Verify H_t binding
```

**Selection Rule:**
```
if deployment_mode = "production":
    use hard clamp ([-2.0, 2.0])  // fastest, most deterministic
else if deployment_mode = "paranoid":
    use soft clip (2·tanh(...))   // catches saturation events
else if deployment_mode = "neural" or "ml":
    use soft clip (gradient-compatible)
```

---

### §A.2c Temporal Phase-Lock Loop (Z2 Extreme Non-Linear PLL)

**Purpose:** Exploit Z2's 0.19% GPU occupancy via phase-locked state prediction anchored to GPU completion timestamps. Transforms reactive integrator into predictive state engine via DDR-mimicry.

**Core Mechanism:**

```
Rising Edge (Dispatch):   Measure Z state at GPU dispatch (t_d)
                          ↓
Prediction Integration:   Z_future = Z + ∫ V_manifold · τ_meas
                          where τ_meas = (t_c - t_d) / 1e9
                          ↓
Falling Edge (Completion): Measure actual GPU completion (t_c)
                           ↓
Phase Error Detection:    phase_delta = τ_meas − τ_nominal
                          ↓
Correction Pulse:         B_corrected = B_base · (1.0 + κ_sync · phase_delta)
                          ↓
State Anchoring:          Apply backreaction correction with scaled magnitude
                          ↓
Final Binding:            H_t = FNV1A_PARITY(..., τ_nominal)  [NOT τ_meas]
```

**Velocity Manifold (Lie-Bracket Evolution):**

```
V_manifold[k] = dZ_k/dτ
              = Σⱼ κ_{kj}(Z_k·S_j − Z_j·S_k) − λ·Z_k + Rose_k(Z, W_neural)

where:
  Lie-bracket momentum:    Σⱼ κ_{kj}(Z_k·S_j − Z_j·S_k)
  Dissipation:             −λ·Z_k
  Optional Rose modulation: Rose_k (neural or deterministic)
  NOTE: Backreaction B_k NOT included here; applied only in falling edge
```

**Phase Correction Dynamics:**

```
Rising Edge Step:
  Z_future[k] = Z_k + τ_meas · (V_manifold[k] − λ·Z_k + Rose_k)
  
  State Boundary Clamp (§A.2b):
    Z_future[k] := CLAMP(Z_future[k], −2.0, +2.0)

Falling Edge Correction:
  phase_delta = τ_meas − τ_nominal
  
  Proportional Sync Scaling (κ_sync = 0.25):
    α_sync = α_base · (1.0 + 0.25 · phase_delta).clamp(0.8, 1.2)
  
  Backreaction Pulse Magnitude (correction_scale = 4.0):
    B_k = −α_sync · (‖Z_future‖² − E_target) · Z_future[k]
    Correction[k] = 4.0 · B_k · τ_nominal
  
  Apply Correction with Clamping:
    Z_corrected[k] = (Z_future[k] + Correction[k]).clamp(−2.0, +2.0)
    
  EMA Update:
    S_k ← ema_beta · S_k + (1.0 − ema_beta) · Z_corrected[k]
```

**Hash Binding (Critical: τ_nominal only):**

```
H_t = FNV1A_PARITY(
    μ_t 
    ⊕ Z_t^{corrected}
    ⊕ S_t
    ⊕ W_t
    ⊕ κ 
    ⊕ λ 
    ⊕ α 
    ⊕ E_target 
    ⊕ Q_mode 
    ⊕ frame_rate_hz        ← NOMINAL dt, NOT τ_meas
    ⊕ vr_enabled
    ⊕ neural_enabled
    ⊕ protocol_version
)

CRITICAL: H_t binds τ_nominal (immutable), not τ_meas (jittery).
          This ensures peer consensus despite GPU latency variance.
```

**Telemetry & Stability:**

```
Phase Error EMA (for diagnostics):
  phase_error_ema = 0.95 · phase_error_ema + 0.05 · |phase_delta|
  
  Warning threshold: if phase_error_ema > 0.0002 s (0.2ms)
                      → GPU systematically underperforming or clock skew

Saturation Detection (paranoid mode):
  For each tick, count dimensions where |Z_k| ≥ 1.8
  saturation_rate = count / (dim · total_ticks)
  
  Alert: if saturation_rate > 0.001 (0.1%)
         → State manifold near boundary; possible numerical stress
```

**VR Quaternion Renormalization (Post-Correction):**

```
if vr_enabled:
    q_norm_sq = Z_3² + Z_4² + Z_5² + Z_6²
    
    if |q_norm_sq − 1.0| > 0.01:
        q_norm = √q_norm_sq
        Z_3 := Z_3 / q_norm
        Z_4 := Z_4 / q_norm
        Z_5 := Z_5 / q_norm
        Z_6 := Z_6 / q_norm
```

---

### §A.2d Portable C-Struct Specification (Hardware-Locked ABI)

**Purpose:** Define a canonical, ABI-stable memory layout that ensures byte-identical behavior across Windows, Linux, SteamOS, and firmware implementations.

**Design Principles:**
```
1. Language-Agnostic: C, C++, Rust, Python ctypes all see identical layout
2. Platform-Agnostic: x86_64, ARM64 (endianness handled at serialization)
3. Cache-Efficient: 64-byte alignment (one cache line)
4. Fixed-Point Only: No floats in hot path (determinism guarantee)
5. Extensible: Reserve fields for future protocol versions
```

**The Canonical Structure:**

```c
// Portable type alias (all platforms)
typedef int64_t  q31_32_t;   // Q31.32 fixed-point (31 int + 32 frac bits)
typedef int128_t q63_64_t;   // Q63.64 fixed-point (extended range)

// ABI-stable struct (alignment, packing, endianness agnostic)
#[repr(C, align(64))]  // Rust FFI
pub struct DVSMManifoldStateV31 {
    // === IMMUTABLE PROTOCOL HEADER ===
    protocol_version:     u64,       // 0x0301 (v3.1, frozen)
    sync_tier:            u8,        // 1=Proportional, 2=Gudermannian
    q_format:             u8,        // 0=Q31.32, 1=Q64.64
    vr_enabled:           u8,        // 0 or 1
    neural_enabled:       u8,        // 0 or 1
    _reserved_header:     [u8; 4],   // Future use
    
    // === IMMUTABLE BACKREACTION COEFFICIENTS (Q31.32) ===
    alpha_base:           q31_32_t,  // Base backreaction strength
    kappa_sync:           q31_32_t,  // Phase-lock sync scale (0.25)
    e_target:             q31_32_t,  // Target norm squared (1.0)
    lambda:               q31_32_t,  // Dissipation coefficient
    
    // === MUTABLE STATE VECTORS ===
    z: [q31_32_t; 64],               // Primary state manifold (16D or 20D, padded to 64)
    s: [q31_32_t; 64],               // EMA memory (same dim)
    w: [q31_32_t; 64],               // Stiefel basis (optional, zeroed if unused)
    
    // === MUTABLE TELEMETRY & CONTROL ===
    frame_idx:            u64,       // Immutable frame counter
    hash_chain:           u64,       // FNV1A_PARITY(σ_t)
    phase_error_ema:      q31_32_t,  // Exponential moving average of |phase_delta|
    norm_sq_current:      q31_32_t,  // ‖Z‖² in Q31.32 (cached)
    
    // === FEATURE FLAGS (Runtime Control, No Restart) ===
    sync_tier_override:   u8,        // User override: 0=use config, 1=Proportional, 2=Gudermannian
    paranoid_mode:        u8,        // Soft-clip state clamping (0=hard clamp, 1=tanh)
    frame_gen_enabled:    u8,        // Frame generation parity (0 or 1)
    vrs_mask_enabled:     u8,        // Tile-based VRS (0 or 1)
    
    // === SAFETY & FALLBACK ===
    gd_inv_safety_threshold: f32,    // Singularity guard (units: gd_inv value, not scaled)
    gpu_hang_threshold_ms: u32,      // Fallback if phase_delta > this (units: ms)
    z_clamp_min:          q31_32_t,  // Hard clamp lower (-2.0)
    z_clamp_max:          q31_32_t,  // Hard clamp upper (+2.0)
    
    // === METADATA FOR DEBUG/TELEMETRY ===
    tick_count:           u64,       // Total ticks executed
    rollback_count:       u32,       // Number of suchness rollbacks
    safety_valve_triggers: u32,      // Number of fallbacks to Tier 1
    
    // === RESERVED (Future Protocol Versions) ===
    _reserved_future:     [u8; 64],  // Padding for v3.2+
}

// Size verification (compile-time)
const DVSM_STATE_SIZE: usize = core::mem::size_of::<DVSMManifoldStateV31>();
const DVSM_STATE_ALIGN: usize = core::mem::align_of::<DVSMManifoldStateV31>();

// Assertions
// DVSM_STATE_SIZE must be 768 bytes (12 × 64-byte cache lines)
// DVSM_STATE_ALIGN must be 64
```

**Memory Layout (Byte-Accurate):**

```
Offset (bytes)   Field                        Type        Size    Notes
───────────────────────────────────────────────────────────────────────
0                protocol_version             u64         8       Frozen
8                sync_tier                    u8          1       1 or 2
9                q_format                     u8          1       0 or 1
10               vr_enabled                   u8          1
11               neural_enabled               u8          1
12               _reserved_header             [u8; 4]     4
16               alpha_base                   q31_32_t    8       Q31.32
24               kappa_sync                   q31_32_t    8       Q31.32
32               e_target                     q31_32_t    8       Q31.32
40               lambda                       q31_32_t    8       Q31.32
48               z[0..64]                     [q31_32_t]  512     Primary state
560              s[0..64]                     [q31_32_t]  512     EMA memory
... [w vectors follow]
784              sync_tier_override           u8          1       No-restart
785              paranoid_mode                u8          1       No-restart
786              frame_gen_enabled            u8          1       No-restart
787              vrs_mask_enabled             u8          1       No-restart
... [safety thresholds follow]
```

**Serialization (C-Equivalence):**

```c
// C99 equivalent (portable across all languages)
typedef struct {
    uint64_t protocol_version;
    uint8_t sync_tier;
    uint8_t q_format;
    uint8_t vr_enabled;
    uint8_t neural_enabled;
    uint8_t _reserved_header[4];
    
    int64_t alpha_base;    // Q31.32
    int64_t kappa_sync;    // Q31.32
    int64_t e_target;      // Q31.32
    int64_t lambda;        // Q31.32
    
    int64_t z[64];         // Q31.32 state
    int64_t s[64];         // Q31.32 EMA
    int64_t w[64];         // Q31.32 basis
    
    uint64_t frame_idx;
    uint64_t hash_chain;
    int64_t phase_error_ema;  // Q31.32
    int64_t norm_sq_current;  // Q31.32
    
    uint8_t sync_tier_override;
    uint8_t paranoid_mode;
    uint8_t frame_gen_enabled;
    uint8_t vrs_mask_enabled;
    
    float gd_inv_safety_threshold;
    uint32_t gpu_hang_threshold_ms;
    int64_t z_clamp_min;   // Q31.32
    int64_t z_clamp_max;   // Q31.32
    
    uint64_t tick_count;
    uint32_t rollback_count;
    uint32_t safety_valve_triggers;
    
    uint8_t _reserved_future[64];
} DVSM_ManifoldState_v31;
```

**Hardware Specificity:**

```
Z1 Extreme (Phoenix, gfx1103):
  DIM = 16 (only first 16 of 64 z/s/w used, rest zeroed)
  Protocol version: 0x0301
  Typical: sync_tier = 1 (Proportional)

Z2 Extreme (Strix Point, gfx1150):
  DIM = 16 or 64 (configurable)
  Protocol version: 0x0301 (same, portable)
  Typical: sync_tier = 2 (Gudermannian, opt-in)
  
Windows/Linux (C++ wrapper):
  Can deserialize same binary, produce identical results
  Endianness: Big-endian systems swap at deserialization boundary
```

**Hardware Specificity:**

```
Z1 Extreme (RDNA 3, gfx1103):
  MAX_CU = 4
  MAX_WAVES = 128
  DVSM occupancy = 0.78%
  Expected phase_error convergence: < 0.5 ms

Z2 Extreme (RDNA 3.5, gfx1150):
  MAX_CU = 16
  MAX_WAVES = 512
  DVSM occupancy = 0.19%  ← Exploited by PLL
  Expected phase_error convergence: < 0.1 ms (4× better headroom)
```

**Validation Criteria (§9.6b Convergence Test):**

```
1000 frames with ±0.5ms random GPU jitter:

  ✓ avg_phase_error < 0.1 ms
  ✓ max_phase_error < 0.5 ms
  ✓ avg_norm_deviation < 0.01 (toward E_target)
  ✓ No suchness rollbacks (orthogonality maintained)
  ✓ Quaternion norm preserved (if VR enabled)
  ✓ Hash determinism with clamped state
```

---

### §A.2e Q31.32 Fixed-Point Encoding (Determinism Guarantee)

**Purpose:** Define bit-perfect conversion rules between floating-point and fixed-point representations. All arithmetic in the hot path uses integer operations only—no floats, no rounding errors.

**Q31.32 Format Definition:**

```
Range:     [-2^31, 2^31) ≈ [-2.147e9, +2.147e9]
ULP:       2^-32 ≈ 2.33e-10 (sub-nanosecond precision)
Encoding:  value_q31_32 = floor(value_float × 2^32)
Decoding:  value_float = value_q31_32 / 2^32

Example representations (exact):
───────────────────────────────────────────────────────────────
Value      Q31.32 Hex              Notes
───────────────────────────────────────────────────────────────
0.0        0x0000000000000000      Zero
1.0        0x0000000100000000      1 << 32
0.25       0x0000000040000000      κ_sync = 0.25 exactly
0.08       0x000000147AE147A7      α_base ≈ 0.08
-2.0       0xFFFFFFFE00000000      Hard clamp lower
+2.0       0x0000000200000000      Hard clamp upper
```

**Critical Property:**
```
ZERO-DRIFT GUARANTEE:
  All hot-path arithmetic is integer-only.
  Z evolution is DETERMINISTIC and byte-identical across:
    - Z2 Extreme (Linux, SteamOS)
    - Ally X (Windows, Linux)
    - PC (DX12/Vulkan wrapper)
    - Firmware (gfx1150 kernel)

Because int64 addition/multiplication is deterministic, there is NO
rounding error. The manifold will not drift or diverge over 100+ hours.
```

**Conversion Functions (Pseudocode):**

```
// Float → Q31.32
encode_q31_32(x: f64) → i64 = floor(x * 2^32)

// Q31.32 → Float
decode_q31_32(q: i64) → f64 = q / 2^32

// Q31.32 Multiplication (down-shift by 32)
mul_q31_32(a: i64, b: i64) → i64 = ((a × b) >> 32)

// For Gudermannian LUT (precomputed offline):
//   phase_normalized ∈ [-1.0, +1.0]
//   lut_index = (phase_normalized + 1.0) × 256 ∈ [0, 511]
//   α_sync = decode_q31_32(gd_lut[lut_index])
```

---

### §A.3 Hash Identity Binding (Immutable, Tautology-Critical)

```
H_t = FNV1A_PARITY(
    μ_t 
    ⊕ Z_t 
    ⊕ S_t 
    ⊕ W_t 
    ⊕ κ 
    ⊕ λ 
    ⊕ α 
    ⊕ E_target 
    ⊕ Q_mode 
    ⊕ frame_rate_hz
    ⊕ vr_enabled
    ⊕ neural_enabled
    ⊕ protocol_version
)

where FNV1A_PARITY(x) = (FNV1A(x), parity_bit(x))
  parity_bit = XOR of all bits in final hash

CRITICAL: frame_rate_hz is locked in hash → cannot change mid-session
         vr_enabled determines Z dimension (16D vs 20D)
```

**TAUTOLOGY (3-clause):**
1. **Binding**: H_t uniquely identifies σ_t (collision-resistant, frame rate locked)
2. **Orthogonality**: Z · S ≈ 0 (forward ⊥ residual, even with VR extension)
3. **Ghost closure**: G_t = Z_t − Π_W(Z_t) is pure residual (no feedback)

### §A.4 Fixed-Point Arithmetic (Determinism Under Sub-Zero SNR)

```
Q_mode options:

┌──────────────────────────────────────────────────────────┐
│ Mode      Range             ULP          Use Case        │
├──────────────────────────────────────────────────────────┤
│ Q31       [-1,1)            2^{-31}      primary (dev)   │
│ Q16       [-2^15,2^15)      2^{-16}      overflow        │
│ Q64.64    [-2^63,2^63)      2^{-64}      extended range  │
│ custom    user-defined      user-defined domain-specific │
└──────────────────────────────────────────────────────────┘

Encoding/decoding (deterministic, language-agnostic):

  Q31 encode:  x ∈ [-1, 1) → q = floor(x · 2^31) ∈ i32
  Q31 decode:  q → x = q / 2^31

  Q16 encode:  x ∈ R → q = floor(x · 2^16) ∈ i32
  Q16 decode:  q → x = q / 2^16

  Q64.64 encode:  x ∈ R → q = floor(x · 2^64) ∈ i128
  Q64.64 decode:  q → x = q / 2^64
  Range: ±9.223e18 (full f64 dynamic range)
  Precision: ~5.4e-20 (near f64 epsilon)

Sub-zero SNR support:
  When SNR < 0 dB (noise > signal):
    - Use Q16 for bandwidth-constrained scenarios
    - Use Q64.64 for precision-critical deep dynamics
    - Clamp Z_k to safe range per Q_mode
    - Monitor |Z_k| < ULP; trigger ghost rebirth if persistent

Adaptive Q-switching (optional):
  if ‖Z‖ > 10.0:
    Use Q64.64 (extended range)
  else if ‖Z‖ > 2.0:
    Use Q16 (wider than Q31)
  else:
    Use Q31 (default precision)
```

### §A.5 Rose Curve Logic (Optional Neural Extension)

```
Rose curve: r(θ) = a · cos(k·θ)  OR  r(θ) = a · sin(k·θ)

In Z-space (Stiefel basis W provides angle parametrization):

  Rose_k(Z, W_neural) = {
    0                      if neural_enabled = false
    β · a_learned · cos(k·θ_learned) · Z_k / (‖Z_k‖ + ε)   if enabled
  }

where:
  a_learned              learned petal amplitude (from neural net)
  k_learned              learned harmonic order (from neural net)
  θ_learned              angle extracted from W (via principal eigenvector)
  β ∈ [0, 1]             coupling strength (protocol-tunable)

Neural support (optional):
  - Input: Z_t, S_t, ‖Z_t‖², rolling variance
  - Output: (a_learned, k_learned) ∈ R² (deterministic, reproducible)
  - Architecture: 3-layer MLP, frozen weights (no online learning)
  - Initialization: seed from protocol_version (reproducibility)

Rose curve effect:
  - Adds harmonic modulation to Z evolution
  - Can suppress or enhance specific spectral modes
  - Coupled to basis adaptation W (allows resonance detection)
```

### §A.6 Dual Arithmetic: Orthogonality & Ghost Closure

```
Primary evolution:    Z_t+1 = Z_t + dt·dZ/dt
Memory (EMA):         S_t+1 = β·S_t + (1−β)·Z_t
Ghost residual:       G_t = Z_t − Π_W(Z_t)
Dual accumulator:     Dual_t+1 = γ·Dual_t + (1−γ)·G_t

Orthogonality check:  Z_t · S_t < ε_ortho  (typically ε = 1e-10)

Ghost closure rule:
  ✓ G_t computed but NEVER fed back to Z_t+1 or S_t+1
  ✓ G_t stored only via Dual EMA, never referenced in forward dynamics
  ✓ If ‖Z_t‖ < collapse_threshold, rebirth from S_t (not G_t)
```

### §A.7 GhostSnap: Bit-Creep Purging (Determinism Artifact)

```
Problem: Floating-point accumulated error in Z_t causes gradual bit-creep.
         Across 100k ticks, Z_k may diverge in low-order bits between peers.

Solution: GhostSnap checkpoint + purge rule.

GhostSnap(Z_t, interval=1000):
  Every interval ticks:
    1. Quantize Z_k to Q31 (force rounding)
    2. Recompute S_t from Z_t (EMA restart, keeping history)
    3. Compute H_t with fresh hash
    4. Store (tick_idx, H_t, Z_snap) as checkpoint
    5. Continue evolution with quantized Z

Bit-creep metric:
  creep_k = |Z_k(recomputed_from_Q31) − Z_k(float_accumulated)|
  if creep_k > threshold:
    trigger GhostSnap immediately
    log creep event (diagnostic)

Determinism effect:
  - Purges sub-ULP errors
  - Allows peer resync without full rollback
  - Checkpoints form determinism audit trail
```

### §A.8 FNV-1A Hashing with Parity (Spyware Defense)

```
FNV-1a (Fowler-Noll-Vo) properties:
  - Fast (no cryptographic overhead)
  - Deterministic (identical on all platforms)
  - Non-cryptographic (not for security-critical auth)

Implementation:

  FNV1A(data: [u8]) → u64:
    h := 0xcbf29ce484222325  // FNV offset basis
    for byte in data:
      h ^= byte
      h = h * 0x100000001b3   // FNV prime
    return h

  parity_bit = popcount(h) & 1  // XOR of all bits

  FNV1A_PARITY(x) = (h, parity_bit)
```

**Parity check (detect bit-flip injection):**
```
Verify(h, expected_parity):
  actual_parity = popcount(h) & 1
  return actual_parity == expected_parity
```

### §A.9 Cayley Projection (Spyware Rejection)

```
Cayley transform:
  Protect Z from unauthorized modification via parameterized projection.

  U = (I + A)^{-1}(I − A)    where A is skew-symmetric (A^T = −A)

In DVSM context:
  Detect injected state via:
    1. Compute A from (Z_claimed, Z_trusted) difference
    2. Check if A is approximately skew-symmetric
    3. If ‖A + A^T‖ > threshold: injection detected, reject Z_claimed
    4. Otherwise: apply Cayley correction Z_corrected = U · Z_trusted

Detection formula:
  ΔZ = Z_claimed − Z_trusted
  A = ΔZ ⊗ e_principal − e_principal ⊗ ΔZ / 2  (antisymmetrize)
  
  Skew test:
    skew_error = ‖A + A^T‖_F / (‖A‖_F + ε)
    if skew_error > 0.1:  # 10% tolerance
      REJECT (likely spyware injection)
    else:
      ACCEPT (natural drift)
```

### §A.9b 3D/VR Extension (Spatial State & Haptics)

```
Optional 3D/VR variant of Z (extends from 16D to higher dimension):

Z_spatial = {
  position: [x, y, z] ∈ R^3         (world-space coords)
  rotation: [qw, qx, qy, qz] ∈ S^3  (quaternion, normalized)
  velocity: [vx, vy, vz] ∈ R^3      (linear velocity)
  angular_vel: [ωx, ωy, ωz] ∈ R^3   (angular velocity)
  haptic_force: [fx, fy, fz] ∈ R^3  (feedback force)
  haptic_torque: [τx, τy, τz] ∈ R^3 (feedback torque)
}

Total dimension: 3 + 4 + 3 + 3 + 3 + 3 = 19D (or padded to 20D)

Haptics output layer:
  Force_feedback(Z) = K_p · (Z_target − Z_current) + K_d · (V_target − V_current)
    K_p ∈ R         proportional gain (stiffness)
    K_d ∈ R         derivative gain (damping)
    Range: [0, 1] (normalized to haptic device limits)

  Torque_feedback(Z) = K_rot · (R_target − R_current) + K_drot · (ω_target − ω_current)
    Similar structure for rotational feedback

Haptics device profiles:
  Profile            Max Force    Max Freq    Latency    Use Case
  ──────────────────────────────────────────────────────────────
  Basic (phone)      0.5 N        60 Hz       50 ms      mobile VR
  Standard (joy)     5.0 N        120 Hz      20 ms      console
  Advanced (glove)   10.0 N       240 Hz      5 ms       enterprise VR
```

### §A.9c Variable Wattage Tuning (Power Rail Telemetry)

```
Power telemetry integration (optional, local-only):

  b_t = actual_watts / tdp_ceiling  ∈ [0, 1]

Scaled parameters (LOCAL, not in H_t):
  λ_actual(b) = λ_base · (0.5 + 0.5·b)     [never fully disabled]
  α_actual(b) = α_base · b                 [scales to zero at low power]
  dt_thermal(b) = dt · (1.0 + thermal_headroom/100)  [adaptive step]

Constraints:
  - λ_actual ≥ 0.5·λ_base (never shut down dissipation)
  - α_actual ≥ 0 (backreaction may disable at ultra-low power)
  - Thermal headroom < 5°C triggers power throttle
  - Power budget < 20% triggers conservative mode

Power budget levels:
  b ≥ 0.8   → FULL (all features enabled)
  0.5 ≤ b < 0.8   → BALANCED (reduced DFE depth)
  b < 0.5    → CONSERVATIVE (scalar only, no VR/haptics)

Telemetry is INFORMATIONAL ONLY:
  - Does NOT affect H_t (hash uses λ_base, α_base)
  - Does NOT trigger consensus failure
  - Each peer applies own power scaling
  - Consensus still requires identical Z_base
```

### §A.9d Display Geometry (2D/3D Flat/Concave)

```
Optional display geometry variant (couples to VR renderer):

DisplayMode ∈ {
  FLAT_2D,          2D screen, no spatial curvature
  FLAT_3D,          flat monitor, 3D content (perspective)
  CONCAVE_2D,       curved screen, 2D mapped to dome
  CONCAVE_3D,       curved screen, 3D with distortion correction
  SPHERICAL_VR,     180°/360° VR headset
}

Geometry effect on Z_spatial (from VR state):
  pos_display = transform_geometry(pos_spatial, display_mode)

Examples:
  FLAT_2D:      pos_display = (x, y, 0)          [ignore z depth]
  FLAT_3D:      pos_display = perspective(x, y, z)  [compute z → xy]
  CONCAVE_3D:   pos_display = apply_barrel_correction(pos)
  SPHERICAL_VR: pos_display = equirectangular_map(pos)

Display curvature parameter (if concave):
  κ_display ∈ R  (curvature radius, negative = concave)

Haptic feedback adjustment:
  force_display = force_spatial · curve_jacobian(pos_spatial, κ_display)
  (curved displays redistribute forces to match perceived pressure)
```

### §A.10 Frame Rate Lock (Hard Fix Switch)

```
Frame rate is IMMUTABLE per session (protocol-frozen, non-negotiable).

Options:
  Frame_rate ∈ {60, 120, 240} Hz

Hard fix switch (SET AT INIT, CANNOT CHANGE MID-SESSION):

  struct SessionConfig {
    frame_rate_hz: u32,  // LOCKED at session start
    dt: f32,             // Computed as 1.0 / frame_rate_hz
    // Once session begins, changing frame_rate is FORBIDDEN
    // Violation → session error, must restart
  }

Effect on dt and integration:
  60 Hz:   dt = 1/60   ≈ 0.01667 s   (120 ms per frame, latency-tolerant)
  120 Hz:  dt = 1/120  ≈ 0.00833 s   (60 ms per frame, balanced)
  240 Hz:  dt = 1/240  ≈ 0.00417 s   (24 ms per frame, low-latency VR)

3D/VR integration per frame rate:
  60 Hz:   Haptics update every 1-2 frames (16-32 ms)
  120 Hz:  Haptics update every frame (8 ms)
  240 Hz:  Haptics update every frame (4 ms)

Dissipation scaling by frame rate:
  λ_fps = λ_base · (60 / frame_rate_hz)  [faster frames = less dissipation per step]
  α_fps = α_base · (frame_rate_hz / 60)  [faster frames = stronger backreaction]

Example:
  λ_base = 0.12, 60 Hz  → λ_60 = 0.12
  λ_base = 0.12, 120 Hz → λ_120 = 0.06
  λ_base = 0.12, 240 Hz → λ_240 = 0.03
  (faster frame rate = gentler per-step dissipation)
```

### §A.11 Frame Generation Parity (Anti-Ghosting & Determinism)

**Problem:** Synthetic frame interpolation/extrapolation can diverge between peers due to accumulated floating-point error or injection attacks. Frame generation parity provides cryptographically-binding anti-ghosting.

**FrameGenState (mutable per frame):**
```
z_prev[n]          frame N-1 (locked, immutable once committed)
z_curr[n]          frame N   (locked, immutable once committed)
z_synth[n]         synthetic frame (interpolated/extrapolated, N ∈ {0.5, 1.5})
ghost_err          ‖z_synth − z_actual_next‖ (prediction error metric)
frame_parity       XOR-based parity check (detects bit-flip corruption)
mode               generation mode (INTERPOLATE, EXTRAPOLATE, ADAPTIVE)
generation_tick    tick index when frame was synthesized (audit trail)
```

**Frame Generation Modes:**
```
INTERPOLATE:   z_synth = 0.5·z_prev + 0.5·z_curr
               (blend two real frames)

EXTRAPOLATE:   z_synth = 2·z_curr − z_prev
               (linear extrapolation forward)

ADAPTIVE:      choose mode based on motion magnitude δ = ‖z_curr − z_prev‖
               if δ < threshold_linear:
                 use EXTRAPOLATE (linear motion)
               else:
                 use INTERPOLATE (safer fallback)
```

**Parity Computation (Deterministic, Language-Agnostic):**
```
parity_computation(z_synth, ghost_err):
  parity_bits = 0
  for k in 0..n:
    parity_bits ⊕= bitcast_to_u32(z_synth[k])
  for error_bits in ghost_err (bitcast to u32):
    parity_bits ⊕= error_bits
  
  return parity_bits & 0xFF  (8-bit parity)

Verification:
  parity_expected = stored at frame generation
  parity_actual = recomputed from z_synth + ghost_err
  
  if parity_actual ≠ parity_expected:
    CORRUPTED: fall back to z_curr (real frame)
  else:
    VALID: use z_synth
```

**Frame Hash Binding (Immutable Protocol):**
```
H_frame = FNV1A_PARITY(
    z_synth 
    ⊕ mode 
    ⊕ frame_parity 
    ⊕ ghost_err 
    ⊕ generation_tick
    ⊕ frame_rate_hz  [from session]
)

Critical: All peers with identical inputs (z_prev, z_curr) MUST compute:
  - Identical z_synth (deterministic arithmetic)
  - Identical ghost_err (metric formula exact)
  - Identical frame_parity (bit-exact XOR)
  - Identical H_frame (hash agreement enables consensus)

Divergence → frame rollback to z_curr (real frame, safe baseline)
No retry: frame-gen failure is silent; real frame used without error.
```

**Anti-Ghosting Guarantee:**
```
If frame parity passes but z_synth is still corrupted:
  - Ghost error must exceed error bounds (§A.12b L3)
  - Spectral analysis must flag aliasing (§A.12b L5)
  - Temporal coherence must detect repeated mismatches (§A.12b L6)
  
Multi-layer forensic stack catches corruption that parity alone misses.
```

### §A.12 Suchness Identified (Extended Verification Stack)

**Definition:** Suchness = simultaneous satisfaction of tautology closure across 7 orthogonal verification layers. System is "self-consistent" only when all applicable layers pass; any failure triggers rollback to last valid checkpoint.

**Seven-Layer Verification Stack:**

```
L1 (BINDING):         H_t hash chain continuity (core tautology)
L2 (ORTHOGONALITY):   Z · S ≈ 0 (dual space separation)
L3 (GHOST CLOSURE):   G_t computed but never fed to Z evolution
L4 (FRAME PARITY):    Frame-gen parity matches (anti-ghosting)
L5 (QUATERNION):      Rotation norm ‖R‖ = 1.0 on S³ (VR geometry)
L6 (POWER SCALING):   λ_actual, α_actual consistent with telemetry
L7 (DISPLAY GEOMETRY): Display transforms yield deterministic output
```

**Aggregation Levels (Hierarchical Compliance):**
```
SUCHNESS TRIPLET (L1-L3):
  Binding ✓ + Orthogonality ✓ + Ghost closure ✓
  → Core tautology satisfied (baseline safe)
  → Sufficient for scalar 16D mode

SUCHNESS QUINTET (L1-L5):
  Triplet ✓ + Frame parity ✓ + Quaternion ✓
  → Core + VR geometric invariant (with 3D/haptics)
  → Sufficient for VR/frame-gen modes

SUCHNESS SEPTET (L1-L7):
  Quintet ✓ + Power scaling ✓ + Display geometry ✓
  → All invariants verified (paranoid mode)
  → Required for cross-datacenter forensic consensus
```

**Verification Algorithm (Per Tick):**
```
verify_suchness(config: SessionConfig, state: &DVSMState) → SuchnessLevel:

  # L1: BINDING CHECK (mandatory)
  hash_computed = FNV1A_PARITY(σ_t)
  hash_chain = hash_computed ⊕ hash_prev
  if hash_chain ≠ hash_expected:
    return FAILURE("Hash binding broken")
  
  # L2: ORTHOGONALITY CHECK (mandatory)
  dot_product = Σ_k Z_t[k] · S_t[k]
  if |dot_product| > ε_ortho (1e-10):
    return WARNING("Orthogonality degraded, basis reset recommended")
  
  # L3: GHOST CLOSURE CHECK (code audit, one-time)
  if code_audit_references_G_in_Z_step():
    return FAILURE("Ghost leaked into forward evolution")
  
  # L4: FRAME PARITY CHECK (if frame_gen enabled)
  if frame_gen_enabled:
    parity_actual = compute_frame_parity(z_synth, ghost_err)
    if parity_actual ≠ parity_stored:
      return FAILURE("Frame corruption detected")
  
  # L5: QUATERNION NORM CHECK (if vr_enabled)
  if vr_enabled:
    q_norm_sq = R[0]² + R[1]² + R[2]² + R[3]²
    if |q_norm_sq − 1.0| > ε_quat (1e-6):
      return FAILURE("Quaternion degenerate")
  
  # L6: POWER SCALING CHECK (paranoid mode only)
  if paranoid_mode_enabled:
    λ_recomputed = λ_base · (0.5 + 0.5·b_current)
    if |λ_recomputed − λ_applied| > ε_power (1e-8):
      return FAILURE("Power scaling diverged")
  
  # L7: DISPLAY GEOMETRY CHECK (paranoid mode only)
  if paranoid_mode_enabled && display_mode ≠ FLAT_2D:
    z_test = [0.5, 0.5, 0.5, ...]  // known vector
    z_transformed = apply_display_transform(z_test, display_mode)
    z_recovered = apply_inverse_display_transform(z_transformed, display_mode)
    if ‖z_recovered − z_test‖ > ε_display (1e-9):
      return FAILURE("Display transform non-deterministic")
  
  # Verdict
  if L1, L2, L3 pass:
    return TRIPLET_OK
  if L1-L5 pass:
    return QUINTET_OK
  if L1-L7 pass:
    return SEPTET_OK
```

**Rollback Semantics (On Failure):**
```
When any layer fails:
  1. Mark current tick as CORRUPTED
  2. Rewind to last GhostSnap checkpoint (immutable)
  3. Validate checkpoint passes same suchness_level
  4. Resume from checkpoint (deterministic state)
  5. Log corruption event (forensic audit trail)
  
If rollback fails (no valid checkpoint):
  → Session enters QUARANTINE mode
  → Requires manual intervention or forensic recovery
```

---

### §A.12b Frame Generation Forensic Stack (L1-L10)

**Problem:** Frame-gen parity detects bit-flip corruption but cannot detect systematic ghosting (synthetic frame diverging from physical reality due to model mismatch or nonlinear dynamics). Ten-layer forensic stack provides graduated confidence in frame validity.

**Forensic Layers (Orthogonal Checks):**

```
L1: INTERPOLATION DETERMINISM
    z_synth = 0.5·z_prev + 0.5·z_curr
    Verify arithmetic is byte-identical across peers
    
L2: PARITY VALIDATION (§A.11)
    parity_actual = XOR(z_synth ⊕ ghost_err)
    if parity_actual ≠ parity_expected:
      REJECT frame (bit corruption)

L3: ERROR BOUNDS CHECK
    ghost_err = ‖z_synth − z_actual_next‖  (L2 norm)
    Define threshold_L3 per deployment
    if ghost_err > threshold_L3:
      FLAG for forensic review (frame may lag reality)

L4: MOTION COHERENCE CHECK
    δ_prev = ‖z_curr − z_prev‖  (motion magnitude prev tick)
    δ_next = ‖z_actual_next − z_curr‖  (motion magnitude next tick)
    coherence = δ_next / (δ_prev + ε)
    
    if coherence > 2.0 or coherence < 0.5:
      FLAG for review (motion non-linear, extrapolation unreliable)

L5: SPECTRAL ANALYSIS
    Compute FFT of z_synth and z_actual_next
    peak_freq_synth = argmax_f |FFT(z_synth)[f]|
    peak_freq_actual = argmax_f |FFT(z_actual_next)[f]|
    
    if |peak_freq_synth − peak_freq_actual| > threshold_spectral:
      FLAG for review (aliasing detected, frame lags in frequency)

L6: TEMPORAL COHERENCE (Windowed Hash Drift)
    hash_synth = FNV1A(z_synth)
    hash_actual = FNV1A(z_actual_next)
    divergence_t = (hash_synth ≠ hash_actual) ? 1 : 0
    
    Accumulate divergence_t over window W (e.g., 100 frames)
    divergence_rate = Σ_t divergence_t / W
    
    if divergence_rate > threshold_temporal (e.g., 20%):
      DEGRADE forensic confidence (frame gen systematically off)

L7: ROLLBACK RECONSTRUCTION CHECK
    Can we recover z_actual from z_synth + ghost_err + delta?
    Attempt Taylor expansion backward:
      z_actual ≈ z_synth + (ghost_err / 2) · (δ_next / max(δ_prev, ε))
    
    If reconstruction error > threshold_L7:
      FLAG (frame gen assumption violated)

L8: ADVERSARIAL FRAME CHECK
    Could malicious peer craft z_synth to hide state mutation?
    
    Test: inject known delta to Z_t, predict z_synth, verify:
      If attacker can craft z_synth such that ghost_err is small
      but actual_z differs from predicted:
        VULNERABLE (frame gen can mask mutation)
    
    Mitigation: require frame parity + hash commitment (L2 + L10)

L9: MERKLE TREE HISTORY
    Build merkle(z_prev, z_curr, z_synth, ghost_err) for each frame
    merkle_root_t = SHA256(merkle_t−1 ‖ node_t)
    
    Peer agreement on merkle roots:
      if peer_root ≠ local_root:
        REJECT frame history (peer diverged)

L10: CRYPTOGRAPHIC COMMITMENT (SHA256 + Signature)
     H_commit = SHA256(z_synth ‖ ghost_err ‖ frame_parity ‖ generation_tick)
     sig = SIGN(H_commit, peer_key)  [immutable, forensic-grade]
     
     Verify on retrieval:
       if VERIFY(sig, H_commit) fails:
         REJECT (frame tampering detected post-hoc)
```

**Forensic Confidence Modes (Graduated Depth):**

```
Mode         Layers   Trust Level    Latency Cost    Use Case
─────────────────────────────────────────────────────────────
Green        L1-L2    50% (dev)      negligible      development
Standard     L1-L5    85% (local)    +0.5 ms         single-machine
Forensic     L1-L10   99% (xDC)      +5 ms           cross-datacenter

Selection:
  Green:    frame_gen enabled, no error checks
  Standard: frame_gen + error bounds, motion, spectral
  Forensic: full stack, merkle history, cryptographic proof
```

**Deployment Decision (Per Tick):**
```
if forensic_level = Green:
  use z_synth (parity check only)

else if forensic_level = Standard:
  if L1-L5 all pass:
    use z_synth
  else:
    fallback to z_curr (real frame, safe)

else if forensic_level = Forensic:
  if L1-L10 all pass AND merkle_root matches AND sig valid:
    use z_synth (high confidence)
  else:
    fallback to z_curr (conservative)
    log forensic failure (audit trail)
```

---

## §B ARCHITECTURAL DESIGN

### §B.1 Operator Pipeline (Strictly Sequential)

```
μ_t → [L_τ load] → [B_τ buffer] → [R_τ run+physics] → Z_t' 
      ↓
      [Memory EMA]
      ↓
      S_t' → [Basis adapt] → W_t' → [GhostSnap?] → [Suchness check] → OBS

At each stage, computed H_t checkpoint.
If any stage violates tautology → ROLLBACK to last valid checkpoint.
```

### §A.11b FPS Boost Mode (Porting-Friendly Acceleration)

```
FPS boost: optional lightweight mode for portable platforms.

Characteristics:
  - Dimension reduction: 20D VR → 16D scalar (spatial ops stripped)
  - Physics simplification: Lie bracket only, no Rose curve
  - Neural disabled: frozen MLP skipped
  - Backreaction reduced: α_boost = α_base · 0.25
  - No haptics feedback (output-only telemetry)
  - dt fixed: 1.0 / 240 (high frame rate, low per-frame work)

Boost factor:
  perf_multiplier = 2.0 to 4.0x (typical speedup vs standard)

Determinism maintained:
  - Same Z, S, W evolution as standard (just optimized path)
  - H_t includes boost_mode flag (different mode → different hash)
  - All tests pass (determinism, orthogonality, suchness)

Porting target: mobile (ARM), embedded (RISC-V), low-power FPGA
```

### §B.2 Runtime Modes (Consensus + Forensic Depth)

```
Mode         Consensus  DFE  Backreaction  GhostSnap  VR/Haptics  Frame Rates  Use
           Threshold                                               
──────────────────────────────────────────────────────────────────────────────────
Green        1          no   no            off        no          60 Hz        dev
Standard     2          yes  yes           1000       optional    60/120 Hz    local
Forensic     3          yes  yes           100        optional    60/120/240   cross-DC
Neural       2          yes  yes           500        yes         120/240 Hz   ML VR
VR           2          yes  yes           100        REQUIRED    120/240 Hz   haptics
```

Frame rate is LOCKED per session:
  - Set at initialization → cannot change mid-session
  - Violation → session reset required
  - Affects dt, λ_actual, α_actual, haptics update rate
```

### §B.3 Monorepo Structure

```
dvsm-core/
├── DVSM_SPEC.md                       (this file)
├── DVSM_IMPL.md                       (implementation + features)
├── Z2_EXTREME_ADDENDUM.md             (hardware variant: ROG Ally X 2025, MSI Claw A8)
├── Cargo.toml
│
├── rust/
│   ├── base/
│   │   ├── src/
│   │   │   ├── lib.rs                 (state, operators, backreaction)
│   │   │   │   ├── const MAX_CU: u32  (4 = Z1 Extreme, 16 = Z2 Extreme)
│   │   │   │   └── const MAX_WAVES    (derived from MAX_CU, platform-specific)
│   │   │   ├── fixed_point.rs         (Q31/Q16 codec)
│   │   │   ├── hash.rs                (FNV1A + parity)
│   │   │   └── cayley.rs              (projection + spyware detection)
│   │   └── Cargo.toml
│   │
│   ├── dfe/
│   │   ├── src/lib.rs                 (Lie-bracket + Rose curve)
│   │   └── Cargo.toml
│   │
│   ├── neural/
│   │   ├── src/lib.rs                 (frozen MLP for a, k params)
│   │   └── Cargo.toml
│   │
│   ├── platform/
│   │   ├── windows/
│   │   │   ├── gpu_occupancy.rs       (Z1 vs Z2 wave slot calculation)
│   │   │   └── profiler.rs            (RGP integration, FrameVarianceRing)
│   │   └── linux/
│   │       └── rocm_target.rs         (--offload-arch selection)
│   │
│   └── tests/
│       ├── determinism.rs             (Q31/Q16 equivalence)
│       ├── orthogonality.rs           (Z · S < ε)
│       ├── ghostsnap.rs               (bit-creep purging)
│       ├── suchness.rs                (tautology closure)
│       ├── cayley.rs                  (spyware rejection)
│       └── hardware_variant.rs        (Z1 vs Z2 occupancy model)
│
├── swift/
│   ├── DVSMCore.swift                 (Rust FFI or native)
│   └── Tests/
│
├── config/
│   └── profiles/
│       ├── z1_extreme.toml            (Phoenix, gfx1103, 4 CU)
│       └── z2_extreme.toml            (Strix Point, gfx1150, 16 CU)
│
└── spec/
    ├── Q31_codec.txt                  (fixed-point definition)
    ├── Rose_curves.txt                (harmonic analysis)
    └── Cayley_geometry.txt            (projection math)
```

**Hardware Variant Note:**
See §B.5 below and Z2_EXTREME_ADDENDUM.md for platform-specific configuration.

### §B.4 Determinism Contract

**Input:** identical μ sequence, same protocol_version
**Output:** identical Z, S, W, H_t across all implementations

Enforced via:
- FNV1A with parity (detects bit-flip attacks)
- Q31/Q16 quantization (no float rounding variance)
- GhostSnap checkpoints (resync on creep)
- Suchness verification (detect inconsistency)
- Cayley projection (reject spyware)

---

### §B.5 Hardware Variants (Platform-Specific Configuration)

**Scope:** Z1 Extreme (Phoenix, gfx1103) vs Z2 Extreme (Strix Point, gfx1150)

**Same Math, Different Hardware:**
- State tuple σ_t identical across platforms
- Operators (Lie bracket, backreaction, GhostSnap) identical
- H_t binding identical
- Suchness verification (L1-L7) identical
- Only difference: GPU compute unit count (4 vs 16 CUs)

**Platform-Specific Constants:**

```
Z1 Extreme (Phoenix, gfx1103):
  MAX_CU = 4
  MAX_WAVES = 4 × 2 × 16 = 128
  Wave occupancy: 1 DVSM wave / 128 slots = 0.78%
  TDP range: 15–35 W
  Compile flag: --offload-arch=gfx1103

Z2 Extreme (Strix Point, gfx1150):
  MAX_CU = 16
  MAX_WAVES = 16 × 2 × 16 = 512
  Wave occupancy: 1 DVSM wave / 512 slots = 0.19%
  TDP range: 17–35 W [minimum idle: 17W, maximum load: 35W]
  Compile flag: --offload-arch=gfx1150
```

**Performance Implications:**
- Z1: DVSM kernel consumes ~0.78% of GPU wave scheduling capacity
- Z2: DVSM kernel consumes ~0.19% of GPU wave scheduling capacity
- Z2 provides 4× more concurrent wave slots available for game renderer
- Z2 wall-time per tick: ~0.25–0.33× of Z1 (due to 4× more SIMDs, embarrassingly parallel kernel)

**Full Details:** See Z2_EXTREME_ADDENDUM.md
- Hardware delta table (GPU architecture, SIMD count, texture throughput)
- Code changes required (constant updates only)
- Kernel optimizations (scalar FPU, s_singleuse_vdst)
- Occupancy model revision
- Frame generation with AFMF2 coexistence
- Benchmark validation methodology

**For Deployment:**
1. Select appropriate profile: config/profiles/z1_extreme.toml or z2_extreme.toml
2. Verify hardware: ROG Ally X 2025 (Z2) vs 2024 (Z1), or equivalent (MSI Claw A8, etc.)
3. Compile with correct --offload-arch flag
4. Run verification harness (§D tests; all pass identically on both platforms)

---

## §C FEATURE MODULES

### §C.1 DFE (Lie-Bracket Spectral, Optional)

```
dZ_k/dt += Σⱼ κ_{kj}(Z_k S_j − Z_j S_k)

Three modes:
  RF_SPECTRAL       → raw Lie bracket
  FEATURE_SECURITY  → Π_W(z_enc) [projection only]
  KAPPA_KEYED       → z_enc * (1 + ‖κ‖)
```

### §C.2 Rose Curve Logic (Optional, Neural-Gated)

```
if neural_enabled:
  (a, k) = NeuralNet(Z, S, ‖Z‖²)  [frozen MLP weights]
  Rose_k += a · cos(k·θ) · Z_k / (‖Z_k‖ + ε)
else:
  Rose_k = 0
```

### §C.3 Ghost Rebirth (From EMA Memory)

```
if |Z_k| < collapse_threshold:
  Z_k := S_k · rebirth_scale
```

Requires backreaction to keep S_k non-zero.

### §C.4 Power-Rail Telemetry

```
b = actual_watts / tdp_ceiling

λ_actual = λ_base · (0.5 + 0.5·b)    [never fully disabled]
α_actual = α_base · b                [scales to zero at low power]

Note: base params frozen in protocol_version; telemetry is LOCAL ONLY.
Does NOT affect H_t.
```

### §C.5 Frame Generation + Anti-Ghost

```
FrameGenState:
  z_prev, z_curr, z_synth, ghost_err

Modes:
  Interpolate: z_synth = 0.5·z_prev + 0.5·z_curr
  Extrapolate: z_synth = 2·z_curr − z_prev

Anti-ghost check: ‖z_synth − z_actual‖ < threshold
```

---

## §D VERIFICATION HARNESS (IMMUTABLE)

**All derivatives must pass:**

```
CORE TESTS (all modes):
✓ test_q31_determinism         Same Z across languages
✓ test_q64_64_precision         Extended range accuracy
✓ test_q_adaptive_switching     Q31 → Q16 → Q64.64 as ‖Z‖ changes
✓ test_orthogonality            Z · S < ε at all ticks
✓ test_ghost_closure            G never fed to Z evolution
✓ test_ghostsnap_bitcreep       Purges accumulated error
✓ test_fnv1a_parity             Hash chain unbroken
✓ test_cayley_spyware           Rejects injected state
✓ test_suchness_identified      Tautology verified
✓ test_hash_determinism         H_t identical across peers
✓ test_backreaction_stability   ‖Z‖² → E_target
✓ test_rose_curve_harmonic      Neural output reproducible

FRAME RATE TESTS (60/120/240 Hz):
✓ test_frame_rate_immutable      dt locked at init
✓ test_frame_rate_scaling        λ_actual, α_actual scale correctly
✓ test_dt_determinism           dt identical across frame rate modes
✓ test_dt_hash_binding          frame_rate_hz in H_t prevents mid-session change

VR/HAPTICS TESTS (if vr_enabled):
✓ test_vr_spatial_state         Z 20D dimension maintained
✓ test_quaternion_normalization R_t stays on S³
✓ test_haptics_force_bounds     Force/torque within device limits
✓ test_haptics_latency          Feedback update ≤ frame period
✓ test_vr_orthogonality         Z_spatial · S_spatial < ε
✓ test_haptics_determinism      Identical inputs → identical force output
```

Failure → ROLLBACK to last suchness checkpoint.
Frame rate violation → SESSION RESET (non-recoverable).

---

## §E CONTROL PANEL (ON-SCREEN + BIOS)

### §E.1 On-Screen Control Panel

```
Real-time display (60 fps overlay):

┌─────────────────────────────────────────┐
│ DVSM Control Panel                      │
├─────────────────────────────────────────┤
│ Power:      35.0 W / 35.0 W (100%)      │
│ Temp:       65°C / 90°C (72%)           │
│ Frame Rate: 240 Hz [LOCKED]             │
│ Mode:       Standard                    │
│                                         │
│ Z norm:     0.987 (target: 1.000)       │
│ Z·S ortho:  4.2e-11 ✓                   │
│ Suchness:   ✓ ✓ ✓ (binding/ortho/ghost)│
│ Hash chain: 0x3f4a7c... (last 8)        │
│                                         │
│ VR Haptics: 5.2 N / 10.0 N max   (52%)  │
│ Ghost SNR:  18.3 dB                     │
│ Bit Creep:  0 checkpoints / 1000        │
│                                         │
│ [Boost Mode OFF] [Power Throttle OFF]   │
└─────────────────────────────────────────┘

Readable metrics:
  - Power budget (actual / tdp)
  - Thermal headroom
  - Frame rate + lock status
  - Z norm stability vs E_target
  - Orthogonality check (passes if < ε)
  - Suchness triplet (binding/orthogonality/ghost)
  - Hash chain integrity (last 64 bits)
  - Haptics force magnitude + device max
  - Ghost SNR (signal-to-noise)
  - GhostSnap checkpoint count
  - Boost mode toggle (if supported)
  - Power throttle active indicator
```

### §E.2 BIOS Configuration Panel

```
BIOS-level settings (persist across reboots):

┌─────────────────────────────────────────┐
│ DVSM Kernel Configuration               │
├─────────────────────────────────────────┤
│ Enable DVSM:           [ON]              │
│ Boot Mode:             [Standard]        │
│   Options: Green / Standard / Forensic  │
│                                         │
│ Power Limit:           [Dynamic]        │
│   Options: Fixed 15W / 25W / 35W        │
│           Dynamic (telemetry)           │
│                                         │
│ Frame Rate:            [240 Hz]         │
│   Options: 60 / 120 / 240 Hz            │
│                                         │
│ VR Support:            [Enabled]        │
│ Haptics:               [Enabled]        │
│ FPS Boost Mode:        [Disabled]       │
│                                         │
│ Display:               [Flat 3D]        │
│   Options: Flat 2D / Flat 3D           │
│           Concave 2D / Concave 3D      │
│           Spherical VR                 │
│                                         │
│ Fixed-Point Precision: [Q31]            │
│   Options: Q31 / Q16 / Q64.64          │
│           Adaptive (auto-switch)       │
│                                         │
│ Security Level:        [Standard]       │
│   Options: Standard / Paranoid         │
│           (paranoid = 2x GhostSnap)    │
│                                         │
│ Thermal Throttle:      [65°C]          │
│ Power Throttle:        [20% budget]    │
│                                         │
│ [Save & Reboot]  [Load Defaults]       │
└─────────────────────────────────────────┘

Persistent settings:
  - Boot mode (Green/Standard/Forensic)
  - Power limit (fixed or dynamic)
  - Frame rate lock
  - VR/haptics enabled
  - Display geometry
  - Q-mode default (can auto-switch runtime)
  - Security policy (standard vs paranoid)
  - Thermal/power thresholds
```

### §E.3 Telemetry Log (Scrollable)

```
Recent events (last 100):

Tick 9847 [10:23:45.231]  Power throttle ON (18% budget)
Tick 9834 [10:23:45.069]  Thermal headroom: 4.8°C (warning)
Tick 9821 [10:23:44.907]  GhostSnap checkpoint #47 (bitcreep: 2.1e-8)
Tick 9807 [10:23:44.745]  Frame rate immutable lock enforced
Tick 9794 [10:23:44.583]  Quaternion norm drift detected (rebalance)
Tick 9781 [10:23:44.421]  Suchness check: PASS (binding/ortho/ghost)
Tick 9768 [10:23:44.259]  Adaptive Q-switch: Q31 → Q16 (norm jumped)
Tick 9755 [10:23:44.097]  Haptic force clipped (desired 11.2N, max 10.0N)
...
```

---

## §F LANGUAGE & PORTABILITY

### §F.1 C Language Reference (For Porting)

```
Core kernel minimum viable implementation (C89 compatible):

File structure:
  dvsm_core.h      (types, prototypes, constants)
  dvsm_core.c      (operators, state evolution)
  dvsm_hash.c      (FNV1A + parity)
  dvsm_quaternion.c (rotation, normalization)
  dvsm_haptics.c   (force/torque)
  dvsm_display.c   (geometry transforms)
  dvsm_power.c     (telemetry, scaling)

Design principles:
  - No dynamic allocation (all stack/pre-allocated)
  - No floating-point when fixed-point suffices
  - SIMD-friendly loops (vectorizable over Z[])
  - Const-correctness (protocol-frozen in const tables)
  - Portable: IEEE 754 f32/f64, two's complement i32/i64

Target platforms: x86, ARM, RISC-V, MIPS, PowerPC
Minimum compiler: C89 (ANSI C) + IEEE 754 support
```

### §F.2 Security Hardening Checklist

```
✓ Bounds checks on all array accesses (16D, 20D, 256D κ)
✓ Overflow protection (Q-mode adaptive switching)
✓ Stack-only allocation (no malloc/free in hot path)
✓ Quaternion normalization every tick (prevent degenerate rotation)
✓ NaN/Inf guards on norm computation (clamp to [0, 10])
✓ Hash parity verification (detect bit-flip injection)
✓ Cayley projection (reject non-skew perturbations)
✓ GhostSnap checkpoints immutable (fork-join semantics)
✓ Replay hash chain (detect mutation post-hoc)
✓ Frame rate lock flag (prevent mid-session change)
✓ Protocol version tag in hash (prevent version downgrade)
✓ Suchness triplet verification (tautology closure proof)
✓ STATE BOUNDARY CLAMPING (§A.2b) — Immediately after Euler step
  • Hard clamp [-2.0, +2.0] (production; O(1))
  • OR soft clip 2·tanh(x/2) (paranoid; continuous)
  • Prevents NaN propagation → preserves H_t binding

Paranoid mode (optional, 2x cost):
✓ Recompute norm every 10 ticks (detect silent bit corruption)
✓ Hash entire Z, S, W each tick (catch any state mutation)
✓ Double-check Cayley skew-symmetry (two independent tests)
✓ GhostSnap every 100 ticks (vs default 1000)
✓ State saturation detection: count ticks where |Z_k| ≥ 1.8 (near boundary)
  → Log warning if saturation_count > 0.1% of ticks (anomaly indicator)
```

---

## References

- Protocol-frozen parameters: κ, λ, α, E_target, Q_mode, neural_enabled, protocol_version
- Mutable state: μ, Z, S, W
- Immutable contract: H_t binding, Z·S orthogonality, G ghost closure
- Tautology: Suchness (all three properties simultaneously)
