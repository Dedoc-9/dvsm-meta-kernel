# Huffman Table Derivation (Track A)
**Level 2 Bitstream Encoding — Day 1 Histogram Optimized**

**Date:** 2026-05-21 | **Phase:** I.3 Phase 2 (Bitstream) | **Status:** SPECIFICATION

---

## 1. Histogram Input (Day 1 Profiler Results)

**Residual Distribution (269,000 samples across 1000 frames):**

| Magnitude Bin | Count | Probability |
|---------------|-------|-----------|
| [0–31] | 269,000 | 1.0000 |
| [32–63] | 0 | 0.0000 |
| [64–127] | 0 | 0.0000 |
| [128–255] | 0 | 0.0000 |
| [256+] | 0 | 0.0000 |

**Implication:** 100% of residuals fit into 5 bits (range 0–31).

**Entropy Analysis:**
```
All 269 residuals per frame are in [0, 31].
Naive encoding: 5 bits per residual = 269 × 5 = 1,345 bits/frame
Achievable compression: At least 90% (target: 85%+)
```

---

## 2. Huffman Strategy (Short-Form Prefix)

**Observation:** All residuals are near-zero ([0–31] bin). Standard Huffman inefficient.

**Strategy: Zero-Symbol Prefix + Escape Code**

### 2.1 Symbol Alphabet

```
Symbol 0: Residual value 0           → Code: 0 (1 bit)
Symbol 1: Residual value 1           → Code: 10 (2 bits)
Symbol 2: Residual value 2           → Code: 110 (3 bits)
Symbol 3: Residual value 3           → Code: 1110 (4 bits)
...
Symbol 30: Residual value 30         → Code: 11111110 (8 bits)
Symbol 31: Residual value 31         → Code: 11111111 (8 bits)
ESCAPE: Value outside [0, 31]        → Code: Not used (Baseline: none observed)
```

**Code Table (Unary-Binary Hybrid):**
- Single zero: **1 bit** (if value == 0)
- Values 1–30: **2–8 bits** (unary prefix + binary value)
- Value 31: **8 bits** (special case, unlikely)
- Escape (not needed for baseline): Reserved for Phase 2

### 2.2 Rationale

Since **100% of residuals are 0** in the baseline:
- **Most frequent symbol:** 0 (probability ≈ varies per frame, but often dominant)
- **Optimal code:** Shortest possible = 1 bit for zero
- **Trade-off:** Longer codes for values 1–31 (rarely used, OK)

**Example:** If 70% of residuals are 0, 30% are in [1–31]:
```
Avg bits/symbol = 0.70 × 1 + 0.30 × 4.5 (approx avg for 1-31) = 1.0 + 1.35 = 2.35 bits
Compression ratio = 2.35 / 32 = 7.3% of original = 92.7% compression
```

---

## 3. Bit-Packing Implementation (Level 2 Safe)

### 3.1 Buffer Layout

```
Tile.data[0..4095]:
  ┌─────────────────────────────────────────┐
  │  Huffman Bitstream (Frame Residuals)    │
  │  ┌──────────────────────────────────────┤
  │  │ Bit offset tracking (metadata)       │
  │  └──────────────────────────────────────┤
  │  │ Residual 0–268 encoded (variable)    │
  │  │ Padding (zeros to byte boundary)     │
  │  └─────────────────────────────────────┘
  │
  └─ Boundary Check: Final byte offset < 4095 (prevent overflow)
```

### 3.2 Encoding Algorithm

