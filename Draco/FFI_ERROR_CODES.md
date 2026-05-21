# FFI Error Codes: Operational Error Semantics
**Author:** Daniel J. Dillberg | **Date:** 2026-05-21 | **Status:** Error Contract Specification

---

## Overview

This document defines the complete error code enumeration for all DVSM operations across C, Rust, and Swift FFI layers. Error codes are stateless numeric identifiers that distinguish between fail-fast initialization errors (fatal) and graceful runtime errors (non-fatal, logged).

**Principle:** Errors are *observed state*, not exceptions. The supervisor loop never blocks on error; it records telemetry and continues.

---

## §1 Error Code Enumeration

### §1.1 Numeric Layout (12 Error Codes)

```c
// C enumeration (stable across all platforms, ABI-safe)
typedef enum {
    // ═══════════════════════════════════════════════════════
    // SUCCESS CODES (0x0000–0x00FF)
    // ═══════════════════════════════════════════════════════
    DVSM_OK = 0x0000,                      // All checks passed, session initialized
    DVSM_OK_MODALITY_DISABLED = 0x0001,    // RF/ELF not enabled (benign)
    DVSM_OK_MODALITY_STALE_ACCEPTED = 0x0002, // Stale data within tolerance
    
    // ═══════════════════════════════════════════════════════
    // INITIALIZATION ERRORS (0x0400–0x04FF, FAIL-FAST)
    // ═══════════════════════════════════════════════════════
    DVSM_ERR_MODALITY_MISSING = 0x0401,    // enable_rf_elf=true but buffer_ptr=null
    DVSM_ERR_MODALITY_INIT_FAILED = 0x0402, // User's buffer initialization failed
    DVSM_ERR_INVALID_BUFFER_SIZE = 0x0403,  // buffer capacity ≠ 256 samples
    DVSM_ERR_INVALID_CONFIG = 0x0404,       // Config struct validation failed
    
    // ═══════════════════════════════════════════════════════
    // RUNTIME ERRORS (0x0500–0x05FF, NON-FATAL, LOGGED)
    // ═══════════════════════════════════════════════════════
    DVSM_ERR_MODALITY_STALE = 0x0501,       // No sample in threshold; data stale
    DVSM_ERR_MODALITY_CORRUPTED = 0x0502,   // Integrity hash mismatch (torn read?)
    DVSM_ERR_MODALITY_OVERFLOW = 0x0503,    // Ring buffer full; sample dropped
    
    // ═══════════════════════════════════════════════════════
    // PROTOCOL/SESSION ERRORS (0x0600–0x06FF)
    // ═══════════════════════════════════════════════════════
    DVSM_ERR_SESSION_HASH_MISMATCH = 0x0601, // H_session ≠ expected (config changed?)
    DVSM_ERR_PROTOCOL_VERSION_UNSUPPORTED = 0x0602, // version < 0x0302, RF/ELF not supported
} dvsm_error_code_t;
```

**Encoding:**
- `0x0000–0x00FF`: Success codes
- `0x0400–0x04FF`: Initialization errors (FATAL, session_init returns error)
- `0x0500–0x05FF`: Runtime errors (NON-FATAL, logged in session telemetry)
- `0x0600–0x06FF`: Protocol/session errors (validation-layer)

---

## §2 Layer 1: Initialization Errors (Fail-Fast)

These errors occur during `dvsm_session_init()`. If any gate fails, the function returns error and **does NOT create a session**. The caller must handle the error.

### §2.1 ERR_MODALITY_MISSING (0x0401)

**Condition:**
```
config.enable_rf_elf_coupling == true
AND
config.rf_elf_buffer == null
```

**Semantics:** User enabled RF/ELF coupling but did not provide a ring buffer. Session initialization cannot proceed.

**Caller Responsibility:**
1. Check config.enable_rf_elf_coupling
2. If true, allocate RfElfRingBuffer and spawn producer thread
3. Pass buffer pointer to SessionConfig
4. Retry session_init()

**Example (C):**
```c
dvsm_session_t *session = nullptr;
dvsm_error_code_t result = dvsm_session_init_ffi(&config, &session);

if (result == DVSM_ERR_MODALITY_MISSING) {
    fprintf(stderr, "RF/ELF enabled but no buffer provided.\n");
    fprintf(stderr, "Allocate RfElfRingBuffer and pass to config.rf_elf_buffer.\n");
    return -1;  // Fatal
}
```

---

### §2.2 ERR_MODALITY_INIT_FAILED (0x0402)

**Condition:**
```
config.rf_elf_buffer != null
AND
buffer validation failed (e.g., capacity check, alignment check)
```

**Semantics:** User provided a buffer, but it does not meet the contract (e.g., wrong capacity, not cache-line aligned).

**Caller Responsibility:**
1. Ensure buffer is exactly 256 samples
2. Ensure buffer is aligned to 64-byte boundary
3. Ensure buffer is zero-initialized at startup

---

### §2.3 ERR_INVALID_BUFFER_SIZE (0x0403)

