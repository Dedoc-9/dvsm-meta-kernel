# USER_SETTINGS_SPEC.md — Runtime Configuration & Kill-Switch UI

## Overview

**Objective:** Enable end-user configuration of DVSM runtime behavior without requiring application restart. All settings apply immediately via immutable state snapshot + reconfiguration handshake.

**Guarantee:** Changes to runtime settings do NOT require restart; supervisor validates and applies changes within next frame.

---

## §1 Settings Schema (JSON + C-Struct Dual)

### §1.1 JSON Configuration Template

File: `user_settings.json` (portable, editable by users)

```json
{
  "dvsm_config_version": "3.1",
  "runtime_settings": {
    "frame_rate_hz": 120,
    "sync_tier": 1,
    "paranoid_mode": false,
    "frame_generation_enabled": true,
    "vrs_enabled": true,
    "vrs_tile_size": 8,
    "spectral_harmonic_rose_enabled": false,
    "wattage_profile": "ALLY_X_Z2_BALANCED",
    "ghostsnap_max_checkpoints": 1000
  },
  "advanced_tuning": {
    "alpha_base": 0.05,
    "lambda_damping": 0.1,
    "ema_beta": 0.99,
    "q_mode": "Q31",
    "hard_clamp_enabled": true,
    "backreaction_pulse_scale": 4.0,
    "phase_lock_kappa": 0.25,
    "gudermannian_threshold": 5.0
  },
  "kill_switches": {
    "enable_phase_lock_pll": true,
    "enable_rose_curve": true,
    "enable_backreaction": true,
    "enable_ghostsnap": true,
    "enable_vr_quaternion_renorm": true,
    "enable_forensic_stack": false
  }
}
```

### §1.2 C-Struct Representation (Runtime State)

**Invariant:** C-struct is repr(C, align(64)) for memory safety and portability.

```c
#pragma pack(push, 1)

typedef struct {
    // === IMMUTABLE HEADER (64 bytes) ===
    uint32_t  config_version;              // 3.1 = 0x00030100
    uint32_t  protocol_version;            // H_t binding anchor
    uint64_t  timestamp_created_ns;        // Supervisor audit trail
    uint32_t  frame_rate_hz;               // Locked at creation
    uint32_t  __pad1;
    uint64_t  __pad2;
    
    // === RUNTIME TOGGLES (32 bytes) ===
    uint8_t   sync_tier;                   // 1=proportional, 2=Gudermannian, 3=Cayley
    uint8_t   paranoid_mode;               // Bool: tanh soft-clip
    uint8_t   frame_gen_enabled;           // Bool: interpolate/extrapolate
    uint8_t   vrs_enabled;                 // Bool: variable-rate shading
    uint8_t   spectral_harmonic_rose_enabled;         // Bool: spectral harmonic Rose curve forcing
    uint8_t   q_mode;                      // 1=Q31, 2=Q16, 3=Q64.64
    uint8_t   hard_clamp_enabled;          // Bool: [-2.0, 2.0] hard clamp
    uint8_t   __pad3;
    
    uint32_t  vrs_tile_size;               // 8 = 8×8 tiles
    uint32_t  ghostsnap_max_checkpoints;   // Max entries in ghost rebirth buffer
    
    // === KILL SWITCHES (16 bytes) ===
    uint8_t   kill_phase_lock;             // If 0, phase-lock PLL disabled
    uint8_t   kill_rose_curve;             // If 0, Rose term omitted
    uint8_t   kill_backreaction;           // If 0, backreaction pulse = 0
    uint8_t   kill_ghostsnap;              // If 0, ghost rebirth disabled
    uint8_t   kill_vr_quaternion_renorm;   // If 0, VR quaternion renorm skipped
    uint8_t   kill_forensic_stack;         // If 0, L4-L10 forensic checks disabled
    uint8_t   __pad4[10];
    
    // === COEFFICIENTS (64 bytes, Q31.32 encoded) ===
    int64_t   alpha_base_q31_32;           // Backreaction gain
    int64_t   lambda_damping_q31_32;       // Damping coefficient
    int64_t   ema_beta_q31_32;             // EMA memory weight
    int64_t   backreaction_pulse_scale_q31_32;  // Empirical scaling (default 4.0)
    int64_t   phase_lock_kappa_q31_32;     // Phase PLL coupling strength
    int64_t   gudermannian_threshold_q31_32;   // Singularity protection threshold
    int64_t   __pad5[2];
    
    // === WATTAGE PROFILE ENUM (8 bytes) ===
    uint32_t  wattage_profile_id;          // 1=ALLY_X_Z1_ECO, 2=ALLY_X_Z2_BALANCED, ...
    uint32_t  __reserved_future;
    
    // === SUPERVISOR STATE (32 bytes) ===
    uint32_t  apply_pending;               // Bool: pending settings reconfiguration
    uint32_t  apply_tick_deadline;         // Tick number by which to apply changes
    uint32_t  last_applied_tick;           // Last tick where new settings took effect
    uint32_t  validation_status;           // 0=OK, 1=PENDING, 2=FAILED
    uint64_t  __pad6[2];
    
    // === TOTAL SIZE: 256 bytes (4 cache lines) ===
} DVSMUserSettings;

#pragma pack(pop)

---

### §1.3 Multimodal Coupling Configuration (DVSM v3.2–v3.3)

**Purpose:** Session-immutable parameters for RF/ELF/BioScience 3D modality coupling. All parameters are set at initialization; changes require session restart to recalculate H_global.

```c
#pragma pack(push, 1)