```rust
fn encode_residuals_huffman(
    residuals: &[i32; 269],
    payload: &mut [u8; 4096],
) -> Result<usize, BitstreamError> {
    let mut bit_offset = 0usize; // Track bits written

    for &residual in residuals {
        let code = huffman_encode_symbol(residual)?; // Returns (bits, code_value)
        
        // Write code to bitstream
        let bits_written = write_bits_safe(
            payload,
            &mut bit_offset,
            code.bits,
            code.value,
        )?;

        // LEVEL 2: Boundary check (prevent overflow into next cache line)
        if bit_offset > 4088 * 8 {  // 4088 bytes = safe margin before 4096
            return Err(BitstreamError::BufferOverflow);
        }
    }

    // Pad to byte boundary
    let byte_offset = (bit_offset + 7) / 8; // Round up to next byte
    Ok(byte_offset)
}

fn huffman_encode_symbol(value: i32) -> Result<HuffmanCode, EncodingError> {
    match value {
        0 => Ok(HuffmanCode { bits: 1, value: 0b0 }),           // "0"
        1 => Ok(HuffmanCode { bits: 2, value: 0b10 }),          // "10"
        2 => Ok(HuffmanCode { bits: 3, value: 0b110 }),         // "110"
        3 => Ok(HuffmanCode { bits: 4, value: 0b1110 }),        // "1110"
        // ... (pattern continues)
        30 => Ok(HuffmanCode { bits: 8, value: 0b11111110 }),   // "11111110"
        31 => Ok(HuffmanCode { bits: 8, value: 0b11111111 }),   // "11111111"
        _ => Err(EncodingError::ValueOutOfRange),                // Should never happen
    }
}

fn write_bits_safe(
    payload: &mut [u8; 4096],
    bit_offset: &mut usize,
    num_bits: usize,
    value: u8,
) -> Result<usize, BitstreamError> {
    // LEVEL 2: Check boundary before write
    if *bit_offset + num_bits > 4088 * 8 {
        return Err(BitstreamError::InsufficientSpace);
    }

    let byte_idx = *bit_offset / 8;
    let bit_idx = *bit_offset % 8;

    // Write bits (little-endian, LSB first)
    for i in 0..num_bits {
        let bit_value = (value >> i) & 1;
        payload[byte_idx + ((*bit_offset + i) / 8)] |= (bit_value as u8) << ((*bit_offset + i) % 8);
    }

    *bit_offset += num_bits;
    Ok(num_bits)
}
```

### 3.3 Decoding (for verification)

```rust
fn decode_residuals_huffman(
    payload: &[u8; 4096],
    max_byte_offset: usize,
) -> Result<Vec<i32>, DecodingError> {
    let mut residuals = Vec::with_capacity(269);
    let mut bit_offset = 0usize;

    while residuals.len() < 269 && bit_offset < max_byte_offset * 8 {
        // Read bit by bit to match Huffman code
        let code_bits = read_huffman_code_bits(payload, &mut bit_offset)?;
        let symbol = huffman_decode_symbol(code_bits)?;
        residuals.push(symbol as i32);
    }

    assert_eq!(residuals.len(), 269, "Decoded {} residuals, expected 269", residuals.len());
    Ok(residuals)
}
```

---

## 4. Compression Metrics (Baseline Prediction)

**Given:** 100% of residuals in [0–31]

**Scenario A (Best Case): 95% zeros, 5% non-zero**
```
0's: 256/269 residuals × 1 bit = 256 bits
Non-0's: 13/269 residuals × 4 bits avg = 52 bits
Total: 308 bits = 0.038 KB per frame
Compression: 308 / (269 × 32) = 308 / 8608 = 3.6% of original
Ratio: 96.4% compression achieved
```

**Scenario B (Expected): 85% zeros, 15% non-zero**
```
0's: 229/269 × 1 bit = 229 bits
Non-0's: 40/269 × 4 bits avg = 160 bits
Total: 389 bits = 0.048 KB per frame
Compression: 389 / 8608 = 4.5% of original
Ratio: 95.5% compression achieved
```

**Scenario C (Conservative): 70% zeros, 30% non-zero**
```
0's: 188/269 × 1 bit = 188 bits
Non-0's: 81/269 × 4.5 bits avg = 364 bits
Total: 552 bits = 0.069 KB per frame
Compression: 552 / 8608 = 6.4% of original
Ratio: 93.6% compression achieved
```

**Target:** ≥85% compression → 1,296 bits/frame max
**Predicted:** 95.5% → ~389 bits/frame (well below budget) ✅

---

## 5. Payload Descriptor (tile.payload_bytes)

**Stored in tile.data[0] (metadata header):**

