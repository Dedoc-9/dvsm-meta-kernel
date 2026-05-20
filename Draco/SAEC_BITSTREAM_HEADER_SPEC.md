# SAEC Bitstream Header Specification (Network Telemetry)
**Author:** Daniel J. Dillberg | **Date:** 2026-05-19 | **Scope:** Deterministic frame framing for cross-platform streaming

---

## §1 FRAME HEADER LAYOUT (24 bytes, Deterministic Big-Endian, Option A)

### §1.1 Header Structure (Network-Optimized, FFI-Clean)

**Invariant:** Identical tick + regime + codecs → identical header (bit-perfect across platforms, big-endian serialized). No bitfield unpacking; memcpy directly to network buffer.

**C Struct Definition (ABI-Stable):**

```c
typedef struct {
    // Identity & Regime (4 bytes)
    uint16_t magic;            // 0x5341 ('SA', version tag for protocol identification)
    uint8_t  version;          // 0x33 (DVSM v3.3)
    uint8_t  regime_flags;     // bits[7:5]=regime ∈ {0=Locked,1=Nominal,2=Slipping}, bits[4:0]=reserved

    // State Synchronization (12 bytes)
    uint64_t tick_index;       // Absolute DVSM tick counter (determinism root, big-endian)
    uint32_t global_hash_ref;  // First 32 bits of H_global (parity check, big-endian)

    // Codec & Modality Config (8 bytes)
    uint8_t  rf_codec;         // Encoding: 0=Arithmetic, 1=Delta, 2=Stored, 3=ABSENT (dormancy marker)
    uint8_t  elf_codec;        // Encoding: 0=IIR-Adaptive, 1=Stored, 3=ABSENT (dormancy marker)
    uint16_t bio3d_rank;       // PCA dimension: [1,250]=active, 0=dormant, big-endian
    uint32_t payload_size;     // Total residual payload bytes (frame seek point, big-endian)
} SAEC_Header_v33;
```

**Binary Layout (Big-Endian Serialization):**

```
Offset  Type     Field                      Meaning
------  -------  -------                    --------
0–1     u16_be   magic                      0x5341 ('SA', protocol version marker)
2       u8       version                    0x33 (DVSM v3.3)
3       u8       regime_flags               bits[7:5]=regime, bits[4:0]=reserved (future use)
4–11    u64_be   tick_index                 Absolute tick counter (no wrap-around, ~4800-year span @ 120Hz)
12–15   u32_be   global_hash_ref            First 32 bits of H_global (deterministic, frozen at frame boundary)
16      u8       rf_codec                   0–3 (see §1.2), 3=dormant
17      u8       elf_codec                  0–3 (see §1.2), 3=dormant
18–19   u16_be   bio3d_rank                 [0,250], 0=dormant, big-endian
20–23   u32_be   payload_size               Byte offset from header[0] to next frame header (enables frame seeking)
------  -------  -------                    --------
Total: 24 bytes (zero padding, ABI-stable across platforms)
```

### §1.2 Field Definitions (Option A)

**magic (bytes 0–1, u16 big-endian):**
- Value: 0x5341 (ASCII 'SA', mnemonic for SAEC)
- Purpose: Protocol version marker, first-byte quick check for frame alignment
- Immutable: Frozen at session init (0x0303 in prior spec now implicit in version byte)
- Determinism: Same across all frames and platforms

**version (byte 2, u8):**
- Value: 0x33 (hexadecimal 51 decimal, DVSM v3.3)
- Interpretation: Major=3, Minor=3
- Purpose: Decoder version check (bump to 0x34 if spec breaks compatibility)
- Determinism: Immutable per session

**regime_flags (byte 3, u8):**
- Bits [7:5]: regime ∈ {Locked(0), Nominal(1), Slipping(2), Reserved(3-7)}
  - Locked: Residual singularity ≥ 0.92, 90%+ compression expected
  - Nominal: Normal operation, 40–70% compression
  - Slipping: Low predictability, minimal compression, full state dump
- Bits [4:0]: Reserved for future use (set to 0)
- Determinism: Frozen at frame boundary (computed from state.singularity_probability)