typedef struct {
    // === MULTIMODAL INFLUENCE WEIGHTS (Q31.32, immutable per session) ===
    int64_t   rf_influence_q31_32;      // Radio frequency coupling strength [0.0, 1.0)
    int64_t   elf_influence_q31_32;     // ELF biological coupling strength [0.0, 1.0)
    int64_t   bio3d_influence_q31_32;   // BioScience 3D coupling strength [0.0, 1.0)
    
    // === COUPLING MODE ===
    uint8_t   coupling_mode;             // 0=off, 1=additive, 2=multiplicative
    uint8_t   _reserved[7];              // Padding to 64-byte alignment
    
    // === TOTAL SIZE: 32 bytes ===
} CouplingConfig;

#pragma pack(pop)

**JSON Representation (in user_settings.json):**

```json
{
  "coupling": {
    "rf_influence": 0.5,        // 0.0–1.0, strength of RF EM influence on backreaction
    "elf_influence": 0.75,      // 0.0–1.0, strength of ELF bio-sync influence
    "bio3d_influence": 0.0,     // 0.0–1.0, strength of BioScience 3D feedback
    "mode": 1                   // 0=off, 1=additive, 2=multiplicative
  }
}
```

**Conversion Rule (UI Slider to Q31.32):**

```rust
pub fn ui_slider_to_q31_32(slider_value: f32) -> i64 {
    // slider_value ∈ [0.0, 1.0] → Q31.32 ∈ [0, 2^31)
    assert!(slider_value >= 0.0 && slider_value <= 1.0);
    let scaled = (slider_value * (1i64 << 32) as f32) as i64;
    cmp::min(scaled, (1i64 << 31) - 1)
}
```

**Hash Binding (Session Initialization):**

```rust
pub fn hash_coupling_config(config: &CouplingConfig) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&config.rf_influence_q31_32.to_le_bytes());
    hasher.update(&config.elf_influence_q31_32.to_le_bytes());
    hasher.update(&config.bio3d_influence_q31_32.to_le_bytes());
    hasher.update(&[config.coupling_mode]);
    hasher.finalize().into()
}
```

**Immutability Constraint:**