**Condition:**
```
buffer.capacity() != 256
```

**Semantics:** Ring buffer must be exactly 256 samples (power-of-2, for wrap-around arithmetic).

**Caller Responsibility:**
```c
// Correct allocation
RfElfRingBuffer buffer;
buffer.capacity = 256;  // Fixed
buffer.sample_size = 64; // Fixed (must match struct)
```

---

### §2.4 ERR_INVALID_CONFIG (0x0404)

**Condition:**
```
Any SessionConfig field fails validation:
  - frame_rate_hz ∉ {60, 120, 240}
  - sync_tier ∉ {1, 2, 3}
  - q_mode ∉ {Q31, Q16, Q64.64}
  - coefficients out of representable range
  - protocol_version < 0x0301
```

**Semantics:** Config struct contains invalid values. Session cannot be created.

---

## §3 Layer 2: Runtime Errors (Non-Fatal, Logged)

These errors occur during supervisor tick (Phase I.0.5 RF/ELF poll). If a runtime error occurs, the supervisor **continues without blocking**. The error is recorded in session telemetry for diagnostics.

### §3.1 ERR_MODALITY_STALE (0x0501)

**Condition:**
```
rf_elf_ring_buffer.last_poll_ns + rf_elf_stale_threshold_ms < now_ns
```

**Semantics:** Ring buffer has not been updated in the staleness threshold (default 50ms). Producer thread may be hung or blocked.

**Supervisor Behavior:**
```rust
if buffer.is_stale(threshold_ns) {
    state.modality_error = ERR_MODALITY_STALE;
    state.modality_error_count += 1;
    // Skip RF/ELF coupling, continue to next phase
    // NO FRAME DROP
}
```

**Telemetry:**
- `session.last_error = ERR_MODALITY_STALE`
- `session.modality_stale_count += 1`
- User can monitor via dashboard: "RF/ELF producer thread hung?"

---

### §3.2 ERR_MODALITY_CORRUPTED (0x0502)

**Condition:**
```
validate_rf_elf_sample(&sample) == false
// i.e., integrity hash mismatch: computed_hash ≠ sample.hash_sample
```

**Semantics:** Ring buffer sample failed integrity check. Possible torn read across cache lines (should be rare/impossible in SPSC same-process, but caught for paranoia).

**Supervisor Behavior:**
```rust
match buffer.try_pop() {
    Some(sample) => {
        if !validate_rf_elf_sample(&sample) {
            state.modality_error = ERR_MODALITY_CORRUPTED;
            state.modality_error_count += 1;
            // Skip coupling, continue
        } else {
            apply_rf_elf_coupling(&mut state, &sample);
        }
    },
    None => { /* empty, handled separately */ }
}
```

**Root Cause Analysis:**
- If this error persists: producer thread writing beyond 64-byte boundary
- If sporadic: likely L1/L2 cache interaction (pre-generate `FFI_DEBUGGING_GUIDE.md` section on this)

---

### §3.3 ERR_MODALITY_OVERFLOW (0x0503)

**Condition:**
```
rf_elf_ring_buffer.try_push() returns Err
// i.e., (head + 1) % 256 == tail (buffer full)
```

**Semantics:** Ring buffer is full. Producer attempted to push a sample, but supervisor hasn't consumed fast enough. Sample is dropped (oldest data lost, newest preserved via FIFO wrap).

**Supervisor Behavior:**
```rust
match buffer.try_pop() {
    Err(overflow) => {
        state.modality_error = ERR_MODALITY_OVERFLOW;
        state.modality_overflow_count += 1;
        // Continue: data loss is logged, not fatal
    },
    Ok(sample) => { /* normal path */ }
}
```

**Telemetry:**
- `session.modality_overflow_count` tracks total drops
- If overflow_count grows → producer thread is faster than supervisor can consume
- Consider: increase buffer size (but 256 is chosen for L2 cache residency)

---

## §4 Layer 3: Protocol/Session Errors

These errors validate the session state against expected conditions.

### §4.1 ERR_SESSION_HASH_MISMATCH (0x0601)

**Condition:**
```
computed_H_session ≠ expected_H_session
```

**Semantics:** Session hash binding is broken. This indicates either:
1. Config was modified after init (violates immutability)
2. Peer is running different protocol version or config
3. Corruption (extremely unlikely)

**Caller Responsibility:**
- Verify config has not changed since init
- Verify peer is running same DVSM version
- If persistent: log incident, restart session

---

### §4.2 ERR_PROTOCOL_VERSION_UNSUPPORTED (0x0602)

**Condition:**
```
config.protocol_version < 0x0302  // v3.2 is first with RF/ELF
AND
config.enable_rf_elf_coupling == true
```

**Semantics:** User requested RF/ELF on a DVSM version that doesn't support it. Protocol version must be >= 0x0302 for RF/ELF, >= 0x0303 for BioScience 3D.

---

## §5 Error Semantics in FFI Context

### §5.1 Rust Layer (Result<T, ErrorCode>)