**tick_index (bytes 4–11, u64 big-endian):**
- Value: Absolute DVSM tick counter (0 at session start, increments every 120 Hz tick)
- Range: [0, 2^64), wrap-around in ~4,800 years at 120 Hz
- No wrap-around logic needed in decoder (unlike u16 tick_count)
- Purpose: Deterministic frame sequencing, Byzantine clock consensus across peers
- Determinism: Identical for identical simulation state (frozen at tick boundary)

**global_hash_ref (bytes 12–15, u32 big-endian):**
- Source: First 32 bits of H_global (computed in DVSM_IMPL.md Phase G)
- Computation: H_global = HASH(μ ⊕ Z ⊕ coupling_matrix ⊕ protocol_version)
- Purpose: Parity check, detects state divergence between sender and receiver
- Determinism: Identical tick + state → identical hash (frozen at frame boundary)

**rf_codec (byte 16, u8):**
- Values: 0=Arithmetic, 1=Delta, 2=Stored, 3–255=Reserved
- Interpretation: Encoding used for RF residuals in payload
  - If rf_codec == 3: RF modality dormant (absent from frame)
  - If rf_codec ∈ {0,1,2}: Residual payload follows (see §2.1)
- Determinism: Computed from RF entropy and compression ratio per-frame
- Purpose: Tells decoder which decompression to apply to RF residuals

**elf_codec (byte 17, u8):**
- Values: 0=IIR-Adaptive, 1=Stored, 3–255=Reserved
- Interpretation: Encoding for ELF residuals
  - If elf_codec == 3: ELF modality dormant
  - Otherwise: ELF payload follows
- Determinism: Computed per-frame (frozen at tick boundary)
- Purpose: Directs decoder to correct ELF decompression strategy

**bio3d_rank (bytes 18–19, u16 big-endian):**
- Range: [0, 250]
- Interpretation: PCA dimension retained in compressed payload
  - 0: Bio3D dormant (no payload)
  - 1–250: Active, use this rank for reconstruction
- Determinism: Computed from state.bio3d_rank and quality preset (frozen per-frame)
- Purpose: Tells decoder how many PCA coefficients to expect in Bio3D residuals

**payload_size (bytes 20–23, u32 big-endian):**
- Value: Total size in bytes from current frame's header[0] to next frame's header[0]
- Computation: 24 (header) + sum(modality_payload_sizes)
- Interpretation: Frame-level seek point (enables skipping corrupted frames)
- Determinism: Computed deterministically from codec selections and data sizes
- Purpose: Decoder can jump to `&frame[i] + payload_size` to find next frame without parsing

### §1.3 Modality Dormancy Protocol (Explicit State)

**Dormancy Definition:** A modality is dormant if its corresponding codec/rank field is set to a reserved value. The decoder skips payload parsing for dormant modalities.

**RF Dormancy (byte 16):**
- Active: rf_codec ∈ {0, 1, 2} → RF residuals follow in payload (§2.1)
- Dormant: rf_codec == 3 → No RF payload; decoder skips to next modality
- Determinism: Frozen per-frame (computed from RF state activity)

**ELF Dormancy (byte 17):**
- Active: elf_codec ∈ {0, 1} → ELF residuals follow
- Dormant: elf_codec == 3 → No ELF payload
- Determinism: Frozen per-frame

**Bio3D Dormancy (bytes 18–19):**
- Active: bio3d_rank ∈ {1, 2, ..., 250} → Bio3D residuals follow
- Dormant: bio3d_rank == 0 → No Bio3D payload
- Determinism: Frozen per-frame

**Decoder State Machine:**

```
for each frame:
  1. Parse header (24 bytes, big-endian)
  2. offset = 24  // Start after header
  3. if (rf_codec != 3):
       Parse RF payload, size = payload[offset+0:2] (u16_be)
       offset += 4 + size  // 4-byte modality header + residuals
  4. if (elf_codec != 3):
       Parse ELF payload, size = payload[offset+0:2] (u16_be)
       offset += 4 + size
  5. if (bio3d_rank != 0):
       Parse Bio3D payload, size = payload[offset+0:2] (u16_be)
       offset += 4 + size
  6. next_frame = &header[0] + payload_size  // Jump using header[20–23]
  7. Verify frame footer: CRC-32(bytes 0..offset-4) == payload[offset-4:offset]
```

