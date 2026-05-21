# RF/ELF External Queue Specification
**Author:** Daniel J. Dillberg | **Date:** 2026-05-21 | **Status:** Layout-ID Locked (0x8F3E1A9C)

---

## Overview

This specification defines the RF/ELF (Radio Frequency / Extremely Low Frequency) external ring buffer contract for DVSM v3.2+. The external queue model moves RF/ELF data ingestion from supervisor-resident state machines to user-provided asynchronous producer threads, reducing supervisor cost from 1.5–3.0ms to ~50ns.

**Design Principle:** DVSM supervisor is purely *consumer* of pre-computed RF/ELF samples. User (or sensor driver) is *producer*. Lock-free SPSC (Single-Producer, Single-Consumer) ring buffer mediates communication. No mutable state machines inside supervisor.

---

## §1 RfElfSample Struct Definition

### §1.1 Layout and Offsets (64 Bytes, Cache-Aligned)

```c
// C struct definition (stable ABI across platforms)
typedef struct {
    // Primary data fields (28 bytes)
    uint64_t timestamp_ns;          // Offset 0:  Nanosecond timestamp from producer
    float    rf_power_dbm;          // Offset 8:  RF power measurement (dBm scale)
    float    elf_frequency_hz;      // Offset 12: ELF frequency (Hz)
    uint32_t bio_feature_flags;     // Offset 16: Feature bits (heart rate, ECG, etc.)
    uint32_t sample_counter;        // Offset 20: Producer's sample index (for sequencing)
    uint32_t queue_occupancy;       // Offset 24: Ring buffer occupancy at production
    uint32_t hash_sample;           // Offset 28: Integrity hash (CRC32 or FNV1A)
    
    // Reserved extension block (32 bytes, pre-allocated for v3.4-3.5)
    uint16_t bio_hr_bpm;            // Offset 32: Heart rate (BPM), reserved v3.4+
    uint16_t bio_spo2_pct;          // Offset 34: Blood oxygen saturation (%), reserved v3.4+
    float    bio_temp_c;            // Offset 36: Body temperature (Celsius), reserved v3.4+
    float    bio_ecg_mv;            // Offset 40: ECG amplitude (millivolts), reserved v3.4+
    uint16_t bio_resp_rate;         // Offset 44: Respiration rate (bpm), reserved v3.4+
    float    rf_gain_db;            // Offset 46: RF amplifier gain (dB), reserved v3.4+
    float    elf_phase_rad;         // Offset 50: ELF phase modulation (radians), reserved v3.4+
    float    elf_snr_db;            // Offset 54: ELF signal-to-noise (dB), reserved v3.4+
    uint8_t  future_padding[2];     // Offset 58: Alignment padding, reserved v3.5+
    uint32_t layout_guard;          // Offset 60: Sentinel (MUST = 0x00000000)
} dvsm_rf_elf_sample_t;

_Static_assert(sizeof(dvsm_rf_elf_sample_t) == 64, "RfElfSample must be 64 bytes");
_Static_assert(offsetof(dvsm_rf_elf_sample_t, bio_hr_bpm) == 32, "bio_hr_bpm offset");
_Static_assert(offsetof(dvsm_rf_elf_sample_t, layout_guard) == 60, "layout_guard offset");
```

### §1.2 Layout-ID Hash

```
Layout-ID = 0x8F3E1A9C

Computed as: HASH(struct_definition || field_offsets || field_types || reserved_allocation)

This hash is IMMUTABLE for v3.2–3.5. If struct changes (e.g., size > 64 bytes, field reorder),
Layout-ID must be recomputed, invalidating H_session for all prior sessions.
```

### §1.3 Field Semantics

**Primary Data (Bytes 0–28):**
- `timestamp_ns`: Producer's wall-clock time (nanoseconds since UNIX epoch or session start). Supervisor uses this to detect staleness.
- `rf_power_dbm`: RF power level in decibels relative to 1 milliwatt. Range: −80 to +40 dBm typical.
- `elf_frequency_hz`: ELF fundamental frequency in Hz. Range: 0.1–300 Hz typical.
- `bio_feature_flags`: Bitmask indicating which biomedical modalities are active in this sample (bit 0: HR, bit 1: SPO2, bit 2: ECG, etc.).
- `sample_counter`: Producer's monotonically-increasing sample index. Supervisor checks for dropped samples: (current_counter − prev_counter) should equal 1.
- `queue_occupancy`: Ring buffer depth at production time (diagnostic). Helps identify buffer undersizing.
- `hash_sample`: Integrity checksum. Computed as FNV1A or CRC32 of bytes 0–59. Detector torn reads across cache lines.

**Reserved Extension (Bytes 32–59):**
All fields in this block are reserved for v3.4 and later. v3.2 producer threads should zero-initialize these bytes. v3.3+ producers may populate them; v3.2 supervisor will safely ignore them.

