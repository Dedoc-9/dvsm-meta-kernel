/// RF/ELF External Modality Integration (Model B)
/// Level 2 Contract Definition — Tier 2 Implementation
///
/// Date: 2026-05-21
/// Specification: RF_ELF_INTEGRATION_SPEC.md
/// Status: Phase I.0.5 Supervisor Hook (Day 3)
///
/// Module Overview:
/// - RfElfSample: 64-byte aligned external modality data structure
/// - RfElfBuffer: Non-blocking SPSC ring buffer interface (user-provided)
/// - Error codes: Fail-fast vs. non-fatal semantics
/// - Layout-ID: 0x8F3E1A9C (ABI drift detection)

use std::fmt;

/// RF/ELF External Modality Sample (64-byte aligned)
/// Layout-ID: 0x8F3E1A9C
///
/// Produced by external RF/ELF modality producer (Core 1+)
/// Consumed by DVSM supervisor (Core 0) via try_pop()
///
/// Total size: 64 bytes (perfectly aligned to cache line)
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug)]
pub struct RfElfSample {
    // Header (16 bytes)
    pub timestamp_us: u64,           // Microsecond timestamp (producer's clock)
    pub sample_id: u32,              // Monotonic counter (overflow checked)
    pub flags: u16,                  // Bit 0: valid, Bit 1: stale, Bits [2-15]: reserved
    pub payload_size: u16,           // Bytes of modality data (0-48)

    // RF Modality (24 bytes)
    pub rf_phase: f32,               // RF phase angle [0, 2π]
    pub rf_amplitude: f32,           // RF signal strength
    pub rf_frequency: f32,           // RF center frequency (Hz)
    pub rf_bandwidth: f32,           // RF bandwidth (Hz)
    pub rf_reserved1: f32,           // Reserved for v3.4
    pub rf_reserved2: f32,           // Reserved for v3.4

    // ELF Modality (24 bytes)
    pub elf_power_density: f32,      // ELF power (W/m²)
    pub elf_frequency: f32,          // ELF center frequency (Hz)
    pub elf_phase: f32,              // ELF phase angle
    pub elf_coherence: f32,          // Coherence metric [0, 1]
    pub elf_reserved1: f32,          // Reserved for v3.4
    pub elf_reserved2: f32,          // Reserved for v3.4
}

// Total: 16 + 24 + 24 = 64 bytes (perfectly aligned)

impl RfElfSample {
    /// Create default sample (all zeros, valid=false)
    pub fn new() -> Self {
        RfElfSample {
            timestamp_us: 0,
            sample_id: 0,
            flags: 0,
            payload_size: 0,
            rf_phase: 0.0,
            rf_amplitude: 0.0,
            rf_frequency: 0.0,
            rf_bandwidth: 0.0,
            rf_reserved1: 0.0,
            rf_reserved2: 0.0,
            elf_power_density: 0.0,
            elf_frequency: 0.0,
            elf_phase: 0.0,
            elf_coherence: 0.0,
            elf_reserved1: 0.0,
            elf_reserved2: 0.0,
        }
    }

    /// Check if sample is marked valid
    pub fn is_valid(&self) -> bool {
        (self.flags & 0x01) != 0
    }

    /// Check if sample is marked stale
    pub fn is_stale(&self) -> bool {
        (self.flags & 0x02) != 0
    }

    /// Set valid flag
    pub fn set_valid(&mut self) {
        self.flags |= 0x01;
    }

    /// Set stale flag
    pub fn set_stale(&mut self) {
        self.flags |= 0x02;
    }

    /// Get size in bytes (for verification)
    pub fn size_bytes(&self) -> usize {
        std::mem::size_of::<RfElfSample>()
    }
}

impl Default for RfElfSample {
    fn default() -> Self {
        Self::new()
    }
}

/// RF/ELF error codes (fail-fast vs. non-fatal semantics)
#[derive(Debug, Clone)]
pub enum RfElfError {
    // Fail-Fast (initialization only, session becomes invalid)
    BufferMissing,              // No buffer provided at init
    InvalidCapacity,            // Buffer too small (< 128 samples)
    LayoutIdMismatch,          // User ABI drifted (Layout-ID != 0x8F3E1A9C)