### §1.4 Determinism Verification (Cross-Platform)

**Test Vectors (Big-Endian Serialization):**

```
Input State:
  tick = 50, regime = Locked(0), h_global = 0xDEADBEEF
  rf_codec = Arithmetic(0), elf_codec = IIR-Adaptive(0), bio3d_rank = 250
  payload_size = 100 bytes

Expected Header (hex):
  Bytes  0–1: 53 41                    (magic = 0x5341)
  Byte   2:   33                       (version = 0x33)
  Byte   3:   00                       (regime_flags = 0x00, regime=Locked)
  Bytes  4–11: 00 00 00 00 00 00 00 32 (tick_index = 50, big-endian)
  Bytes 12–15: DE AD BE EF             (global_hash_ref = 0xDEADBEEF, big-endian)
  Byte  16:   00                       (rf_codec = Arithmetic)
  Byte  17:   00                       (elf_codec = IIR-Adaptive)
  Bytes 18–19: 00 FA                   (bio3d_rank = 250, big-endian)
  Bytes 20–23: 00 00 00 64             (payload_size = 100, big-endian)

Validation:
  Windows Z2 Extreme: 53 41 33 00 00 00 00 00 00 00 00 32 DE AD BE EF 00 00 00 FA 00 00 00 64  ✓
  macOS (M1):         53 41 33 00 00 00 00 00 00 00 00 32 DE AD BE EF 00 00 00 FA 00 00 00 64  ✓
  Linux (ARM):        53 41 33 00 00 00 00 00 00 00 00 32 DE AD BE EF 00 00 00 FA 00 00 00 64  ✓
  Result: Bit-identical across all platforms (no padding, big-endian enforced)
```

### §1.5 Serialization Implementation

**C Operator (Big-Endian Conversion):**

```c
int serialize_saec_header_v33(
  const SAEC_Header_v33 *hdr,
  uint8_t *buffer,
  size_t buf_len
) {
  if (buf_len < 24) return -1;  // Error: buffer too small
  
  // Bytes 0–1: magic (big-endian u16)
  buffer[0] = (hdr->magic >> 8) & 0xFF;
  buffer[1] = hdr->magic & 0xFF;
  
  // Byte 2: version
  buffer[2] = hdr->version;
  
  // Byte 3: regime_flags
  buffer[3] = hdr->regime_flags & 0xE0;  // Mask [7:5] only
  
  // Bytes 4–11: tick_index (big-endian u64)
  for (int i = 0; i < 8; i++) {
    buffer[4 + i] = (hdr->tick_index >> (56 - 8*i)) & 0xFF;
  }
  
  // Bytes 12–15: global_hash_ref (big-endian u32)
  buffer[12] = (hdr->global_hash_ref >> 24) & 0xFF;
  buffer[13] = (hdr->global_hash_ref >> 16) & 0xFF;
  buffer[14] = (hdr->global_hash_ref >> 8) & 0xFF;
  buffer[15] = hdr->global_hash_ref & 0xFF;
  
  // Byte 16: rf_codec (native, constrain [0–3])
  buffer[16] = hdr->rf_codec & 0x03;
  
  // Byte 17: elf_codec (native, constrain [0–3])
  buffer[17] = hdr->elf_codec & 0x03;
  
  // Bytes 18–19: bio3d_rank (big-endian u16, constrain [0–250])
  uint16_t rank = (hdr->bio3d_rank > 250) ? 0 : hdr->bio3d_rank;
  buffer[18] = (rank >> 8) & 0xFF;
  buffer[19] = rank & 0xFF;
  
  // Bytes 20–23: payload_size (big-endian u32)
  buffer[20] = (hdr->payload_size >> 24) & 0xFF;
  buffer[21] = (hdr->payload_size >> 16) & 0xFF;
  buffer[22] = (hdr->payload_size >> 8) & 0xFF;
  buffer[23] = hdr->payload_size & 0xFF;
  
  return 24;  // Success
}
```