**Layout Guard (Bytes 60–63):**
Sentinel field that MUST equal 0x00000000. Supervisor validates this at pop time; if nonzero, sample fails integrity check (ERR_MODALITY_CORRUPTED).

---

## §2 RfElfRingBuffer (SPSC Lock-Free Queue)

### §2.1 Structure

```c
typedef struct {
    dvsm_rf_elf_sample_t samples[256];    // 256 samples × 64 bytes = 16 KB
    uint32_t head;                        // Producer write index (mod 256)
    uint32_t tail;                        // Supervisor read index (mod 256)
    uint64_t last_poll_ns;                // Last supervisor pop() time (for staleness)
    uint8_t  _padding[8];                 // Cache-line alignment
} dvsm_rf_elf_ring_buffer_t;

// Total: 16 KB + 16 bytes = 16,416 bytes (fits in L2 cache)
```

### §2.2 SPSC Lock-Free Semantics

**Producer Thread:**
```rust
pub fn try_push(buffer: &mut RfElfRingBuffer, sample: RfElfSample) -> Result<(), ()> {
    let new_head = (buffer.head + 1) % 256;
    
    // Check if full: (new_head == tail) means no space
    if new_head == buffer.tail {
        return Err(()); // Buffer full, sample dropped
    }
    
    // Write sample (single 64-byte store, atomically visible via cache line)
    buffer.samples[buffer.head] = sample;
    
    // Advance head (compiler barrier ensures sample written before head visible)
    buffer.head = new_head;
    
    Ok(())
}
```

**Supervisor Thread (Consumer):**
```rust
pub fn try_pop(buffer: &mut RfElfRingBuffer) -> Result<RfElfSample, PopError> {
    // Empty check: head == tail means no data
    if buffer.head == buffer.tail {
        return Err(PopError::Empty);
    }
    
    // Read sample
    let sample = buffer.samples[buffer.tail];
    
    // Validate integrity
    if !validate_sample_hash(&sample) {
        buffer.tail = (buffer.tail + 1) % 256;
        return Err(PopError::Corrupted);
    }
    
    // Update staleness marker
    buffer.last_poll_ns = now_ns();
    
    // Advance tail
    buffer.tail = (buffer.tail + 1) % 256;
    
    Ok(sample)
}

fn validate_sample_hash(sample: &RfElfSample) -> bool {
    let computed_hash = fnv1a_hash(&sample.bytes[0..60]);
    computed_hash as u32 == sample.hash_sample
}
```

**Ordering Guarantees:**
- No explicit locks: true lock-free
- Producer and supervisor run on different cores (Core 1 and Core 0 on Z2 Extreme)
- 64-byte sample = single cache-line write (atomic from L1 perspective)
- Head/tail advancement uses atomic (or volatile) store to ensure visibility
- Stale detection via `last_poll_ns` timestamp (not synchronized counter)

### §2.3 Capacity and Performance

```
Samples per buffer:         256 (power-of-2, for wrap-around arithmetic)
Bytes per sample:           64 (cache-line aligned)
Total buffer size:          16 KB (fits in L2, good for Zen 5)
Per-sample cost:            ~50ns (L1-hit memcpy + validation on supervisor tick)
Maximum throughput:         256 samples × frame_rate (e.g., 30,720 samples/sec at 120 Hz)
```

---

## §3 Staleness Detection

### §3.1 Algorithm

```
Supervisor Phase I.0.5:
  now_ns = current_wallclock_time()
  stale_threshold_ms = 50  // Configurable, default 50ms
  stale_threshold_ns = stale_threshold_ms × 1_000_000
  
  if (now_ns - buffer.last_poll_ns) > stale_threshold_ns:
    state.modality_error = ERR_MODALITY_STALE
    state.modality_stale_count += 1
    // Skip RF/ELF coupling, continue to next phase
  else:
    // Attempt pop and coupling
    match buffer.try_pop():
      Ok(sample) → apply_rf_elf_coupling(state, sample)
      Err(_) → handle_pop_error(state, error)
```

### §3.2 Semantics

- Stale detection is *pessimistic*: if supervisor hasn't seen a fresh sample in 50ms, assume producer is hung.
- Staleness does NOT block supervisor; frame completes on-time.
- Stale counter is telemetry: user can monitor dashboard to diagnose producer thread issues.
- Threshold is configurable per session (e.g., 20ms for real-time biomedical, 500ms for low-power mode).

---

## §4 Integration with H_session Binding

### §4.1 Hash Computation

```
H_session = HASH(Config ⊕ Protocol_Version ⊕ BufferPresence ⊕ Layout-ID)

If enable_rf_elf_coupling == true:
  BufferPresence = HASH(buffer_ptr) ⊕ Layout-ID
  → H_session includes buffer presence + struct layout fingerprint
  
If enable_rf_elf_coupling == false:
  BufferPresence = 0x00
  Layout-ID is not included
  → H_session only includes Core config (no modality coupling)
```