```
Once H_global = HASH(H_core ⊕ H_aux ⊕ H_bio3d ⊕ hash_coupling_config(config) ⊕ version)
is computed at session initialization, config must remain unchanged.

If any coupling parameter is modified:
  → config hash changes
  → H_global is invalidated
  → All previously compressed frames become incompatible
  → REJECT configuration changes after initialization (or restart session)
```
```

---

## §2 Configuration Loading & Validation

### §2.1 Load from JSON (Runtime)

**Implementation Note (Phase I.0):** Use typed schema validation from `USER_SETTINGS_VALIDATION.rs` module. The schema-driven approach eliminates manual `.as_bool()` / `.as_f64()` parsing fragility and provides comprehensive error handling.

```rust
/// Load user_settings.json using typed schema validation
/// This delegates to UserSettingsSchema deserialization + validate_schema()
/// from USER_SETTINGS_VALIDATION.rs, then converts to C-struct
pub fn load_user_settings_json(path: &str) -> Result<DVSMUserSettings, String> {
    // Step 1: Use typed schema validation (eliminates serde_json::Value fragility)
    let schema = user_settings_validation::load_user_settings_json(path)
        .map_err(|e| format!("Schema validation failed: {}", e))?;
    
    // Step 2: Convert UserSettingsSchema → DVSMUserSettings C-struct
    let mut settings = DVSMUserSettings::default();
    
    // === IMMUTABLE HEADER ===
    settings.config_version = 0x00030100;
    settings.protocol_version = parse_version_string(&schema.dvsm_config_version)
        .map_err(|e| format!("Invalid version: {}", e))?;
    settings.timestamp_created_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Frame rate (LOCKED, immutable, already validated by schema)
    settings.frame_rate_hz = schema.runtime_settings.frame_rate_hz;
    
    // === RUNTIME TOGGLES ===
    settings.sync_tier = schema.runtime_settings.sync_tier;
    settings.paranoid_mode = schema.runtime_settings.paranoid_mode as u8;
    settings.frame_gen_enabled = schema.runtime_settings.frame_generation_enabled as u8;
    settings.vrs_enabled = schema.runtime_settings.vrs_enabled as u8;
    settings.spectral_harmonic_rose_enabled = schema.runtime_settings.spectral_harmonic_rose_enabled as u8;
    settings.vrs_tile_size = schema.runtime_settings.vrs_tile_size;
    settings.ghostsnap_max_checkpoints = schema.runtime_settings.ghostsnap_max_checkpoints;
    
    settings.q_mode = user_settings_validation::validate_q_mode(&schema.advanced_tuning.q_mode)
        .map_err(|e| format!("Q mode validation: {}", e))?;
    
    // === KILL SWITCHES ===
    settings.kill_phase_lock = schema.kill_switches.enable_phase_lock_pll as u8;
    settings.kill_rose_curve = schema.kill_switches.enable_rose_curve as u8;
    settings.kill_backreaction = schema.kill_switches.enable_backreaction as u8;
    settings.kill_ghostsnap = schema.kill_switches.enable_ghostsnap as u8;
    settings.kill_vr_quaternion_renorm = schema.kill_switches.enable_vr_quaternion_renorm as u8;
    settings.kill_forensic_stack = schema.kill_switches.enable_forensic_stack as u8;
    
    // === COEFFICIENTS (Q31.32, already validated by schema) ===
    settings.alpha_base_q31_32 = f32_to_q31_32(schema.advanced_tuning.alpha_base);
    settings.lambda_damping_q31_32 = f32_to_q31_32(schema.advanced_tuning.lambda_damping);
    settings.ema_beta_q31_32 = f32_to_q31_32(schema.advanced_tuning.ema_beta);
    settings.backreaction_pulse_scale_q31_32 = f32_to_q31_32(schema.advanced_tuning.backreaction_pulse_scale);
    settings.phase_lock_kappa_q31_32 = f32_to_q31_32(schema.advanced_tuning.phase_lock_kappa);
    settings.gudermannian_threshold_q31_32 = f32_to_q31_32(schema.advanced_tuning.gudermannian_threshold);
    
    // === VALIDATION ===
    settings.validation_status = 1;  // PENDING
    settings.apply_tick_deadline = u32::MAX;  // Will be set by supervisor
    
    Ok(settings)
}
```

**Key Changes:**
- Schema validation is delegated to `USER_SETTINGS_VALIDATION.rs` typed structs (not manual `.as_bool()` parsing)
- All numeric fields are already validated before conversion (range checks, finite checks, discrete value checks)
- RF/ELF address validation uses `parse_rf_elf_address()` from validation module (hex parsing, NULL check, range validation)
- Errors are actionable and specific (e.g., `"Invalid alpha_base=0.25. Must be in [0.01, 0.2]"` instead of generic parse error)

### §2.2 Validation Rules

**Rule 1:** Frame rate is immutable; cannot be changed after creation. Discrete values only: [30, 60, 120, 240] Hz.

**Rule 2:** Sync tier changes are allowed; Tier 1 (default) is always safe. Tier 2 and 3 require explicit opt-in via configuration.

**Rule 3:** All Q31.32 coefficients must satisfy range invariants (validated at schema deserialization):
  - `alpha_base`: [0.01, 0.2]
  - `lambda_damping`: [0.01, 0.5]
  - `ema_beta`: [0.8, 0.99]
  - All other coefficients: [-2.0, +2.0] encoded range

**Rule 4:** kill_switches override feature enablement; if kill_X = 0, feature X is disabled regardless of other flags.

**Rule 5:** All numeric field validation is performed during schema deserialization (USER_SETTINGS_VALIDATION.rs), NOT at C-struct level.

```rust
/// Validate C-struct representation post-conversion
/// (Most validation already completed during schema deserialization)
pub fn validate_user_settings(settings: &DVSMUserSettings) -> Result<(), String> {
    // Frame rate must be discrete valid value
    if ![30, 60, 120, 240].contains(&settings.frame_rate_hz) {
        return Err(format!("Invalid frame_rate_hz: {}. Must be 30|60|120|240", settings.frame_rate_hz));
    }
    
    // Sync tier must be 1-3
    if settings.sync_tier < 1 || settings.sync_tier > 3 {
        return Err(format!("Invalid sync_tier: {}. Must be 1|2|3", settings.sync_tier));
    }
    
    // Q mode must be valid
    if settings.q_mode < 1 || settings.q_mode > 3 {
        return Err(format!("Invalid q_mode: {}. Must be 1|2|3", settings.q_mode));
    }
    
    // VRS tile size range check (1-16)
    if settings.vrs_tile_size < 1 || settings.vrs_tile_size > 16 {
        return Err(format!("Invalid vrs_tile_size: {}. Must be in [1, 16]", settings.vrs_tile_size));
    }
    
    // Backreaction pulse scale range check (1.0-10.0 in float, [2^32, 10*2^32] in Q31.32)
    let pulse_min = f32_to_q31_32(1.0);
    let pulse_max = f32_to_q31_32(10.0);
    if settings.backreaction_pulse_scale_q31_32 < pulse_min || settings.backreaction_pulse_scale_q31_32 > pulse_max {
        return Err(format!("Invalid backreaction_pulse_scale: {}. Must be [1.0, 10.0]", 
            q31_32_to_f32(settings.backreaction_pulse_scale_q31_32)));
    }
    
    settings.validation_status = 0;  // OK
    Ok(())
}