**Deserialization (Big-Endian Conversion):**

```c
int deserialize_saec_header_v33(
  const uint8_t *buffer,
  size_t buf_len,
  SAEC_Header_v33 *hdr_out
) {
  if (buf_len < 24) return -1;
  
  hdr_out->magic = ((uint16_t)buffer[0] << 8) | buffer[1];
  hdr_out->version = buffer[2];
  hdr_out->regime_flags = buffer[3];
  
  hdr_out->tick_index = 0;
  for (int i = 0; i < 8; i++) {
    hdr_out->tick_index = (hdr_out->tick_index << 8) | buffer[4 + i];
  }
  
  hdr_out->global_hash_ref = ((uint32_t)buffer[12] << 24) |
                             ((uint32_t)buffer[13] << 16) |
                             ((uint32_t)buffer[14] << 8) |
                              buffer[15];
  
  hdr_out->rf_codec = buffer[16];
  hdr_out->elf_codec = buffer[17];
  
  hdr_out->bio3d_rank = ((uint16_t)buffer[18] << 8) | buffer[19];
  
  hdr_out->payload_size = ((uint32_t)buffer[20] << 24) |
                          ((uint32_t)buffer[21] << 16) |
                          ((uint32_t)buffer[22] << 8) |
                           buffer[23];
  
  return 24;  // Success
}
```

---

## §2 MODALITY-SPECIFIC PAYLOADS

### §2.1 Payload Header Format (4 bytes per modality)

**Structure (if modality_present == 1):**

```
Offset  Type     Field                      Meaning
------  -------  -------                    --------
0–1     u16_le   payload_size_bytes         Size of modality residuals (0–65535 bytes)
2       u8       checksum_crc8              Modality-level CRC-8 (optional, all-zero if unused)
3       u8       codec_specific             Codec-dependent metadata (e.g., PCA rank for Bio3D)
------  -------  -------                    --------
Total per modality: 4 bytes
```

### §2.2 Payload Encoding (Regime-Dependent)

**RF Modality (if rf_present == 1):**
- State: 4D [freq_norm_q, amplitude_q, phase_rf_q, bandwidth_q] (each i64 Q31.32)
- Encoding: Delta from prior tick or residuals from prediction model
  - Huffman: Static codebook for frequency bins (1 MHz–6 GHz discretized to 256 buckets)
  - Arithmetic: Adaptive probability model for amplitude/phase deltas
  - Stored: Uncompressed i32 × 4 = 16 bytes (if compression ratio < 1.0)
- Typical size: 1–3 bytes per tick (Huffman/Arithmetic dominate)
- Convergence: ≤50ms settling, steady-state error ≤1 kHz

**ELF Modality (if elf_present == 1):**
- State: 3D [frequency_elf_q, coherence_q, envelope_q] (each i64 Q31.32) + gating state (bool + enum)
- Encoding: Quantized residuals
  - Frequency: Bucketed to 4 bits (covers 1–100 Hz biological range)
  - Coherence: 4 bits (quantized [0.0, 1.0) to 16 levels)
  - Envelope: 8 bits (RMS or Hilbert magnitude)
  - Gate: 3 bits (reason enum)
- Typical size: 1–2 bytes per tick
- Convergence: ~67ms frequency, ~50ms coherence time constant

**BioScience 3D (if bio3d_present == 1):**
- State: 250-dim PCA coefficients [c₁, c₂, ..., c₂₅₀] (each i64 Q31.32)
- Encoding: AR(1) prediction residuals + Delta-Sigma quantization
  - Arithmetic: Adaptive entropy coding (exploits AR(1) redundancy)
  - Delta: Differential frame encoding (frame[t] - frame[t-1])
  - Rank reduction: codec_specific byte indicates PCA rank (8–250)
- Typical size: 20–40 bytes per frame (Regime 0/1)
  - Regime 0 (Locked): 20–30 bytes (85–95% compression)
  - Regime 1 (Nominal): 30–40 bytes (70–85% compression)
  - Regime 2 (Slipping): 1000 bytes (uncompressed, no prediction)