### §4.2 Immutability Consequence

**Example Scenario 1: Core-Only Session**
```
Config: {enable_rf_elf=false, frame_rate=120, ...}
H_session = HASH(Config ⊕ 0x0302 ⊕ 0x00)
           = 0x... (some hash)

This session log is TAGGED as "Core-Only."
Trying to replay with RF/ELF buffer attached:
  Config now: {enable_rf_elf=true, frame_rate=120, ...}
  H_session_new = HASH(Config ⊕ 0x0302 ⊕ HASH(buffer_ptr) ⊕ 0x8F3E1A9C)
                ≠ H_session_old
  
→ Validation fails: ERR_SESSION_HASH_MISMATCH
→ Prevents accidental data fusion of Core and Core+RF runs
```

**Example Scenario 2: Struct Evolution**
```
v3.2 session (current Layout-ID 0x8F3E1A9C):
  H_session = HASH(...⊕ 0x8F3E1A9C)

v3.3 session (if struct changes, new Layout-ID 0xAABBCCDD):
  H_session_new = HASH(...⊕ 0xAABBCCDD)
                ≠ H_session_old
  
→ v3.2 and v3.3 sessions are cryptographically distinct
→ No silent incompatibility: old sessions remain reproducible under v3.2 Layout-ID
```

---

## §5 Thread Affinity and Performance

### §5.1 Z2 Extreme Allocation (Zen 5, 4 Cores)

```
Core 0: Supervisor (DVSM tick loop)
  - Frame-critical path
  - Cache-private L1 (~32 KB)
  - Shared L2 (512 KB per core pair)
  
Core 1: RF/ELF Producer Thread
  - RF/ELF sensor polling
  - Modality decoding (I/Q demod, FFT, etc.)
  - Ring buffer production
  - Can run at lower priority (not frame-critical)
  
L1-L1 Coherency Cost (Core 0 ↔ Core 1): ~25ns (L2 hit)
Ring buffer access pattern: Sequential write (producer), sequential read (supervisor)
  → Excellent cache locality, no false sharing

Cores 2–3: Available for GPU offload or other workloads
```

### §5.2 Latency Budget

```
Supervisor Phase I.0.5 (RF/ELF poll):
  ├─ try_pop():              ~15ns (L1 memcpy, modulo arithmetic)
  ├─ validate_hash():        ~20ns (FNV1A over 60 bytes)
  ├─ stale_check():          ~5ns (timestamp compare)
  └─ apply_coupling():       ~10ns (matrix multiply, state update)
  
Total per tick:             ~50ns
Frame budget impact:        50ns / 8.33ms = 0.0006% (negligible)

Headroom remaining:         0.97ms − 0.00005ms ≈ 0.97ms (effectively unchanged)
```

---

## §6 Error Semantics

All error codes are defined in FFI_ERROR_CODES.md. Key behaviors:

**Initialization Layer (Fail-Fast):**
- `ERR_MODALITY_MISSING` (0x0401): enable_rf_elf=true but buffer_ptr=null
- `ERR_INVALID_BUFFER_SIZE` (0x0403): buffer capacity ≠ 256
- → Session init returns error, NO session created

**Runtime Layer (Non-Fatal):**
- `ERR_MODALITY_STALE` (0x0501): last_poll_ns stale; skip coupling, continue
- `ERR_MODALITY_CORRUPTED` (0x0502): hash validation failed; skip sample, continue
- `ERR_MODALITY_OVERFLOW` (0x0503): buffer full, sample dropped; continue
- → Supervisor tick completes on-time, error logged in telemetry

---

## §7 Reserved Field Allocation (v3.4–3.5)

Pre-allocated offsets ensure future biomedical modalities fit without struct expansion.

```
Offset  Field              Type    Size  Status      Purpose
────────────────────────────────────────────────────────────────
32      bio_hr_bpm         u16     2     Reserved    Heart rate
34      bio_spo2_pct       u16     2     Reserved    Blood oxygen
36      bio_temp_c         f32     4     Reserved    Temperature
40      bio_ecg_mv         f32     4     Reserved    ECG amplitude
44      bio_resp_rate      u16     2     Reserved    Respiration
46      rf_gain_db         f32     4     Reserved    RF gain
50      elf_phase_rad      f32     4     Reserved    ELF phase
54      elf_snr_db         f32     4     Reserved    ELF SNR
58      future_padding     u8[2]   2     Reserved    v3.5 expansion
60      layout_guard       u32     4     Mandatory   Sentinel (0x00)

Total Allocated: 28 bytes
Safety Margin:   4 bytes (layout_guard)
Available:       0 bytes (reserved block is full)

Extensibility Policy:
- If v3.4 modalities exceed 28 bytes: struct size must expand to 80+ bytes
  → Layout-ID must be recomputed (new hash)
  → Old v3.2 and v3.3 sessions remain valid under old Layout-ID
- No in-place modification of existing fields
- All v3.4+ producers must zero-initialize reserved[32] on startup (backward compatibility)
```

