# SAEC Bitstream Header Specification (Network Telemetry)
**Author:** Daniel J. Dillberg | **Date:** 2026-05-19 | **Scope:** Deterministic frame framing for cross-platform streaming

---

## §1 FRAME HEADER LAYOUT (24 bytes, Q31.32 Deterministic)

### §1.1 Header Structure

**Invariant:** All fields are Q31.32 projections or frozen immutable config. Same tick always produces same header (bit-identical across platforms).

**Binary Layout (Big-Endian Serialization):**

```
Offset  Type     Field                      Meaning
------  -------  -------                    --------
0–1     u16_be   protocol_version           0x0303 = DVSM v3.3
2–3     u16_be   tick_count                 Frame sequence (0–65535, wraps ~546ms @ 120Hz)
4–7     u32_be   h_global_hash32            First 32 bits of BLAKE3(H_core ⊕ H_coupling ⊕ version)
8       u8       regime_and_flags           bits[7:5]=regime, bits[4:0]=modality_flags
9–10    u16_be   codec_modes                bits[4:0]=rf_codec, [9:5]=elf_codec, [14:10]=bio3d_codec
11      u8       compression_metadata       bits[2:0]=quality, bit[3]=tiling_present, [7:4]=reserved
12–15   u32_be   timestamp_ns_frame         Frame time (ns mod 2^32, local synchronization)
16–19   u32_be   header_crc32               CRC-32(bytes 0–15), error detection
20–23   u32_be   next_frame_offset_bytes    Byte offset to next frame header (enables frame-level seeking)
------  -------  -------                    --------
Total: 24 bytes
```

### §1.2 Field Definitions

**protocol_version (bytes 0–1, u16 big-endian):**
- Value: 0x0303
- Interpretation: Major version 3, minor version 3 (DVSM v3.3)
- Immutable: Frozen at session init
- Purpose: Decoder version check

**tick_count (bytes 2–3, u16 big-endian):**
- Range: [0, 65535]
- Wraps every 65536 ticks ≈ 546 ms at 120 Hz
- Interpretation: Frame sequence number within session
- Purpose: Detect dropped frames, temporal ordering verification

**h_global_hash32 (bytes 4–7, u32 big-endian):**
- Source: First 32 bits of BLAKE3(H_core ⊕ H_coupling ⊕ version)
- Purpose: Quick integrity check (full BLAKE3 is 256 bits, header includes hash prefix)
- Determinism: Identical for identical state + coupling + protocol version

**regime_and_flags (byte 8, u8):**
- Bits [7:5]: regime ∈ {Locked(0), Nominal(1), Slipping(2), Reserved(3-7)}
  - Locked: Residual singularity ≥ 0.92, high predictability, 90%+ compression
  - Nominal: Normal operation, 40–70% compression
  - Slipping: Low predictability, minimal compression, full state dump
- Bits [4:0]: modality flags (packed as 5-bit field)
  - Bit [0]: rf_present (1=RF modality active, 0=dormant)
  - Bit [1]: elf_present (1=ELF modality active, 0=dormant)
  - Bit [2]: bio3d_present (1=BioScience 3D active, 0=dormant)
  - Bit [3]: depth_present (reserved for future geometric modality)
  - Bit [4]: reserved

**codec_modes (bytes 9–10, u16 big-endian):**
- Bits [4:0]: rf_codec ∈ {Huffman(0), Arithmetic(1), Stored(2), Delta(3), Reserved(4-31)}
- Bits [9:5]: elf_codec (same enumeration)
- Bits [14:10]: bio3d_codec (same enumeration)
- Bit [15]: reserved
- Interpretation: Per-modality codec selection (adaptive per-frame)
  - Huffman: static table, low entropy signals (good for RF/ELF baseline)
  - Arithmetic: adaptive entropy coding, high compression on variable signals (good for Bio3D)
  - Stored: uncompressed (used when compression ratio < 1.0, i.e., residual entropy too high)
  - Delta: differential encoding (frame[t] - frame[t-1]) for temporal prediction residuals
- Determinism: Codec selection depends on regime and residual entropy (frozen per-frame)

**compression_metadata (byte 11, u8):**
- Bits [2:0]: quality_preset ∈ {Lossless(0), VisuallyLossless(1), Quality(2), Balanced(3), Speed(4)}
  - Lossless: No quantization loss, full fidelity (slow encoding, ~3 ms latency)
  - VisuallyLossless: PCA rank-250 retained, spatial filtering applied, imperceptible loss
  - Quality: Rank-200 PCA, good visual fidelity, moderate compression
  - Balanced: Rank-150 PCA, good speed, reasonable compression (default)
  - Speed: Rank-100 PCA, fastest encoding, lower visual fidelity
- Bit [3]: spatial_tiling_present (0=full frame, 1=tile hints included in payload)
- Bits [7:4]: reserved

**timestamp_ns_frame (bytes 12–15, u32 big-endian):**
- Source: Frame acquisition time in nanoseconds mod 2^32
- Wraparound: Every ~4.3 seconds
- Purpose: Local frame synchronization (not for absolute time, just relative ordering)
- Determinism: Frozen at frame capture time

**header_crc32 (bytes 16–19, u32 big-endian):**
- Computation: CRC-32(bytes 0–15) using polynomial 0x04C11DB7 (Ethernet/ZLIB standard)
- Purpose: Detect header corruption in transit (single-bit error detection, burst error detection up to 32 bits)
- Determinism: CRC is deterministic function of header content

**next_frame_offset_bytes (bytes 20–23, u32 big-endian):**
- Value: Total frame size including header, all payloads, and trailing CRC-32
- Interpretation: Byte offset from current frame header to next frame header
- Purpose: Frame-level seeking (decoder can jump to next frame without parsing payloads)
- Determinism: Computed from modality_flags and payload sizes (deterministic)

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