// ============================================================================
// Rule 5: Multimodal Coupling Configuration Validation
// ============================================================================

pub fn validate_coupling_config(
    config: &CouplingConfig,
    protocol_version: u16,
) -> Result<(), String> {
    
    // Protocol version gates (v3.2 adds RF/ELF, v3.3 adds BioScience)
    if config.rf_influence_q31_32 > 0 || config.elf_influence_q31_32 > 0 {
        if protocol_version < 0x0302 {
            return Err("RF/ELF coupling requires DVSM v3.2+".to_string());
        }
    }
    
    if config.bio3d_influence_q31_32 > 0 {
        if protocol_version < 0x0303 {
            return Err("BioScience coupling requires DVSM v3.3+".to_string());
        }
    }
    
    // All influence values must be valid Q31.32 ∈ [0, 1)
    let valid_range_max = (1i64 << 31) - 1;  // 2^31 - 1
    
    if config.rf_influence_q31_32 < 0 || config.rf_influence_q31_32 > valid_range_max {
        return Err(format!("rf_influence out of range: {}", config.rf_influence_q31_32));
    }
    
    if config.elf_influence_q31_32 < 0 || config.elf_influence_q31_32 > valid_range_max {
        return Err(format!("elf_influence out of range: {}", config.elf_influence_q31_32));
    }
    
    if config.bio3d_influence_q31_32 < 0 || config.bio3d_influence_q31_32 > valid_range_max {
        return Err(format!("bio3d_influence out of range: {}", config.bio3d_influence_q31_32));
    }
    
    // Coupling mode must be valid (0=off, 1=additive, 2=multiplicative)
    if config.coupling_mode > 2 {
        return Err(format!("coupling_mode invalid: {}", config.coupling_mode));
    }
    
    Ok(())
}

// ============================================================================
// Rule 6: RF/ELF Buffer Address Validation
// ============================================================================

