/// src/molecular/mod.rs
///
/// V3.4 Molecular Dynamics Layer: Allosteric Torsion Injection
///
/// **Responsibility:** Validate and inject pre-parsed protein backbone dihedral angles
/// (φ/ψ torsion angles) into the supervisor deterministic core.
///
/// **Design Pattern (Contract-First):**
/// - TorsionArray struct: 740-byte FFI contract (fixed-width, ABI-stable)
/// - inject_torsion_array: Supervisor validator (grounds the contract)
/// - Supervisor is the "Ground Truth"; Python parser is the client
///
/// **Alignment:** TorsionArray is NOT aligned (packed), supervisor handles the layout
///
/// **Determinism (ASIL D):**
/// - All validation deterministic (no RNG, no branching on input data)
/// - Q31.32 fixed-point conversion precise and repeatable
/// - CRC32 informational only (non-fatal on mismatch)

pub mod torsion_injection;
pub mod allosteric_kernel;

pub use torsion_injection::{inject_torsion_array, crc32_checksum};
pub use allosteric_kernel::apply_allosteric_coupling;

// ============================================================================
// TORSION ARRAY FFI CONTRACT (740 bytes, Layout-ID binding)
// ============================================================================

/// FFI-compatible torsion angle array structure
///
/// **Layout (740 bytes):**
/// - Bytes 0-719:    angles[180] (f32 × 180 = 720 bytes)
/// - Byte 720:       sequence_length (u8, ≤ 90)
/// - Byte 721:       source_flags (u8)
/// - Bytes 722-725:  pdb_id[4] (char × 4)
/// - Bytes 726-733:  timestamp_us (u64)
/// - Bytes 734-737:  crc32 (u32)
/// - Bytes 738-739:  _padding (u16)
///
/// **Invariants:**
/// - sequence_length must be ≤ 90 (hard constraint)
/// - angles must be normalized to [-π, π] (advisory, validated by supervisor)
/// - CRC32 computed over angles[0..sequence_length*2] only
/// - Layout-ID would be 0x8F3E1A9C (from v3.4 spec, for future binding)
///
/// **C Binding:**
/// ```c
/// typedef struct {
///     float angles[180];
///     uint8_t sequence_length;
///     uint8_t source_flags;
///     uint8_t pdb_id[4];
///     uint64_t timestamp_us;
///     uint32_t crc32;
///     uint16_t _padding;
/// } TorsionArray;
/// ```
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct TorsionArray {
    /// 90 residues × 2 angles (φ/ψ) = 180 f32 values
    /// Index mapping: angles[2*i] = φ_i, angles[2*i+1] = ψ_i
    /// Range: [-π, π] (normalized dihedral angles in radians)
    /// Q31.32 representation in supervisor (converted from f32 by scaling 2^32)
    pub angles: [f32; 180],

    /// Actual number of residues extracted (≤ 90)
    /// Used to determine active region of angles array
    /// Residues 91+ are ignored (hard truncation at 90)
    pub sequence_length: u8,

    /// Metadata flags:
    ///   Bit 0: from_pdb (1=extracted from PDB, 0=synthetic/computed)
    ///   Bit 1: has_disorder (1=truncated or incomplete, 0=complete)
    ///   Bits 2-7: reserved (must be zero)
    pub source_flags: u8,

    /// PDB code (4-character ASCII) or all zeros if synthetic
    /// Example: 0x31, 0x4D, 0x42, 0x4E = "1MBN" (little-endian storage)
    /// Informational only (not validated by supervisor)
    pub pdb_id: [u8; 4],

    /// Parser execution timestamp (microseconds since epoch)
    /// Used for freshness tracking and forensic logging
    /// Optional: can be zero if not available
    pub timestamp_us: u64,

    /// CRC32 checksum computed over active angles only
    /// Computed as: CRC32(angles[0..sequence_length*2])
    /// Non-fatal if mismatch detected (informational warning only)
    pub crc32: u32,

    /// Reserved padding for alignment (currently unused, must be zero)
    pub _padding: u16,
}

impl TorsionArray {
    /// Construct a default (zero-initialized) TorsionArray
    pub fn new() -> Self {
        TorsionArray {
            angles: [0.0; 180],
            sequence_length: 0,
            source_flags: 0,
            pdb_id: [0; 4],
            timestamp_us: 0,
            crc32: 0,
            _padding: 0,
        }
    }

    /// Byte size verification (must be exactly 740 bytes for ABI compatibility)
    pub const SIZE_BYTES: usize = 740;
}

impl Default for TorsionArray {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// COMPILE-TIME ABI VERIFICATION
// ============================================================================

#[cfg(test)]
mod abi_verification {
    use super::*;
    use std::mem::{size_of, align_of, offset_of};

    #[test]
    fn test_torsion_array_size() {
        assert_eq!(
            size_of::<TorsionArray>(),
            740,
            "TorsionArray must be exactly 740 bytes for FFI compatibility"
        );
    }

    #[test]
    fn test_torsion_array_alignment() {
        // TorsionArray should NOT have any special alignment
        // (supervisor handles alignment of the buffer containing it)
        // But f32 fields must be at least 4-byte aligned
        assert!(align_of::<TorsionArray>() <= 4);
    }

    #[test]
    fn test_angles_offset() {
        // angles[0] should start at offset 0
        let t = TorsionArray::new();
        let t_addr = &t as *const _ as usize;
        let angles_addr = &t.angles as *const _ as usize;
        assert_eq!(angles_addr - t_addr, 0, "angles must be first field at offset 0");
    }

    #[test]
    fn test_sequence_length_offset() {
        // sequence_length should be at offset 720 (720 bytes = 180 × f32)
        let t = TorsionArray::new();
        let t_addr = &t as *const _ as usize;
        let seq_len_addr = &t.sequence_length as *const _ as usize;
        assert_eq!(seq_len_addr - t_addr, 720, "sequence_length at offset 720");
    }

    #[test]
    fn test_timestamp_offset() {
        // timestamp_us should be at offset 726
        let t = TorsionArray::new();
        let t_addr = &t as *const _ as usize;
        let ts_addr = &t.timestamp_us as *const _ as usize;
        assert_eq!(ts_addr - t_addr, 726, "timestamp_us at offset 726");
    }

    #[test]
    fn test_crc32_offset() {
        // crc32 should be at offset 734
        let t = TorsionArray::new();
        let t_addr = &t as *const _ as usize;
        let crc_addr = &t.crc32 as *const _ as usize;
        assert_eq!(crc_addr - t_addr, 734, "crc32 at offset 734");
    }
}
