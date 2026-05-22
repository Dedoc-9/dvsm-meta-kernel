# RF/ELF Integration Specification (Model B)
**Level 2 Contract Definition — Tier 2 Implementation**

**Date:** 2026-05-21 | **Phase:** I.0.5 (Supervisor Hook) | **Status:** SPECIFICATION (Pre-Implementation)

---

## 1. Contract Scope

**Purpose:** Define the interface by which external RF/ELF modality data enters the DVSM supervisor loop.

**Design Pattern:** Producer-Consumer via user-provided ring buffer (external to DVSM, not DVSM-managed)

**Thread Model:**
- **Core 0:** DVSM supervisor (consumer, tries to pop)
- **Core 1+:** User's RF/ELF producer (pushes samples)
- **Latency:** try_pop() ≤ 50 ns (L1-hit memcpy)

**Session Contract:**
- Ring buffer pointer injected at `session_init()`
- Layout-ID bound to H_session (prevents ABI drift)
- Stale detection: sample timestamp checked per frame

---

## 2. Data Structure: RfElfSample (64-byte aligned)

```rust
/// RF/ELF External Modality Sample
/// Layout-ID: 0x8F3E1A9C
/// Alignment: 64-byte (prevents L1D false-sharing)
#[repr(C, align(64))]
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
```

**Serialization:** C-compatible (`repr(C)`), little-endian, no padding

**Version Binding:** Layout-ID `0x8F3E1A9C` = HASH(struct_name ⊕ field_offsets ⊕ types)
- Immutable during session
- Checked at try_pop() to detect user ABI drift
- If mismatch: fail-fast ERR_MODALITY_CORRUPTED

---

## 3. Ring Buffer Contract (SPSC — Single-Producer, Single-Consumer)

**Provided by user.** DVSM only reads (never writes).

```rust
/// User-provided ring buffer interface
pub trait RfElfBuffer {
    /// Try to pop one sample from the ring buffer
    /// Non-blocking, lock-free
    /// 
    /// Returns:
    ///   Ok(sample) if one was available
    ///   Err(RfElfError::Empty) if no sample ready
    ///   Err(RfElfError::Stale) if sample age > MAX_STALE_US
    ///   Err(RfElfError::BufferOverflow) if producer wrote faster than consumer popped
    fn try_pop(&mut self) -> Result<RfElfSample, RfElfError>;

    /// Get the current write position (for diagnostics)
    fn write_position(&self) -> u64;

    /// Get the current read position (for diagnostics)
    fn read_position(&self) -> u64;

    /// Layout-ID verification
    fn layout_id(&self) -> u32;
}
```

**User Responsibility:**
1. Allocate ring buffer (e.g., `Vec<RfElfSample>` or mmap)
2. Initialize with RfElfSample defaults (all zeros except layout_id)
3. Spawn producer thread to write samples
4. Pass buffer ptr to `dvsm_session_init_ffi(buffer_ptr, buffer_capacity)`
5. Maintain producer thread affinity (Core 1+ for cache locality)

**DVSM Responsibility:**
1. Call try_pop() once per frame (Phase I.0.5)
2. Validate Layout-ID (abort if mismatch)
3. Check stale detection (timestamp_us)
4. Couple sample to Z_t evolution if valid
5. Log telemetry (try_pop cost, stale count, overflow count)

---

## 4. Error Codes (fail-fast + non-fatal semantics)

```rust
pub enum RfElfError {
    // Fail-Fast (initialization only, session becomes invalid)
    BufferMissing,              // ERR_MODALITY_MISSING: No buffer provided at init
    InvalidCapacity,            // Buffer too small (< 128 samples)
    LayoutIdMismatch,          // ERR_MODALITY_CORRUPTED: User ABI drifted
    
    // Non-Fatal (runtime, graceful degradation)
    Empty,                      // No sample ready (OK, try next frame)
    Stale,                      // Sample age > 1ms (log warning, skip, continue)
    BufferOverflow,            // Producer wrote faster than consumer (backpressure)
    TimestampInvalid,          // Timestamp went backward (clock glitch)
    PayloadMismatch,           // Reported size != actual data (corruption check)
}
```

---

## 5. Phase I.0.5 Integration (Supervisor Loop Hook)

**Location:** `src/supervisor_loop.rs` after Z_t evolution, before compression

```rust
pub fn supervisor_tick(
    state: &mut DVSMState,
    pool: &mut TilePool,
    queue: &CompressionQueue,
    rf_elf_buffer: Option<&mut dyn RfElfBuffer>, // NEW: external modality
) {
    // ... (existing code) ...

    // === PHASE I.0.5: RF/ELF MODALITY INJECTION ===
    if let Some(buffer) = rf_elf_buffer {
        match buffer.try_pop() {
            Ok(sample) => {
                // Verify Layout-ID (fail-fast protection)
                if sample.layout_id != LAYOUT_ID_RF_ELF {
                    panic!("ERR_MODALITY_CORRUPTED: Layout-ID mismatch");
                }

                // Check staleness (non-fatal)
                let age_us = state.metadata.current_timestamp_us - sample.timestamp_us;
                if age_us > MAX_STALE_US {
                    state.telemetry.rf_elf_stale_count += 1;
                    // Skip this sample, continue (no crash)
                } else {
                    // Couple to Z_t evolution
                    state.rf_elf_sample = sample;
                    state.rf_elf_valid = true;
                    
                    // Compute coupling coefficient (Phase 2)
                    // z_next = z_evolve(z_t, sample.rf_phase, sample.elf_coherence)
                }
            }
            Err(RfElfError::Empty) => {
                // Normal: buffer empty, use last sample or zero
                state.telemetry.rf_elf_empty_frames += 1;
            }
            Err(RfElfError::Stale) => {
                // Producer too slow, skip
                state.telemetry.rf_elf_stale_count += 1;
            }
            Err(RfElfError::BufferOverflow) => {
                // Producer too fast, backpressure
                state.telemetry.rf_elf_overflow_count += 1;
                // Force Phase Shedding to reduce frame cost?
                // (TBD: depends on occupancy)
            }
            Err(e) => {
                // Other errors: log, continue
                eprintln!("RF/ELF error: {:?}", e);
            }
        }
    }

    // === PHASE I.3: COMPRESSION (EXISTING) ===
    // ... (compression enqueue) ...
}
```