```rust
pub fn dvsm_session_init(config: &SessionConfig) -> Result<Session, dvsm_error_code_t> {
    // Validation gates
    if config.enable_rf_elf_coupling && config.rf_elf_buffer.is_none() {
        return Err(DVSM_ERR_MODALITY_MISSING);
    }
    
    // If Ok, session is fully initialized and ready
    Ok(Session { /* ... */ })
}
```

---

### §5.2 C Layer (Output Parameter + Return Code)

```c
dvsm_error_code_t dvsm_session_init_ffi(
    const dvsm_config_t *config,
    dvsm_session_t **out_session  // Populated only if return == DVSM_OK
) {
    // Validation
    if (config->enable_rf_elf_coupling && config->rf_elf_buffer == nullptr) {
        *out_session = nullptr;  // Signal failure
        return DVSM_ERR_MODALITY_MISSING;
    }
    
    // Allocate
    dvsm_session_t *session = malloc(sizeof(dvsm_session_t));
    session->h_session = compute_h_session(config);
    
    *out_session = session;
    return DVSM_OK;
}
```

**Caller Pattern:**
```c
dvsm_session_t *session = nullptr;
dvsm_error_code_t result = dvsm_session_init_ffi(&config, &session);

if (result != DVSM_OK) {
    // Handle error (init-layer errors are fatal)
    print_error(result);
    return -1;
}

// Session is guaranteed valid here
assert(session != nullptr);
assert(session->h_session != 0);
```

---

### §5.3 Swift Layer (Result<Session, ErrorCode>)

```swift
class DVSMSession {
    enum Error: Int {
        case ok = 0x0000
        case modalityMissing = 0x0401
        case modalityStale = 0x0501
        // ...
    }
    
    static func initialize(config: DVSMConfig) throws -> DVSMSession {
        if config.enableRfElf && config.rfElfBuffer == nil {
            throw Error.modalityMissing
        }
        // ...
    }
}
```

---

## §6 Error Handling Best Practices

### Pattern 1: Fail-Fast at Init
```c
dvsm_error_code_t result = dvsm_session_init_ffi(&config, &session);
if (result != DVSM_OK) {
    // Fatal: cannot proceed
    log_fatal("Session init failed: 0x%04X\n", result);
    exit(1);
}
```

### Pattern 2: Graceful Runtime Degradation
```rust
// Supervisor tick continues even if RF/ELF error occurs
pub fn supervisor_tick(session: &mut Session, input: &InputFrame) {
    // ...
    
    // Phase I.0.5: RF/ELF poll (may set session.last_error)
    if let Err(e) = rf_elf_try_pop(session) {
        // Non-fatal: log and continue
        eprintln!("[RF/ELF] Error: 0x{:04X}, continuing", e as u32);
    }
    
    // Frame still completes on-time
}
```

### Pattern 3: Telemetry Dashboard
```c
// Expose error telemetry for monitoring
struct {
    dvsm_error_code_t last_error;
    uint64_t modality_stale_count;
    uint64_t modality_overflow_count;
    uint64_t modality_corrupted_count;
} session->telemetry;

// Dashboard query:
if (session->telemetry.modality_overflow_count > 10) {
    alert("RF/ELF producer too fast, samples dropped");
}
```

---

## §7 Validation Tests

### Test: Init Errors Are Caught
```rust
#[test]
fn test_init_err_modality_missing() {
    let config = SessionConfig {
        enable_rf_elf_coupling: true,
        rf_elf_buffer: None,  // ← Missing
        ..SessionConfig::default()
    };
    
    let result = dvsm_session_init(&config);
    assert_eq!(result, Err(DVSM_ERR_MODALITY_MISSING));
}
```

### Test: Runtime Errors Are Non-Fatal
```rust
#[test]
fn test_runtime_err_stale_does_not_stop_supervisor() {
    let mut session = create_valid_session();
    
    // Simulate stale buffer (don't update for 100ms)
    session.rf_elf_buffer.last_poll_ns = now_ns - 100_000_000;
    
    // Tick should still complete
    let result = dvsm_tick(&mut session, &input_frame);
    
    // No panic, no frame drop
    assert_eq!(session.last_error, Some(DVSM_ERR_MODALITY_STALE));
}
```

---

## §8 Summary

| Error | Layer | Severity | Action |
|-------|-------|----------|--------|
| MODALITY_MISSING | Init | FATAL | Allocate buffer, retry init |
| INVALID_BUFFER_SIZE | Init | FATAL | Fix buffer capacity to 256 |
| MODALITY_STALE | Runtime | NON-FATAL | Log, continue (no coupling) |
| MODALITY_CORRUPTED | Runtime | NON-FATAL | Log, skip sample, continue |
| MODALITY_OVERFLOW | Runtime | NON-FATAL | Log sample drop, continue |
| SESSION_HASH_MISMATCH | Session | FATAL | Verify config, restart |

**Invariant:** Supervisor loop never blocks on error. All runtime errors are observed, logged, and non-blocking.