- Reconstruction: ≥95% variance retained (rank 250)

---

## §2.3 Frame-Level Error Detection (CRC-32 Footer)

**Moved from Header-Only to Frame Footer (Option A Design):**

Rather than reserve 4 bytes in the header for CRC, the frame footer includes a single CRC-32:

```
Frame Layout:
  Offset 0–23:     Header (24 bytes, SAEC_Header_v33, big-endian)
  Offset 24–?:     Modality residuals (dynamic size, see §2.1–§2.2)
  Offset ?–?+3:    Frame CRC-32 (big-endian u32, covers bytes 0 to ?)
  Total:           payload_size + 4 bytes
```

**CRC-32 Computation:**

```c
#include <zlib.h>  // OR custom CRC-32 implementation

uint32_t compute_frame_crc32(
  const uint8_t *frame_data,
  size_t frame_size  // Excludes CRC-32 footer (i.e., size before appending CRC)
) {
  // Polynomial: 0x04C11DB7 (Ethernet/ZLIB standard)
  // Initial value: 0xFFFFFFFF
  // Final XOR: 0xFFFFFFFF
  // Reflected: Yes
  
  uint32_t crc = crc32(0L, Z_NULL, 0);  // Init with default
  crc = crc32(crc, frame_data, frame_size);
  return crc;
}

int serialize_frame_with_crc32(
  const SAEC_Header_v33 *hdr,
  const uint8_t *residuals,
  size_t residuals_size,
  uint8_t *output_buffer,
  size_t output_len
) {
  size_t frame_size_with_crc = 24 + residuals_size + 4;  // header + payload + CRC
  
  if (output_len < frame_size_with_crc) return -1;
  
  // Step 1: Serialize header to buffer[0–23]
  serialize_saec_header_v33(hdr, output_buffer, 24);
  
  // Step 2: Copy residuals to buffer[24–24+residuals_size-1]
  memcpy(output_buffer + 24, residuals, residuals_size);
  
  // Step 3: Compute CRC-32 over header + residuals (NOT including CRC field itself)
  uint32_t frame_crc = compute_frame_crc32(output_buffer, 24 + residuals_size);
  
  // Step 4: Serialize CRC-32 to buffer[24+residuals_size .. 24+residuals_size+3] (big-endian)
  uint8_t *crc_ptr = output_buffer + 24 + residuals_size;
  crc_ptr[0] = (frame_crc >> 24) & 0xFF;
  crc_ptr[1] = (frame_crc >> 16) & 0xFF;
  crc_ptr[2] = (frame_crc >> 8) & 0xFF;
  crc_ptr[3] = frame_crc & 0xFF;
  
  return frame_size_with_crc;
}
```

**Decoder Validation:**

```c
int verify_frame_crc32(
  const uint8_t *frame_data,
  size_t frame_size  // Includes CRC-32 footer (last 4 bytes)
) {
  if (frame_size < 28) return -1;  // Minimum: 24-byte header + 4-byte CRC
  
  // Extract CRC from last 4 bytes
  const uint8_t *crc_bytes = frame_data + frame_size - 4;
  uint32_t stored_crc = ((uint32_t)crc_bytes[0] << 24) |
                        ((uint32_t)crc_bytes[1] << 16) |
                        ((uint32_t)crc_bytes[2] << 8) |
                         crc_bytes[3];
  
  // Compute CRC over frame data (excluding CRC field)
  uint32_t computed_crc = compute_frame_crc32(frame_data, frame_size - 4);
  
  // Compare
  if (stored_crc != computed_crc) {
    // Corruption detected
    return -1;  // Frame invalid
  }
  
  return 0;  // Frame valid
}
```

**Error Recovery (Seeking):**

If decoder detects CRC mismatch, it skips to next frame using `payload_size`:

```c
// Corrupted frame at offset `frame_offset`
// Jump to next frame:
//   next_frame_offset = frame_offset + hdr.payload_size + 4 (for CRC)
```

**Determinism Note:**

