/// USER_SETTINGS_VALIDATION.rs - Robust JSON Schema Validation (Phase I.0)
///
/// Corrected implementation with comprehensive error handling and schema validation.
/// Replaces manual serde_json::Value parsing with structured schema checking.

use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

// ============================================================================
// SCHEMA DEFINITIONS (Type-Safe, Self-Documenting)
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserSettingsSchema {
    pub dvsm_config_version: String,
    pub runtime_settings: RuntimeSettings,
    pub advanced_tuning: AdvancedTuning,
    pub kill_switches: KillSwitches,
    #[serde(default)]
    pub compression: CompressionConfig,
    #[serde(default)]
    pub rf_elf: RfElfConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeSettings {
    pub frame_rate_hz: u32,
    pub sync_tier: u8,
    pub paranoid_mode: bool,
    pub frame_generation_enabled: bool,
    pub vrs_enabled: bool,
    pub vrs_tile_size: u32,
    pub spectral_harmonic_rose_enabled: bool,
    pub wattage_profile: String,
    pub ghostsnap_max_checkpoints: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AdvancedTuning {
    pub alpha_base: f32,
    pub lambda_damping: f32,
    pub ema_beta: f32,
    pub q_mode: String,
    pub hard_clamp_enabled: bool,
    pub backreaction_pulse_scale: f32,
    pub phase_lock_kappa: f32,
    pub gudermannian_threshold: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KillSwitches {
    pub enable_phase_lock_pll: bool,
    pub enable_rose_curve: bool,
    pub enable_backreaction: bool,
    pub enable_ghostsnap: bool,
    pub enable_vr_quaternion_renorm: bool,
    pub enable_forensic_stack: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CompressionConfig {
    #[serde(default = "default_compression_enabled")]
    pub enable_compression: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RfElfConfig {
    #[serde(default)]
    pub enable_rf_elf_coupling: bool,
    #[serde(default)]
    pub rf_elf_buffer_address: Option<String>,
    #[serde(default = "default_stale_threshold")]
    pub rf_elf_stale_threshold_ms: f32,
}

fn default_compression_enabled() -> bool { true }
fn default_stale_threshold() -> f32 { 50.0 }

// ============================================================================
// VALIDATION ERRORS (Explicit, Actionable)
// ============================================================================

#[derive(Debug, Clone)]
pub enum SettingsError {
    JsonParseFailed(String),
    SchemaMismatch(String),
    InvalidFrameRate(u32),
    InvalidQMode(String),
    InvalidSyncTier(u8),
    InvalidAlpha(f32),
    InvalidLambda(f32),
    InvalidEmaBeta(f32),
    RfElfEnabledButNoAddress,
    RfElfAddressParseError(String),
    RfElfAddressIsNull,
    RfElfAddressOutOfRange(usize),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JsonParseFailed(e) => write!(f, "JSON parse failed: {}", e),
            Self::SchemaMismatch(e) => write!(f, "Schema mismatch: {}", e),
            Self::InvalidFrameRate(fr) => write!(f, "Invalid frame_rate_hz={}. Must be in [30, 240]", fr),
            Self::InvalidQMode(qm) => write!(f, "Invalid q_mode='{}'. Must be 'Q31', 'Q16', or 'Q64.64'", qm),
            Self::InvalidSyncTier(st) => write!(f, "Invalid sync_tier={}. Must be 1, 2, or 3", st),
            Self::InvalidAlpha(a) => write!(f, "Invalid alpha_base={}. Must be in [0.01, 0.2]", a),
            Self::InvalidLambda(l) => write!(f, "Invalid lambda_damping={}. Must be in [0.01, 0.5]", l),
            Self::InvalidEmaBeta(b) => write!(f, "Invalid ema_beta={}. Must be in [0.8, 0.99]", b),
            Self::RfElfEnabledButNoAddress => write!(f, "RF/ELF coupling enabled but no buffer address provided"),
            Self::RfElfAddressParseError(e) => write!(f, "RF/ELF buffer address parse error: {}", e),
            Self::RfElfAddressIsNull => write!(f, "RF/ELF buffer address is 0x0 (NULL pointer)"),
            Self::RfElfAddressOutOfRange(addr) => write!(f, "RF/ELF buffer address 0x{:x} out of valid range", addr),
        }
    }
}

// ============================================================================
// VALIDATION FUNCTIONS (Phase I.0 Schema Validation)
// ============================================================================

pub fn validate_frame_rate(fr: u32) -> Result<(), SettingsError> {
    if fr < 30 || fr > 240 {
        return Err(SettingsError::InvalidFrameRate(fr));
    }
    // Only allow discrete values
    if ![30, 60, 120, 240].contains(&fr) {
        return Err(SettingsError::InvalidFrameRate(fr));
    }
    Ok(())
}

pub fn validate_q_mode(qm: &str) -> Result<u8, SettingsError> {
    match qm {
        "Q31" => Ok(1),
        "Q16" => Ok(2),
        "Q64.64" => Ok(3),
        _ => Err(SettingsError::InvalidQMode(qm.to_string())),
    }
}

pub fn validate_sync_tier(st: u8) -> Result<(), SettingsError> {
    if st < 1 || st > 3 {
        return Err(SettingsError::InvalidSyncTier(st));
    }
    Ok(())
}

pub fn validate_alpha(a: f32) -> Result<(), SettingsError> {
    if a < 0.01 || a > 0.2 {
        return Err(SettingsError::InvalidAlpha(a));
    }
    // Check for NaN/Inf
    if !a.is_finite() {
        return Err(SettingsError::InvalidAlpha(a));
    }
    Ok(())
}

pub fn validate_lambda(l: f32) -> Result<(), SettingsError> {
    if l < 0.01 || l > 0.5 {
        return Err(SettingsError::InvalidLambda(l));
    }
    if !l.is_finite() {
        return Err(SettingsError::InvalidLambda(l));
    }
    Ok(())
}

pub fn validate_ema_beta(b: f32) -> Result<(), SettingsError> {
    if b < 0.8 || b > 0.99 {
        return Err(SettingsError::InvalidEmaBeta(b));
    }
    if !b.is_finite() {
        return Err(SettingsError::InvalidEmaBeta(b));
    }
    Ok(())
}

pub fn parse_rf_elf_address(addr_str: &str) -> Result<usize, SettingsError> {
    // Strip "0x" prefix if present
    let hex_str = addr_str
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");

    // Parse hex string
    let addr = usize::from_str_radix(hex_str, 16)
        .map_err(|e| SettingsError::RfElfAddressParseError(e.to_string()))?;

    // Validate: not NULL
    if addr == 0 {
        return Err(SettingsError::RfElfAddressIsNull);
    }

    // Validate: reasonable range (avoid obviously invalid addresses)
    // For 64-bit systems, allow addresses > 1MB (reasonable user space)
    if addr < 0x100_000 {
        return Err(SettingsError::RfElfAddressOutOfRange(addr));
    }

    Ok(addr)
}

// ============================================================================
// LOAD & VALIDATE (Type-Safe, Comprehensive)
// ============================================================================

pub fn load_user_settings_json(path: &str) -> Result<UserSettingsSchema, SettingsError> {
    // Step 1: Read file
    let json_str = std::fs::read_to_string(path)
        .map_err(|e| SettingsError::JsonParseFailed(format!("File I/O: {}", e)))?;

    // Step 2: Parse JSON into typed schema
    let config: UserSettingsSchema = serde_json::from_str(&json_str)
        .map_err(|e| SettingsError::JsonParseFailed(e.to_string()))?;

    // Step 3: Validate schema constraints
    validate_schema(&config)?;

    Ok(config)
}

pub fn validate_schema(config: &UserSettingsSchema) -> Result<(), SettingsError> {
    // Validate version
    let _version = validate_version(&config.dvsm_config_version)?;

    // Validate runtime settings
    validate_frame_rate(config.runtime_settings.frame_rate_hz)?;
    validate_sync_tier(config.runtime_settings.sync_tier)?;
    if config.runtime_settings.vrs_tile_size < 1 || config.runtime_settings.vrs_tile_size > 16 {
        return Err(SettingsError::SchemaMismatch(
            format!("vrs_tile_size {} out of range [1, 16]", config.runtime_settings.vrs_tile_size)
        ));
    }

    // Validate advanced tuning
    validate_alpha(config.advanced_tuning.alpha_base)?;
    validate_lambda(config.advanced_tuning.lambda_damping)?;
    validate_ema_beta(config.advanced_tuning.ema_beta)?;
    validate_q_mode(&config.advanced_tuning.q_mode)?;

    if config.advanced_tuning.backreaction_pulse_scale < 1.0 ||
       config.advanced_tuning.backreaction_pulse_scale > 10.0 {
        return Err(SettingsError::SchemaMismatch(
            format!("backreaction_pulse_scale {} out of range [1.0, 10.0]",
                    config.advanced_tuning.backreaction_pulse_scale)
        ));
    }

    // Validate RF/ELF config
    if config.rf_elf.enable_rf_elf_coupling {
        if let Some(addr_str) = &config.rf_elf.rf_elf_buffer_address {
            let _ = parse_rf_elf_address(addr_str)?;  // Validates format and range
        } else {
            return Err(SettingsError::RfElfEnabledButNoAddress);
        }
    }

    // Validate stale threshold
    if config.rf_elf.rf_elf_stale_threshold_ms < 10.0 ||
       config.rf_elf.rf_elf_stale_threshold_ms > 200.0 {
        return Err(SettingsError::SchemaMismatch(
            format!("rf_elf_stale_threshold_ms {} out of range [10.0, 200.0]",
                    config.rf_elf.rf_elf_stale_threshold_ms)
        ));
    }

    Ok(())
}

fn validate_version(version_str: &str) -> Result<(u16, u16), SettingsError> {
    // Expected format: "3.3" or "3.1"
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() != 2 {
        return Err(SettingsError::SchemaMismatch(
            format!("Invalid version format: '{}'. Expected 'X.Y'", version_str)
        ));
    }

    let major = parts[0].parse::<u16>()
        .map_err(|_| SettingsError::SchemaMismatch(format!("Invalid major version")))?;
    let minor = parts[1].parse::<u16>()
        .map_err(|_| SettingsError::SchemaMismatch(format!("Invalid minor version")))?;

    // Support v3.1 and v3.3
    if major != 3 || (minor != 1 && minor != 3) {
        return Err(SettingsError::SchemaMismatch(
            format!("Unsupported version {}.{}. Expected 3.1 or 3.3", major, minor)
        ));
    }

    Ok((major, minor))
}

// ============================================================================
// HELPER: Fixed-Point Encoding (Q31.32)
// ============================================================================

pub fn f32_to_q31_32(f: f32) -> i64 {
    // Ensure finite
    if !f.is_finite() {
        return 0;
    }

    // Clamp to valid range [-2.0, 2.0]
    let clamped = f.clamp(-2.0, 2.0);

    // Multiply by 2^32 and convert to i64
    let scaled = (clamped * (1i64 << 32) as f32) as i64;

    // Saturate at boundaries
    scaled.saturating_add(0)  // Natural i64 bounds provide saturation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_frame_rate_valid() {
        assert!(validate_frame_rate(60).is_ok());
        assert!(validate_frame_rate(120).is_ok());
        assert!(validate_frame_rate(240).is_ok());
    }

    #[test]
    fn test_validate_frame_rate_invalid() {
        assert!(validate_frame_rate(59).is_err());
        assert!(validate_frame_rate(250).is_err());
        assert!(validate_frame_rate(0).is_err());
    }

    #[test]
    fn test_parse_rf_elf_address_valid() {
        assert_eq!(parse_rf_elf_address("0x7fff0000").unwrap(), 0x7fff0000);
        assert_eq!(parse_rf_elf_address("7fff0000").unwrap(), 0x7fff0000);  // without 0x
    }

    #[test]
    fn test_parse_rf_elf_address_null() {
        assert!(parse_rf_elf_address("0x0").is_err());
        assert!(parse_rf_elf_address("0").is_err());
    }

    #[test]
    fn test_parse_rf_elf_address_too_small() {
        assert!(parse_rf_elf_address("0x1000").is_err());  // < 1MB
    }

    #[test]
    fn test_q31_32_encoding() {
        assert_eq!(f32_to_q31_32(0.0), 0);
        assert_eq!(f32_to_q31_32(1.0), 1i64 << 32);
        assert_eq!(f32_to_q31_32(-1.0), -(1i64 << 32));
        assert_eq!(f32_to_q31_32(2.5).abs(), (2.0 * (1i64 << 32) as f32) as i64);  // Clamped to 2.0