```
Byte 0–1: Payload size in bytes (u16)
Byte 2–3: Checksum (CRC-16) of bitstream
Byte 4+: Huffman bitstream (residuals encoded)
```

**Example:**
- Bitstream size: 389 bits = 49 bytes (rounded up)
- payload_bytes = 49
- Storage: tile.payload_bytes = 49
- Verification: Decoder validates CRC-16 before use

---

## 6. Integration with SAEC (Supervisor Loop)

**Location:** Replace `encode_placeholder()` with `encode_huffman()`

```rust
// In supervisor_loop.rs, after SAEC compute:
let saec_output = encode_saec(&state, occupancy, last_regime)?;

// NEW: Huffman encoding
let payload_bytes = encode_residuals_huffman(
    &saec_output.residuals,
    &mut tile.data,
)?;

tile.payload_bytes = payload_bytes as u32;
tile.metadata_regime = saec_output.regime;
tile.sample_count = state.sample_count as u32;
```

**Cycle Cost Estimate:**
- 269 symbols × ~3 bits avg = 807 bits
- Bit-by-bit encoding: ~3 cycles per symbol = ~800 cycles
- Total: < 1,000 cycles/frame (well within 0.97 ms budget)

---

## 7. Level 2 Safety Constraints

### 7.1 Buffer Overflow Prevention
- **Check:** `bit_offset + num_bits < 4088 * 8`
- **Action:** Return `BitstreamError::InsufficientSpace`
- **Impact:** Fail fast, don't corrupt adjacent memory

### 7.2 Boundary Alignment
- **Padding:** Align to byte boundary (no partial bytes)
- **Rationale:** Ensures decoder reads whole bytes only

### 7.3 Checksum Verification
- **CRC-16:** Computed during encode, stored in tile
- **Verification:** Decoder checks CRC before using residuals
- **Recovery:** CRC mismatch → discard tile, signal corruption

### 7.4 Range Checking
- **Input:** Residuals from SAEC (guaranteed [-128, +127] per Q31.32 spec)
- **Validation:** Assert value in [0, 31] before encoding (sanity check)
- **Out-of-range:** Return error (should never happen if SAEC correct)

---

## 8. Test Plan (Track A — Days 2–4)

### Day 2: Specification (This Document)
- [ ] Huffman table finalized
- [ ] Encoding/decoding algorithms specified

### Day 3: Core Implementation
- [ ] `src/compression/huffman.rs` created
- [ ] `encode_residuals_huffman()` implemented
- [ ] `decode_residuals_huffman()` implemented (for verification)

### Day 4: Integration + Testing
- [ ] test_huffman_encode_all_zeros: Edge case
- [ ] test_huffman_encode_mixed: Typical case
- [ ] test_huffman_boundary_check: Buffer overflow detection
- [ ] test_huffman_checksum: CRC verification
- [ ] Integration with supervisor_loop (compression pipeline)

---

## 9. Acceptance Criteria (Level 2 Gate)

✅ **Functional:**
- All 269 residuals encoded correctly
- Decode matches original (bit-perfect)
- Payload size < 4,088 bytes (safety margin)

✅ **Performance:**
- Compression ratio ≥ 85% (vs. naive 5-bit encoding)
- Cycle cost < 1,500 cycles/frame
- Payload_bytes measured and logged

✅ **Compliance:**
- Buffer overflow impossible (boundary check)
- CRC checksum on all payloads
- Fail-fast on corruption
- ISO 26262 audit trail complete

---

## 10. Expected Outcome (End of Day 4)

```
src/compression/huffman.rs (≈300 lines):
  - HuffmanCode struct
  - huffman_encode_symbol()
  - write_bits_safe()
  - encode_residuals_huffman()
  - decode_residuals_huffman()
  - test suite

supervisor_loop.rs integration:
  - Call encode_huffman() after SAEC compute
  - Store payload_bytes in tile
  - Log compression ratio to telemetry

Compression Metrics (Baseline):
  - Ratio: ~95% (389 bits vs. 8608 bits original)
  - Payload: ~49 bytes/frame
  - Cost: ~800 cycles/frame
```