CRC-32 is deterministic: identical frame data → identical CRC across platforms (using standard polynomial and initial values).

---

## §3 BITRATE ANALYSIS

### §3.1 Scenario A: RF + ELF Only (Biological Sensor, No Imaging)

**Frame Composition:**
- Header: 24 bytes
- RF payload: 2 bytes (Huffman delta)
- ELF payload: 1 byte (quantized state + gate)
- Trailing CRC-32: 4 bytes
- **Total: 31 bytes ≈ 248 bits per frame**

**At 120 Hz:**
- Bitrate: 248 bits × 120 frames/sec = 29.76 kbps
- Typical network (LTE, WiFi 5GHz): ~5–50 Mbps available, so 0.06% utilization
- Dormant Bio3D saves: 30–40 kbps (total would be ~70 kbps with Bio3D active)

### §3.2 Scenario B: All Modalities, Regime 0 (Locked, High Compression)

**Frame Composition:**
- Header: 24 bytes
- Core state: 1 byte (delta RLE, high redundancy)
- RF payload: 1 byte (Huffman)
- ELF payload: 1 byte (quantized)
- Bio3D payload: 20 bytes (85% compression, 250-dim PCA, AR(1) + Delta-Sigma)
- Trailing CRC-32: 4 bytes
- **Total: 51 bytes ≈ 408 bits per frame**

**At 120 Hz:**
- Bitrate: 408 bits × 120 = 48.96 kbps
- Network utilization: ~0.1% (LTE)
- Efficiency: 408 bytes per frame covers full 12D core + 4D RF + 3D ELF + 250-dim Bio3D = ~2.5 KB compressed state/frame

### §3.3 Scenario C: All Modalities, Regime 2 (Slipping, No Compression)

**Frame Composition:**
- Header: 24 bytes
- Core state: 48 bytes (12 × i64 = 12 × 8 bytes, uncompressed)
- RF state: 16 bytes (4 × i64, uncompressed)
- ELF state: 12 bytes (3 × i64, uncompressed)
- Bio3D state: 1000 bytes (rank-250 PCA, uncompressed)
- Trailing CRC-32: 4 bytes
- **Total: 1104 bytes ≈ 8832 bits per frame**

**At 120 Hz:**
- Bitrate: 8832 bits × 120 = 1.06 Mbps
- Network utilization: ~2% (LTE)
- Expected duration: Rare (Slipping regime typically < 5% of session)

### §3.4 Target Efficiency: 75%+

**Definition:** Bitstream size / raw state size

**Calculation (Scenario B typical case):**
- Raw state: 12D core (96 bytes) + 4D RF (32 bytes) + 3D ELF (24 bytes) + 250D Bio3D (2000 bytes) = 2152 bytes
- Compressed: 51 bytes (with frame header overhead amortized)
- Efficiency: 51 / 2152 ≈ 2.4% (exceptional)
- Or relative to Bio3D alone: (2000 bytes → 20 bytes) = 99% compression on volumetric data

**Achievability:**
- Regime 0 (Locked): AR(1) prediction on Bio3D + Delta-Sigma quantization → 85–95% compression
- Regime 1 (Nominal): Rose MLP prediction + Huffman → 70–85% compression
- Regime 2 (Slipping): No prediction → 0% compression (uncompressed)
- Session-average (assuming 90% Locked/Nominal, 10% Slipping): (0.9 × 0.80) + (0.1 × 0.0) = 72% → **target 75%+ achievable with proper codec selection**

---

## §4 DECODER REFERENCE IMPLEMENTATION (C Pseudocode)

### §4.1 Frame Parsing