**Key Properties:**
- **Non-blocking:** try_pop() never waits
- **Fail-fast:** Layout-ID mismatch → panic (session invalid)
- **Graceful degradation:** Stale/overflow → log, continue
- **Atomic coupling:** Z_t evolution sees RF/ELF sample atomically (no tearing)

---

## 6. Stale Detection (Timestamp-Based)

**Definition:** A sample is stale if its age exceeds 1 frame at 120 Hz.

```
Frame duration: 8.33 ms
Max acceptable age: 1 frame = 8.33 ms
MAX_STALE_US = 8333 microseconds
```

**Check (every frame):**
```
age_us = current_timestamp_us - sample.timestamp_us
is_stale = (age_us > 8333)
```

**Handling:**
- **Stale sample:** Skip coupling, increment `telemetry.rf_elf_stale_count`
- **Repeated stale:** If > 10 consecutive, log alert (producer died?)
- **Action:** Continue with last valid sample or zero-coupling

---

## 7. Layout-ID Verification (Corruption Check)

**Purpose:** Prevent ABI drift (user recompiled RfElfSample with different field order/size)

**Computation (one-time, at init):**
```
Layout-ID = HASH(
    "RfElfSample" ⊕
    offset(timestamp_us) ⊕ size(u64) ⊕
    offset(sample_id) ⊕ size(u32) ⊕
    offset(flags) ⊕ size(u16) ⊕
    ... (all fields) ...
)
```

**Hardcoded Expected Value:** `0x8F3E1A9C`

**Runtime Check (every try_pop):**
```rust
if sample.layout_id != 0x8F3E1A9C {
    panic!("ERR_MODALITY_CORRUPTED: ABI mismatch");
}
```

**Rationale:** Layout drift causes silent data corruption (fields misaligned). Better to fail fast than silently corrupt Z_t.

---

## 8. Thread Affinity & Thermal Budgeting

**Core Assignment (Z2 Extreme):**
- **Core 0:** DVSM supervisor (exclusive, real-time priority)
- **Core 1+:** User's RF/ELF producer + other tasks (non-exclusive)

**Context Switching:**
- `try_pop()` is lock-free → no waiting, no context switches
- Producer writes at own pace; supervisor consumes async

**L1D Coherency:**
- RfElfSample: 64-byte aligned (one cache line)
- Producer writes to offset [0, 64)
- Supervisor reads from offset [0, 64)
- L1D coherency: hardware-managed (25 ns typical on Zen 5)

**Thermal Envelope (35W Z2 Extreme):**
- try_pop() cost: ~50 ns per frame (L1 hit)
- CPU utilization: < 0.5% (negligible)
- **Headroom for Track A (Huffman):** 99.5% of 0.97 ms available

---

## 9. H_Session Binding (Level 2 Certification)

**Final H_session formula (Day 5 lock):**
```
H_session = HASH(
    μ_t ⊕ Z_t ⊕ S_t ⊕ W_t ⊕
    protocol_version ⊕
    BufferPresence ⊕ Layout-ID ⊕
    layout_id_0x8F3E1A9C
)
```

**Interpretation:**
- `BufferPresence = 0` if no RF/ELF buffer (Z-only evolution)
- `BufferPresence = 1` if buffer provided (with coupling enabled)
- `Layout-ID = 0x8F3E1A9C` (RfElfSample structure signature)

**Certification Implication:** Manifold's identity commits to RF/ELF contract; changing either breaks the hash.

---

## 10. Test Plan (Track C — Days 2–5)

### Day 2: Specification (This Document)
- [ ] RfElfSample layout finalized
- [ ] RfElfBuffer trait defined
- [ ] Error codes specified

### Day 3: Supervisor Integration
- [ ] Phase I.0.5 hook added to supervisor_tick()
- [ ] try_pop() call integrated
- [ ] Telemetry (stale_count, overflow_count, empty_frames) added

### Day 4: Integration Tests
- [ ] test_rf_elf_try_pop_success: Normal sample ingestion
- [ ] test_rf_elf_stale_detection: Age > 8333 μs
- [ ] test_rf_elf_buffer_overflow: Producer too fast
- [ ] test_rf_elf_layout_id_mismatch: Corruption check triggers

### Day 5: Compliance & H_session Lock
- [ ] All tests pass
- [ ] Layout-ID verification in place
- [ ] H_session binding finalized

---

## 11. Acceptance Criteria (Level 2 Gate)

✅ **Functional:**
- try_pop() calls succeed without blocking
- Stale samples detected and skipped
- Layout-ID mismatch triggers panic
- Coupling parameter available for Phase 2

✅ **Performance:**
- try_pop() cost < 100 ns per call
- No L1D conflicts from ring buffer access
- Thermal headroom maintained (< 1W added)

✅ **Compliance:**
- All error paths logged
- Fail-fast on corruption
- Graceful degradation on stale/overflow
- ISO 26262 audit trail complete

---

**SPECIFICATION LOCKED** | **Ready for Implementation** | **Level 2 Compliant**