/// Validate RF/ELF ring buffer address using typed schema validation
/// Delegates to user_settings_validation::parse_rf_elf_address() which enforces:
///   - Hex format parsing (with or without "0x" prefix)
///   - NULL pointer check (addr != 0x0)
///   - Valid range check (> 0x100_000 for user-space addresses on 64-bit)
pub fn validate_rf_elf_buffer_address(addr_str: &str) -> Result<usize, String> {
    user_settings_validation::parse_rf_elf_address(addr_str)
        .map_err(|e| format!("RF/ELF address validation failed: {}", e))
}

// ============================================================================
// Rule 7: Hash Binding Verification (Session Immutability)
// ============================================================================

pub fn verify_coupling_hash_immutable(
    config: &CouplingConfig,
    expected_hash: &[u8; 32],
) -> Result<(), String> {
    let computed_hash = hash_coupling_config(config);
    
    if computed_hash != *expected_hash {
        return Err("Coupling config hash mismatch (config was modified)".to_string());
    }
    
    Ok(())
}
```

---

## §3 No-Restart Application (Supervisor Handshake)

### §3.1 Settings Change Protocol

**Supervisor tick-driven application:**
1. User modifies `user_settings.json` (or UI updates the struct)
2. Supervisor calls `apply_user_settings_async()` with new settings snapshot
3. Supervisor marks `apply_pending = 1` and `apply_tick_deadline = current_tick + 2` (apply within 2 frames)
4. At frame boundary, if `current_tick == apply_tick_deadline` and `apply_pending == 1`:
   - Store current Z state snapshot
   - Load new coefficient values into running state
   - Recompute `H_t` with new settings and locked protocol version
   - Set `apply_pending = 0`, `last_applied_tick = current_tick`

```rust
/// Apply new user settings within the next 2 frames (no restart required)
pub fn apply_user_settings_async(
    supervisor: &mut DVSMSupervisor,
    new_settings: DVSMUserSettings,
    current_tick: u32,
) -> Result<(), String> {
    // Validate before queuing
    validate_user_settings(&new_settings)?;
    
    // Store new settings in supervisor's pending buffer
    supervisor.pending_settings = new_settings;
    supervisor.settings.apply_pending = 1;
    supervisor.settings.apply_tick_deadline = current_tick.saturating_add(2);
    
    Ok(())
}

/// Called at frame boundary (inside dvsm_step_full or equivalent supervisor loop)
pub fn commit_pending_settings(supervisor: &mut DVSMSupervisor, current_tick: u32) {
    if supervisor.settings.apply_pending == 1 && current_tick == supervisor.settings.apply_tick_deadline {
        // Store snapshot of current Z state
        let z_snapshot = supervisor.state.z.clone();
        
        // Apply new coefficients
        supervisor.settings.alpha_base_q31_32 = supervisor.pending_settings.alpha_base_q31_32;
        supervisor.settings.lambda_damping_q31_32 = supervisor.pending_settings.lambda_damping_q31_32;
        supervisor.settings.ema_beta_q31_32 = supervisor.pending_settings.ema_beta_q31_32;
        supervisor.settings.kill_phase_lock = supervisor.pending_settings.kill_phase_lock;
        supervisor.settings.kill_rose_curve = supervisor.pending_settings.kill_rose_curve;
        supervisor.settings.kill_backreaction = supervisor.pending_settings.kill_backreaction;
        // ... (all other runtime-changeable fields)
        
        // Recompute hash with new protocol binding
        let new_hash = hash_state_with_nominal_dt(
            &supervisor.state,
            &supervisor.wattage_profile,
            supervisor.settings.frame_rate_hz as f32,
        );
        
        // Update telemetry
        supervisor.settings.last_applied_tick = current_tick;
        supervisor.settings.apply_pending = 0;
        supervisor.settings.validation_status = 0;  // OK
        
        eprintln!("[DVSM Supervisor] User settings applied at tick {}. New H_t = {:x}", current_tick, new_hash);
    }
}
```

---

## §4 On-Screen Control Panel UI Schema

### §4.1 In-Game Overlay (Allegro / ImGui)

```cpp
// Pseudo-code for on-screen UI
class DVSMControlPanel {
    bool visible;
    DVSMUserSettings display_settings;
    
