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
    "neural_rose_enabled": false,
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
    uint8_t   neural_rose_enabled;         // Bool: neural network Rose curve
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
```

---

## §2 Configuration Loading & Validation

### §2.1 Load from JSON (Runtime)

```rust
/// Load user_settings.json and convert to C-struct
pub fn load_user_settings_json(path: &str) -> Result<DVSMUserSettings, String> {
    let json_str = std::fs::read_to_string(path)
        .map_err(|e| format!("JSON load failed: {}", e))?;
    
    let config: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("JSON parse failed: {}", e))?;
    
    let mut settings = DVSMUserSettings::default();
    
    // === IMMUTABLE HEADER ===
    settings.config_version = 0x00030100;
    settings.protocol_version = config["dvsm_config_version"]
        .as_str()
        .ok_or("Missing dvsm_config_version")?
        .parse()
        .map_err(|_| "Invalid version")?;
    settings.timestamp_created_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Frame rate (LOCKED, immutable)
    settings.frame_rate_hz = config["runtime_settings"]["frame_rate_hz"]
        .as_u64()
        .ok_or("Missing frame_rate_hz")? as u32;
    
    if settings.frame_rate_hz < 30 || settings.frame_rate_hz > 240 {
        return Err(format!("Frame rate {} out of range [30, 240]", settings.frame_rate_hz));
    }
    
    // === RUNTIME TOGGLES ===
    settings.sync_tier = config["runtime_settings"]["sync_tier"]
        .as_u64()
        .unwrap_or(1) as u8;
    settings.paranoid_mode = config["runtime_settings"]["paranoid_mode"]
        .as_bool()
        .unwrap_or(false) as u8;
    settings.frame_gen_enabled = config["runtime_settings"]["frame_generation_enabled"]
        .as_bool()
        .unwrap_or(true) as u8;
    settings.vrs_enabled = config["runtime_settings"]["vrs_enabled"]
        .as_bool()
        .unwrap_or(true) as u8;
    settings.neural_rose_enabled = config["runtime_settings"]["neural_rose_enabled"]
        .as_bool()
        .unwrap_or(false) as u8;
    
    let q_str = config["advanced_tuning"]["q_mode"]
        .as_str()
        .unwrap_or("Q31");
    settings.q_mode = match q_str {
        "Q16" => 2,
        "Q64.64" => 3,
        _ => 1,  // Default Q31
    };
    
    // === KILL SWITCHES ===
    settings.kill_phase_lock = config["kill_switches"]["enable_phase_lock_pll"]
        .as_bool()
        .unwrap_or(true) as u8;
    settings.kill_rose_curve = config["kill_switches"]["enable_rose_curve"]
        .as_bool()
        .unwrap_or(true) as u8;
    settings.kill_backreaction = config["kill_switches"]["enable_backreaction"]
        .as_bool()
        .unwrap_or(true) as u8;
    settings.kill_ghostsnap = config["kill_switches"]["enable_ghostsnap"]
        .as_bool()
        .unwrap_or(true) as u8;
    settings.kill_vr_quaternion_renorm = config["kill_switches"]["enable_vr_quaternion_renorm"]
        .as_bool()
        .unwrap_or(true) as u8;
    settings.kill_forensic_stack = config["kill_switches"]["enable_forensic_stack"]
        .as_bool()
        .unwrap_or(false) as u8;
    
    // === COEFFICIENTS (Q31.32) ===
    settings.alpha_base_q31_32 = f32_to_q31_32(
        config["advanced_tuning"]["alpha_base"]
            .as_f64()
            .unwrap_or(0.05) as f32
    );
    settings.lambda_damping_q31_32 = f32_to_q31_32(
        config["advanced_tuning"]["lambda_damping"]
            .as_f64()
            .unwrap_or(0.1) as f32
    );
    settings.ema_beta_q31_32 = f32_to_q31_32(
        config["advanced_tuning"]["ema_beta"]
            .as_f64()
            .unwrap_or(0.99) as f32
    );
    settings.backreaction_pulse_scale_q31_32 = f32_to_q31_32(
        config["advanced_tuning"]["backreaction_pulse_scale"]
            .as_f64()
            .unwrap_or(4.0) as f32
    );
    
    // === VALIDATION ===
    settings.validation_status = 1;  // PENDING
    settings.apply_tick_deadline = u32::MAX;  // Will be set by supervisor
    
    Ok(settings)
}
```

### §2.2 Validation Rules

**Rule 1:** Frame rate is immutable; cannot be changed after creation.

**Rule 2:** Sync tier changes are allowed; Tier 1 (default) is always safe. Tier 2 requires explicit opt-in via kill_switch.

**Rule 3:** All Q31.32 coefficients must be in [-2.0, +2.0] encoded range.

**Rule 4:** kill_switches override feature enablement; if kill_X = 0, feature X is disabled regardless of other flags.

```rust
pub fn validate_user_settings(settings: &DVSMUserSettings) -> Result<(), String> {
    // Frame rate must be in valid range
    if settings.frame_rate_hz < 30 || settings.frame_rate_hz > 240 {
        return Err(format!("Invalid frame_rate_hz: {}", settings.frame_rate_hz));
    }
    
    // Sync tier must be 1-3
    if settings.sync_tier < 1 || settings.sync_tier > 3 {
        return Err(format!("Invalid sync_tier: {}", settings.sync_tier));
    }
    
    // Q mode must be valid
    if settings.q_mode < 1 || settings.q_mode > 3 {
        return Err(format!("Invalid q_mode: {}", settings.q_mode));
    }
    
    // Validate Q31.32 coefficients are in representable range
    let clamp_max = f32_to_q31_32(2.0);
    let clamp_min = f32_to_q31_32(-2.0);
    
    for coeff in &[
        settings.alpha_base_q31_32,
        settings.lambda_damping_q31_32,
        settings.ema_beta_q31_32,
        settings.backreaction_pulse_scale_q31_32,
    ] {
        if *coeff < clamp_min || *coeff > clamp_max {
            return Err(format!("Coefficient out of range: {}", coeff));
        }
    }
    
    settings.validation_status = 0;  // OK
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
            ImGui::Checkbox("Neural Rose Enabled", (bool*)&display_settings.neural_rose_enabled);
            
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
pub fn save_settings_to_json(settings: &DVSMUserSettings, path: &str) -> Result<(), String> {
    let json = serde_json::json!({
        "dvsm_config_version": "3.1",
        "runtime_settings": {
            "frame_rate_hz": settings.frame_rate_hz,
            "sync_tier": settings.sync_tier as i32,
            "paranoid_mode": settings.paranoid_mode != 0,
            "frame_generation_enabled": settings.frame_gen_enabled != 0,
            "vrs_enabled": settings.vrs_enabled != 0,
            "neural_rose_enabled": settings.neural_rose_enabled != 0,
            "wattage_profile": format_wattage_profile(settings.wattage_profile_id),
        },
        "advanced_tuning": {
            "alpha_base": q31_32_to_f32(settings.alpha_base_q31_32),
            "lambda_damping": q31_32_to_f32(settings.lambda_damping_q31_32),
            "ema_beta": q31_32_to_f32(settings.ema_beta_q31_32),
            "q_mode": format_q_mode(settings.q_mode),
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

---

## §6 Summary

- **No-Restart Guarantee:** Settings applied within 2 frames via supervisor handshake
- **Kill-Switch Control:** Binary toggles disable features without code changes
- **Dual Interface:** On-screen UI (ImGui) + BIOS/firmware integration
- **Determinism:** All coefficients in Q31.32; changes preserve integer-only arithmetic
- **Portability:** JSON schema cross-platform; C-struct repr(C, align(64)) byte-identical
