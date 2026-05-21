## Z2 Hardware Baseline: Extended Forensic Report

**Platform:** ASUS ROG Ally X (Snapdragon X Elite variant), Z2 Extreme  
**TDP:** 35W nominal (test at sustained load)  
**Target Headroom:** 0.97 ms (frame budget 8.33ms - supervisor 7.36ms)  
**Date:** 2026-05-21 (Phase I.3 Integration - Hardened)

---

## Performance Baseline: Supervisor + Placeholder Encoding

| Metric | Target | Baseline (TBD) | Status | Notes |
|--------|--------|--------|--------|-------|
| **Supervisor Loop (Total)** | < 7.36 ms | -- | -- | Z_t evolution + Phase I.3 hook |
| **TilePool Pop Latency** | < 100 ns | -- | -- | ABA CAS, non-blocking |
| **TilePool Push Latency** | < 100 ns | -- | -- | LIFO insertion |
| **Encode (Placeholder)** | < 1.0 μs | -- | -- | 64-byte memcpy only |
| **Phase I.3 Total Cost** | < 2.0 μs | -- | -- | pop + encode + push |
| **L1D Cache Conflicts** | < 0.1% per frame | -- | -- | Core 0 ↔ Core 1 coherency |
| **Headroom Remaining** | > 0.95 ms | -- | -- | 0.97ms - Phase I.3 cost |

---

## Hysteresis Validation

| Metric | Target | Observed | Status | Notes |
|--------|--------|----------|--------|-------|
| **Shedding Entry Threshold** | 200 tiles (78%) | -- | -- | Occupancy > 200 → Phase Shedding |
| **Shedding Exit Threshold** | 150 tiles (59%) | -- | -- | Occupancy < 150 → Resume |
| **Hysteresis Width** | 50 tiles | -- | -- | Prevents ping-pong at boundary |
| **Transition Frequency** | < 1 per 1000 frames | -- | -- | Should be rare under normal load |
| **Transition Cost** | < 100 ns | -- | -- | State flag flip, no allocations |

---

## Tile Pool Drain Rate

| Scenario | Target | Observed | Status |
|----------|--------|----------|--------|
| **Compression Thread Drain Rate** | > 120 tiles/sec | -- | -- | Must consume faster than supervisor produces |
| **Peak Occupancy** | < 200 tiles | -- | -- | Indicates system is healthy |
| **Sustained Occupancy** | < 64 tiles | -- | -- | Indicates no backpressure |
| **Recovery Time (from 200 → 150)** | < 10 frames | -- | -- | ~83ms at 120Hz |

---

## L1D Cache Conflict Detection

### Zen 5 Hardware Details

The Z2 Extreme uses a Zen 5 core architecture with:
- **L1D Cache:** 32 KB per core (8-way associative, 64-byte line size)
- **Core 0:** Supervisor (runs dvsm_step_full, Phase I.3 hook)
- **Core 1:** Compression (reads tiles, writes residuals back)
- **False Sharing Risk:** If supervisor writes to tile.metadata_regime and Core 1 reads tile.data[0:64], they fight for the same cache line

### Mitigation (Already Implemented)

```
#[repr(C, align(64))]
pub struct CompressionTile {
    pub data: [u8; 4096],       // 4KB payload
    pub metadata_regime: u8,     // Written by supervisor
    pub sample_count: u32,       // Written by supervisor
    // Aligned to next 64-byte boundary
}
```

**Expected Behavior:**
- Supervisor writes to bytes [4096:4100] of the tile
- Core 1 reads from bytes [0:4095]
- Different cache lines → no false-sharing stalls

| Conflict Type | Expected | Observed | Status |
|---------------|----------|----------|--------|
| **L1D Miss Rate (supervisor)** | < 0.1% | -- | -- | Baseline with no compression |
| **L1D Miss Rate (Core 1)** | < 1% | -- | -- | Reading unshared 4KB payload |
| **Cache-Line Ping-Pong** | 0 | -- | -- | No line bouncing between cores |
| **Coherency Stall Cycles** | < 50 per frame | -- | -- | Total wasted cycles |

### Measurement Method

Use `rdpmc` (Read Performance Monitoring Counter) to instrument:

```bash
# Before supervisor tick:
L1D_start = rdpmc(IA32_PERFCTR0)  # L1D_CACHE_MISSES

# During encode_placeholder:
... memcpy(tile.data, state.z, 64) ...

# After supervisor tick:
L1D_end = rdpmc(IA32_PERFCTR0)
conflicts = L1D_end - L1D_start
```

---

## Regime Distribution

| Regime | Trigger | Expected Frequency | Status |
|--------|---------|-------------------|--------|
| **Regime 3** | occ ≤ 64 | > 90% (normal load) | -- |
| **Regime 1** | 64 < occ ≤ 128 | < 5% (moderate pressure) | -- |
| **Regime 2** | 128 < occ ≤ 200 | < 3% (high pressure) | -- |
| **Regime 4** | occ > 200 | < 2% (phase shedding) | -- |

---

## Phase Shedding Events

| Metric | Target | Observed | Status |
|--------|--------|----------|--------|
| **Shed Count (per minute)** | < 10 | -- | -- | Should be rare under normal load |
| **Shed Burst (consecutive)** | < 5 | -- | -- | Avoid sustained shedding |
| **Recovery Pattern** | Oscillatory decay | -- | -- | Occupancy: 200 → 180 → 160 → ... → 140 |
| **Z_t Determinism During Shed** | ✅ Preserved | -- | -- | Z_t evolves regardless |
| **S_t Orthogonality After Shed** | ✅ Preserved | -- | -- | S_t only accumulates actual residuals |

---

## Frame Marking & Protocol Integrity

| Flag | Bit | Usage | Expected Impact |
|------|-----|-------|-----------------|
| **FLAG_UNCOMPRESSED** | 0x01 | Set during Phase Shedding | Marks frames skipped by supervisor |
| **FLAG_PHASE_SHEDDING** | 0x02 | Set during Phase Shedding | Indicates system is under extreme load |

**Downstream Protocol:**
- Decompressor must handle FLAG_UNCOMPRESSED frames (raw Z_t, no residual)
- Telemetry system logs FLAG_PHASE_SHEDDING events for diagnostics
- Protocol remains valid even with shed frames (no divergence, just slower compression)

---

## Forensic Checklist

- [ ] **L1D Miss Rate:** Monitoring `rdpmc` for Core 0 vs Core 1 contention
  - Success criteria: < 0.1% miss rate during placeholder encoding
  
- [ ] **Hysteresis Stability:** Verify no "ping-pong" behavior at 200/150 boundary
  - Success criteria: Regime transitions logged, no oscillation within 10 frames
  
- [ ] **Modality Integrity:** Verify S_t remains orthogonal even after shed events
  - Success criteria: || Z_t ⊥ S_t || < 1e-10 after Phase Shedding recovery
  
- [ ] **Frame Rate Stability:** Supervisor tick consistent at 8.33ms ± 0.1%
  - Success criteria: Cycle count variation < 1%
  
- [ ] **Placeholder Overhead:** encode_placeholder() costs < 1.0 μs
  - Success criteria: Pop (100ns) + encode (1μs) + push (100ns) = 1.2μs total
  
- [ ] **Tile Drain Rate:** Compression thread drains > 120 tiles/sec
  - Success criteria: Peak occupancy never exceeds 200 under sustained load

---

## Next Steps

1. **Compile & Link:** Ensure `src/supervisor_loop.rs`, `src/compression/placeholder.rs` build cleanly
2. **Run Baseline:** Execute placeholder encoding 10,000 times, measure cycle distribution
3. **Validate L1D:** Confirm no false-sharing stalls via `rdpmc` or hardware profiler
4. **Hysteresis Test:** Simulate load spike, verify Enter/Exit behavior at 200/150 thresholds
5. **Proceed to SAEC:** Once baseline is clean, integrate real residual computation

---

## Baseline Success Criteria

**Phase I.3 is production-ready when:**

1. ✅ Phase I.3 total cost < 2.0 μs (leaves 0.95ms headroom)
2. ✅ L1D miss rate < 0.1% (no false-sharing detected)
3. ✅ Hysteresis transitions < 1 per 1000 frames (no thrashing)
4. ✅ Frame rate stable ± 0.1% (deterministic timing)
5. ✅ S_t orthogonality preserved through shedding (Ghost Closure holds)

**Then:** Swap placeholder for SAEC encoder and re-measure full budget.