    // Non-Fatal (runtime, graceful degradation)
    Empty,                      // No sample ready (OK, try next frame)
    Stale,                      // Sample age > 8333 μs (> 1 frame at 120 Hz)
    BufferOverflow,            // Producer wrote faster than consumer
    TimestampInvalid,          // Timestamp went backward
    PayloadMismatch,           // Reported size != actual data
}

impl fmt::Display for RfElfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RfElfError::BufferMissing => write!(f, "ERR_MODALITY_MISSING"),
            RfElfError::InvalidCapacity => write!(f, "ERR_MODALITY_INVALID_CAPACITY"),
            RfElfError::LayoutIdMismatch => write!(f, "ERR_MODALITY_CORRUPTED"),
            RfElfError::Empty => write!(f, "ERR_RF_ELF_EMPTY"),
            RfElfError::Stale => write!(f, "ERR_RF_ELF_STALE"),
            RfElfError::BufferOverflow => write!(f, "ERR_RF_ELF_OVERFLOW"),
            RfElfError::TimestampInvalid => write!(f, "ERR_RF_ELF_TIMESTAMP_INVALID"),
            RfElfError::PayloadMismatch => write!(f, "ERR_RF_ELF_PAYLOAD_MISMATCH"),
        }
    }
}

/// RF/ELF ring buffer interface (user-provided)
///
/// Producer-Consumer pattern (SPSC: Single Producer, Single Consumer)
/// - Core 0 (supervisor): consumer (calls try_pop)
/// - Core 1+ (user): producer (writes samples)
///
/// Non-blocking, lock-free interface. Supervisor must never wait.
pub trait RfElfBuffer {
    /// Try to pop one sample from the ring buffer
    /// Non-blocking, lock-free
    ///
    /// Returns:
    ///   Ok(sample) if one was available
    ///   Err(RfElfError::Empty) if no sample ready
    ///   Err(RfElfError::Stale) if sample age > MAX_STALE_US
    ///   Err(RfElfError::BufferOverflow) if producer wrote faster than consumer
    fn try_pop(&mut self) -> Result<RfElfSample, RfElfError>;

    /// Get the current write position (for diagnostics)
    fn write_position(&self) -> u64;

    /// Get the current read position (for diagnostics)
    fn read_position(&self) -> u64;

    /// Layout-ID verification (expected: 0x8F3E1A9C)
    fn layout_id(&self) -> u32;
}

/// Constants (from spec section 6)
pub const MAX_STALE_US: u64 = 8333;            // 1 frame at 120 Hz = 8.33 ms
pub const LAYOUT_ID_RF_ELF: u32 = 0x8F3E1A9C;  // RfElfSample struct signature

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rf_elf_sample_size() {
        let sample = RfElfSample::new();
        assert_eq!(sample.size_bytes(), 64);
    }

    #[test]
    fn test_rf_elf_sample_alignment() {
        // Verify 64-byte alignment at compile time
        let sample = RfElfSample::new();
        let addr = &sample as *const _ as usize;
        assert_eq!(addr % 64, 0, "RfElfSample must be 64-byte aligned");
    }

    #[test]
    fn test_rf_elf_sample_flags() {
        let mut sample = RfElfSample::new();
        assert!(!sample.is_valid());
        assert!(!sample.is_stale());

        sample.set_valid();
        assert!(sample.is_valid());
        assert!(!sample.is_stale());

        sample.set_stale();
        assert!(sample.is_valid());
        assert!(sample.is_stale());
    }

    #[test]
    fn test_rf_elf_error_display() {
        let err = RfElfError::LayoutIdMismatch;
        assert_eq!(format!("{}", err), "ERR_MODALITY_CORRUPTED");
    }

    #[test]
    fn test_layout_id_constant() {
        assert_eq!(LAYOUT_ID_RF_ELF, 0x8F3E1A9C);
    }

    #[test]
    fn test_max_stale_us_constant() {
        assert_eq!(MAX_STALE_US, 8333);
    }
}