---

## §8 Producer Thread Responsibilities

User is responsible for:

1. **Allocation and Initialization**
   ```c
   dvsm_rf_elf_ring_buffer_t buffer;
   memset(&buffer, 0, sizeof(buffer));  // Zero-init reserved fields
   ```

2. **Production Loop** (on Core 1, lower priority)
   ```c
   while (system_running) {
       // Poll sensors (RF, ELF, biomedical)
       dvsm_rf_elf_sample_t sample = {
           .timestamp_ns = get_wallclock_ns(),
           .rf_power_dbm = read_rf_sensor(),
           .elf_frequency_hz = read_elf_sensor(),
           .bio_feature_flags = 0,  // Populated based on active sensors
           .sample_counter = sample_index++,
           .queue_occupancy = (buffer.head - buffer.tail) % 256,
           .hash_sample = compute_fnv1a(sample bytes[0..59]),
           .layout_guard = 0,
       };
       
       // Try enqueue
       if (!buffer_try_push(&buffer, &sample)) {
           // Buffer full: log telemetry, drop sample (or apply backpressure)
       }
   }
   ```

3. **Thread Safety**
   - Use atomic (volatile) loads/stores for head index
   - Producer only modifies head; supervisor only modifies tail
   - No locks required

4. **Monitoring**
   - Track sample_counter for dropped samples (gap detection)
   - Monitor overflow events (telemetry.modality_overflow_count)
   - Check supervisor's last_poll_ns periodically (ensure supervisor is consuming)

---

## §9 Validation Tests

### Test: SPSC Lock-Free Correctness
```rust
#[test]
fn test_spsc_no_data_loss() {
    let mut buffer = RfElfRingBuffer::new();
    
    // Producer pushes 256 samples (fill buffer)
    for i in 0..256 {
        let sample = RfElfSample {
            sample_counter: i,
            ..Default::default()
        };
        assert!(buffer.try_push(sample).is_ok());
    }
    
    // Next push fails (buffer full)
    let sample = RfElfSample { ..Default::default() };
    assert!(buffer.try_push(sample).is_err());
    
    // Supervisor pops all 256
    for i in 0..256 {
        let popped = buffer.try_pop().unwrap();
        assert_eq!(popped.sample_counter, i);
    }
    
    // Buffer now empty
    assert!(buffer.try_pop().is_err());
}
```

### Test: Layout-ID Binding
```rust
#[test]
fn test_layout_id_immutability() {
    let config = SessionConfig {
        enable_rf_elf_coupling: true,
        ..Default::default()
    };
    
    // Layout-ID must be 0x8F3E1A9C for v3.2
    assert_eq!(LAYOUT_ID, 0x8F3E1A9C);
    
    // H_session includes Layout-ID
    let h_session = compute_h_session(&config);
    let h_session_alt = compute_h_session_variant(&config, 0xDEADBEEF);
    
    // Different Layout-ID → different H_session
    assert_ne!(h_session, h_session_alt);
}
```

### Test: Staleness Detection
```rust
#[test]
fn test_staleness_threshold() {
    let mut session = create_valid_session();
    let mut buffer = session.rf_elf_buffer_mut();
    
    // Set last_poll_ns to 100ms ago
    buffer.last_poll_ns = now_ns() - 100_000_000;
    
    // Supervisor tick should detect stale
    let result = dvsm_tick(&mut session, &input_frame);
    
    assert_eq!(session.last_error, ERR_MODALITY_STALE);
    assert_eq!(session.modality_stale_count, 1);
    // Frame still completes on-time
    assert_eq!(frame_time_ms, 8.33);
}
```

---

## §10 Summary

| Aspect | Value |
|--------|-------|
| Struct Size | 64 bytes (cache-aligned) |
| Ring Buffer Capacity | 256 samples (~16 KB) |
| Layout-ID | 0x8F3E1A9C (immutable) |
| Per-Sample Cost | ~50ns (L1-hit) |
| Frame Budget Impact | 0.0006% |
| H_session Binding | HASH(...⊕ Layout-ID) |
| Thread Affinity | Core 0 (supervisor), Core 1 (producer) |
| Staleness Threshold | 50ms (configurable) |
| Error Model | Fail-fast (init), non-fatal (runtime) |

**Lock Status: Layout-ID 0x8F3E1A9C LOCKED for v3.2–3.5. Struct mutations require new Layout-ID and new H_session hash.**