    void render() {
        if (!visible) return;
        
        ImGui::SetNextWindowPos(ImVec2(50, 50), ImGuiCond_FirstUseEver);
        ImGui::SetNextWindowSize(ImVec2(400, 500), ImGuiCond_FirstUseEver);
        
        if (ImGui::Begin("DVSM Control Panel", &visible)) {
            ImGui::Text("Frame Rate: %d Hz (LOCKED)", display_settings.frame_rate_hz);
            
            ImGui::Separator();
            ImGui::Text("Runtime Settings");
            
            ImGui::RadioButton("Sync Tier 1 (Proportional)", (int*)&display_settings.sync_tier, 1);
            ImGui::RadioButton("Sync Tier 2 (Gudermannian)", (int*)&display_settings.sync_tier, 2);
            ImGui::RadioButton("Sync Tier 3 (Cayley)", (int*)&display_settings.sync_tier, 3);
            
            ImGui::Checkbox("Paranoid Mode (Soft-Clip)", (bool*)&display_settings.paranoid_mode);
            ImGui::Checkbox("Frame Generation Enabled", (bool*)&display_settings.frame_gen_enabled);
            ImGui::Checkbox("VRS Enabled", (bool*)&display_settings.vrs_enabled);
            ImGui::Checkbox("Spectral Harmonic Rose Enabled", (bool*)&display_settings.spectral_harmonic_rose_enabled);
            
            ImGui::Separator();
            ImGui::Text("Kill Switches");
            
            ImGui::Checkbox("Enable Phase-Lock PLL", (bool*)&display_settings.kill_phase_lock);
            ImGui::Checkbox("Enable Rose Curve", (bool*)&display_settings.kill_rose_curve);
            ImGui::Checkbox("Enable Backreaction", (bool*)&display_settings.kill_backreaction);
            ImGui::Checkbox("Enable GhostSnap", (bool*)&display_settings.kill_ghostsnap);
            ImGui::Checkbox("Enable VR Quaternion Renorm", (bool*)&display_settings.kill_vr_quaternion_renorm);
            ImGui::Checkbox("Enable Forensic Stack", (bool*)&display_settings.kill_forensic_stack);
            
            ImGui::Separator();
            ImGui::Text("Advanced Tuning (Q31.32)");
            
            float alpha = q31_32_to_f32(display_settings.alpha_base_q31_32);
            if (ImGui::SliderFloat("Alpha (Backreaction Gain)", &alpha, 0.01f, 0.2f)) {
                display_settings.alpha_base_q31_32 = f32_to_q31_32(alpha);
            }
            
            float lambda = q31_32_to_f32(display_settings.lambda_damping_q31_32);
            if (ImGui::SliderFloat("Lambda (Damping)", &lambda, 0.01f, 0.3f)) {
                display_settings.lambda_damping_q31_32 = f32_to_q31_32(lambda);
            }
            
            ImGui::Separator();
            
            if (ImGui::Button("APPLY SETTINGS (No Restart)", ImVec2(-1, 0))) {
                apply_user_settings_async(&supervisor, display_settings, current_tick);
            }
            
            ImGui::Text("Last Applied: Tick %d", display_settings.last_applied_tick);
            ImGui::End();
        }
    }
};
```

### §4.2 BIOS-Level Configuration (EFI Firmware)

**Path:** UEFI Setup → Advanced → DVSM Settings

- Frame Rate: [60 Hz / 120 Hz / 240 Hz] (immutable after POST)
- Sync Tier: [1 / 2 / 3]
- Paranoid Mode: [Yes / No]
- Q Mode: [Q31 / Q16 / Q64.64]
- Kill Switches: [Enable Phase-Lock PLL / Enable Backreaction / ...]

**Persistence:** Settings stored in NVRAM, loaded at boot before DVSM kernel initialization.

---

## §5 File I/O Integration

### §5.1 Config File Paths

- **On-Screen UI:** Supervisor periodically saves UI state to `~/.dvsm/user_settings.json`
- **BIOS/Firmware:** Embedded in UEFI module; serialized to binary format for NVRAM efficiency
- **Fallback:** If `user_settings.json` is missing or corrupted, load hardcoded defaults (Tier 1, paranoid_mode=false, all kill switches enabled)

### §5.2 Persistence Layer

```rust
/// Serialize DVSMUserSettings C-struct back to JSON for persistence
/// Round-trips through UserSettingsSchema for consistent schema representation
pub fn save_settings_to_json(settings: &DVSMUserSettings, path: &str) -> Result<(), String> {
    let json = serde_json::json!({
        "dvsm_config_version": format_version(settings.protocol_version),
        "runtime_settings": {
            "frame_rate_hz": settings.frame_rate_hz,
            "sync_tier": settings.sync_tier as i32,
            "paranoid_mode": settings.paranoid_mode != 0,
            "frame_generation_enabled": settings.frame_gen_enabled != 0,
            "vrs_enabled": settings.vrs_enabled != 0,
            "spectral_harmonic_rose_enabled": settings.spectral_harmonic_rose_enabled != 0,
            "vrs_tile_size": settings.vrs_tile_size,
            "wattage_profile": format_wattage_profile(settings.wattage_profile_id),
            "ghostsnap_max_checkpoints": settings.ghostsnap_max_checkpoints,
        },
        "advanced_tuning": {
            "alpha_base": q31_32_to_f32(settings.alpha_base_q31_32),
            "lambda_damping": q31_32_to_f32(settings.lambda_damping_q31_32),
            "ema_beta": q31_32_to_f32(settings.ema_beta_q31_32),
            "q_mode": format_q_mode(settings.q_mode),
            "hard_clamp_enabled": settings.hard_clamp_enabled != 0,
            "backreaction_pulse_scale": q31_32_to_f32(settings.backreaction_pulse_scale_q31_32),
            "phase_lock_kappa": q31_32_to_f32(settings.phase_lock_kappa_q31_32),
            "gudermannian_threshold": q31_32_to_f32(settings.gudermannian_threshold_q31_32),
        },
        "kill_switches": {
            "enable_phase_lock_pll": settings.kill_phase_lock != 0,
            "enable_rose_curve": settings.kill_rose_curve != 0,
            "enable_backreaction": settings.kill_backreaction != 0,
            "enable_ghostsnap": settings.kill_ghostsnap != 0,
            "enable_vr_quaternion_renorm": settings.kill_vr_quaternion_renorm != 0,
            "enable_forensic_stack": settings.kill_forensic_stack != 0,
        }
    });
    
    std::fs::write(path, serde_json::to_string_pretty(&json)?)
        .map_err(|e| format!("Failed to save settings: {}", e))
}
```

**Compression & RF/ELF Configuration (Phase 2 Integration):**

The `CompressionConfig` and `RfElfConfig` structures from USER_SETTINGS_VALIDATION.rs are not yet integrated into DVSMUserSettings. Integration scheduled for Phase 2:
- `kill_compression` flag: supervisor toggle for async compression enqueue
- `enable_rf_elf_coupling` + `rf_elf_buffer_address`: ring buffer state machine initialization
- `rf_elf_stale_threshold_ms`: staleness detection for buffer eviction

These additions will be merged into §1.2 (C-Struct) and §2.1 (Load) during Phase 2 implementation of compression and RF/ELF ring buffer support.

---

## §6 Summary

- **No-Restart Guarantee:** Settings applied within 2 frames via supervisor handshake
- **Kill-Switch Control:** Binary toggles disable features without code changes
- **Dual Interface:** On-screen UI (ImGui) + BIOS/firmware integration
- **Determinism:** All coefficients in Q31.32; changes preserve integer-only arithmetic
- **Portability:** JSON schema cross-platform; C-struct repr(C, align(64)) byte-identical
- **Phase I.0 Validation (Session 7):** Typed schema validation from USER_SETTINGS_VALIDATION.rs replaces manual serde_json::Value parsing
  - Frame rate: discrete [30, 60, 120, 240] Hz validation
  - Q mode: validated enum ["Q31", "Q16", "Q64.64"]
  - Sync tier: range [1, 3]
  - Coefficients: range validation (α ∈ [0.01, 0.2], λ ∈ [0.01, 0.5], β ∈ [0.8, 0.99])
  - RF/ELF address: hex parsing, NULL check, > 1MB range validation
  - All finite-number checks for floating-point coefficients
- **Phase 2 Pending:** Compression config (CompressionConfig) and RF/ELF ring buffer integration
- **Architecture:** Schema validation module (USER_SETTINGS_VALIDATION.rs) → C-struct conversion → supervisor application handshake