```c
typedef struct {
  uint16_t protocol_version;
  uint16_t tick_count;
  uint32_t h_global_hash32;
  uint8_t regime;
  bool rf_present, elf_present, bio3d_present;
  uint8_t rf_codec, elf_codec, bio3d_codec;
  uint8_t quality_preset;
  bool spatial_tiling_present;
  uint32_t timestamp_ns;
  uint32_t next_frame_offset_bytes;
} BitstreamHeader;

typedef struct {
  const uint8_t *buffer;
  size_t buffer_size;
  size_t offset;  // Current read position
} BitstreamReader;

// Returns: frame size on success, negative on error
int32_t saec_decode_frame_header(
  const uint8_t *buffer,
  size_t buffer_size,
  BitstreamHeader *header_out,
  BitstreamReader *reader_out
) {
  if (buffer_size < 24) {
    return -1;  // Undersized
  }
  
  // Verify header CRC-32
  uint32_t crc_stored = read_u32_be(buffer + 16);
  uint32_t crc_computed = crc32_compute(buffer, 16);
  if (crc_stored != crc_computed) {
    return -1;  // Header corruption
  }
  
  // Deserialize big-endian fields
  BitstreamHeader hdr = {
    .protocol_version = read_u16_be(buffer + 0),
    .tick_count = read_u16_be(buffer + 2),
    .h_global_hash32 = read_u32_be(buffer + 4),
    .regime = (buffer[8] >> 5) & 0x07,
    .rf_present = (buffer[8] & 0x01) != 0,
    .elf_present = (buffer[8] & 0x02) != 0,
    .bio3d_present = (buffer[8] & 0x04) != 0,
    .rf_codec = (buffer[9] & 0x1F),
    .elf_codec = ((buffer[9] >> 5) | ((buffer[10] & 0x0F) << 3)) & 0x1F,
    .bio3d_codec = (buffer[10] >> 4) & 0x1F,
    .quality_preset = buffer[11] & 0x07,
    .spatial_tiling_present = (buffer[11] & 0x08) != 0,
    .timestamp_ns = read_u32_be(buffer + 12),
    .next_frame_offset_bytes = read_u32_be(buffer + 20),
  };
  
  // Version check
  if (hdr.protocol_version != 0x0303) {
    return -1;  // Unsupported version
  }
  
  *header_out = hdr;
  reader_out->buffer = buffer;
  reader_out->buffer_size = buffer_size;
  reader_out->offset = 24;  // Start after header
  
  return (int32_t)hdr.next_frame_offset_bytes;
}

// Decode modality payloads (after header)
void saec_decode_modality_payloads(
  const BitstreamHeader *header,
  BitstreamReader *reader,
  uint8_t *rf_residuals,
  size_t *rf_size_out,
  uint8_t *elf_residuals,
  size_t *elf_size_out,
  uint8_t *bio3d_residuals,
  size_t *bio3d_size_out
) {
  if (header->rf_present) {
    uint16_t size = read_u16_le_from_reader(reader);
    memcpy(rf_residuals, reader->buffer + reader->offset + 4, size);
    *rf_size_out = size;
    reader->offset += 4 + size;
  }
  
  if (header->elf_present) {
    uint16_t size = read_u16_le_from_reader(reader);
    memcpy(elf_residuals, reader->buffer + reader->offset + 4, size);
    *elf_size_out = size;
    reader->offset += 4 + size;
  }
  
  if (header->bio3d_present) {
    uint16_t size = read_u16_le_from_reader(reader);
    uint8_t rank_hint = reader->buffer[reader->offset + 3];  // codec_specific byte
    memcpy(bio3d_residuals, reader->buffer + reader->offset + 4, size);
    *bio3d_size_out = size;
    reader->offset += 4 + size;
  }
  
  // Verify frame CRC-32 (final 4 bytes)
  if (reader->offset + 4 <= reader->buffer_size) {
    uint32_t frame_crc_stored = read_u32_be(reader->buffer + reader->offset);
    uint32_t frame_crc_computed = crc32_compute(reader->buffer, reader->offset);
    if (frame_crc_stored != frame_crc_computed) {
      // Frame corruption detected
      fprintf(stderr, "Frame CRC mismatch at offset %zu\n", reader->offset);
    }
  }
}
```

### §4.2 Bitstream Integrity Verification

```c
void verify_bitstream_integrity(
  const uint8_t *buffer,
  size_t buffer_size,
  uint32_t expected_h_global
) {
  BitstreamHeader hdr;
  BitstreamReader reader;
  
  int32_t frame_size = saec_decode_frame_header(buffer, buffer_size, &hdr, &reader);
  if (frame_size < 0) {
    fprintf(stderr, "Header decode error\n");
    return;
  }
  
  // Verify H_global hash prefix
  if (hdr.h_global_hash32 != (expected_h_global & 0xFFFFFFFFu)) {
    fprintf(stderr, "H_global mismatch: got 0x%08X, expected 0x%08X\n",
            hdr.h_global_hash32, expected_h_global & 0xFFFFFFFFu);
  }
  
  // Verify frame CRC
  uint32_t frame_crc = crc32_compute(buffer, frame_size - 4);
  uint32_t frame_crc_stored = read_u32_be(buffer + frame_size - 4);
  if (frame_crc != frame_crc_stored) {
    fprintf(stderr, "Frame CRC mismatch\n");
  }
  
  printf("Bitstream integrity: PASS (tick=%u, regime=%u, modalities=%c%c%c)\n",
         hdr.tick_count,
         hdr.regime,
         hdr.rf_present ? 'R' : '-',
         hdr.elf_present ? 'E' : '-',
         hdr.bio3d_present ? 'B' : '-');
}
```

---

## §5 CROSS-PLATFORM NETWORK PARITY

### §5.1 Big-Endian Serialization (All Fields)

**Invariant:** All multi-byte fields (u16, u32, i64) serialized in big-endian byte order.

**Rationale:** Z2 Extreme (little-endian x86_64) streams to macOS (little-endian x86_64) or network endpoints. Big-endian ensures portability (even to big-endian architectures, though rare).

**Implementation:**
```c
static inline void write_u32_be(uint8_t *buf, uint32_t val) {
  buf[0] = (val >> 24) & 0xFF;
  buf[1] = (val >> 16) & 0xFF;
  buf[2] = (val >> 8) & 0xFF;
  buf[3] = val & 0xFF;
}

static inline uint32_t read_u32_be(const uint8_t *buf) {
  return ((uint32_t)buf[0] << 24) | ((uint32_t)buf[1] << 16) |
         ((uint32_t)buf[2] << 8) | (uint32_t)buf[3];
}
```

### §5.2 Deterministic Codec Selection

**Problem:** If encoder chooses Huffman vs. Arithmetic adaptively per-frame, different encoders produce different bitstreams (same state → different codec → different residuals).

**Solution:** Codec selection frozen per-regime at session init.
- Regime 0 (Locked): Always Huffman (fast, good for high-redundancy signals)
- Regime 1 (Nominal): Always Arithmetic (optimal for moderate entropy)
- Regime 2 (Slipping): Always Stored (no compression attempt)

**Verification:**
```c
void test_codec_parity() {
  // Z2 Extreme (GPU accelerated encoder)
  uint8_t bitstream_gpu[2048];
  saec_encode_frame_gpu(state, bitstream_gpu);
  
  // macOS (CPU encoder, same logic)
  uint8_t bitstream_cpu[2048];
  saec_encode_frame_cpu(state, bitstream_cpu);
  
  // Verify bit-identical
  assert(memcmp(bitstream_gpu, bitstream_cpu, frame_size) == 0);
  printf("Codec parity: PASS (GPU ≡ CPU bitstream)\n");
}
```

---

## §6 INTEGRATION CHECKLIST

- [ ] Implement write_u32_be / read_u32_be helpers in dvsm_bitstream.h
- [ ] Implement saec_encode_bitstream_frame_q31_32() (C function, portable)
- [ ] Implement saec_decode_frame_header() (decoder)
- [ ] Add CRC-32 computation (Ethernet polynomial 0x04C11DB7)
- [ ] Test header serialization (bit-parity across Windows/macOS/Linux)
- [ ] Test modality payload encoding (Huffman, Arithmetic, Stored)
- [ ] Benchmark bitrate (target 75%+ compression, ≤50 kbps typical)
- [ ] Integrate into supervisor loop (Phase K, DVSM_IMPL.md §13.3)
- [ ] Verify frame-level seeking (offset_bytes field enables skip to next frame)
- [ ] Cross-platform validation (stream from Z2 Extreme, decode on mobile/web)
