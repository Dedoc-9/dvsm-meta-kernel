# FFI C API Reference Stubs: Function Signatures & Contracts
**Author:** Daniel J. Dillberg | **Date:** 2026-05-21 | **Status:** Interface Contract (Pre-Implementation)

---

## Overview

This document defines the C API surface for DVSM across all layers: initialization, supervisor tick, modality polling, and error handling. These are reference signatures (pseudocode) that lock the ABI contract before Tier 2 implementation.

**Principle:** All C signatures are stable and immutable once locked. Changes require version increment and backward-compatibility wrapper.

---

## §1 Type Definitions (ABI-Stable)

### §1.1 Opaque Session Handle

```c
// dvsm_session.h

typedef struct dvsm_session_opaque* dvsm_session_t;
// Opaque pointer: users cannot access internals (prevents ABI break if internals change)
```

**Lifetime:**
```
creation: dvsm_session_init_ffi() → dvsm_session_t
use:      dvsm_tick(), dvsm_get_error(), dvsm_get_state()
cleanup:  dvsm_session_destroy()
```

---

### §1.2 Configuration Struct (Portable, 256 bytes)

```c
// dvsm_config.h

#pragma pack(push, 1)
typedef struct {
    // === IMMUTABLE HEADER (64 bytes) ===
    uint32_t  config_version;               // 0x00030100
    uint32_t  protocol_version;             // 0x0302 (v3.2), 0x0303 (v3.3)
    uint64_t  timestamp_created_ns;         // Audit trail
    uint32_t  frame_rate_hz;                // Locked: {60, 120, 240}
    uint32_t  __pad1;
    
    // === RUNTIME TOGGLES (32 bytes) ===
    uint8_t   sync_tier;                    // {1, 2, 3}
    uint8_t   paranoid_mode;                // bool: soft-clip
    uint8_t   vrs_enabled;                  // bool: variable-rate shading
    uint8_t   neural_rose_enabled;          // bool: neural network
    uint8_t   q_mode;                       // {Q31=1, Q16=2, Q64.64=3}
    uint8_t   hard_clamp_enabled;           // bool: [-2.0, 2.0]
    uint8_t   __pad2[2];
    
    uint32_t  vrs_tile_size;                // 8 (fixed)
    uint32_t  ghostsnap_max_checkpoints;    // Typical: 1000
    
    // === RF/ELF MODALITY (32 bytes) ===
    uint8_t   enable_rf_elf_coupling;       // bool: enable RF/ELF integration
    uint8_t   __pad3[7];
    void*     rf_elf_buffer;                // Pointer to RfElfRingBuffer (user-allocated)
                                            // REQUIRED if enable_rf_elf_coupling=true
    uint32_t  rf_elf_stale_threshold_ms;    // Default 50 (milliseconds)
    uint32_t  __pad4;
    
    // === KILL SWITCHES (32 bytes) ===
    uint8_t   kill_phase_lock;              // bool: disable PLL
    uint8_t   kill_backreaction;            // bool: disable backreaction
    uint8_t   kill_ghostsnap;               // bool: disable ghost rebirth
    uint8_t   kill_vr_quaternion_renorm;    // bool: disable VR renormalization
    uint8_t   __pad5[4];
    
    // === COEFFICIENTS (Q31.32, 64 bytes) ===
    int64_t   alpha_base_q31_32;            // Backreaction gain
    int64_t   lambda_damping_q31_32;        // Dissipation
    int64_t   ema_beta_q31_32;              // EMA memory
    int64_t   backreaction_pulse_scale_q31_32;  // Scaling (default 4.0)
    int64_t   phase_lock_kappa_q31_32;      // Phase PLL coupling (0.25)
    int64_t   gudermannian_threshold_q31_32; // Singularity protection
    int64_t   __pad6[2];
    
    // === TOTAL: 256 bytes (4 cache lines) ===
} dvsm_config_t;

#pragma pack(pop)

_Static_assert(sizeof(dvsm_config_t) == 256, "dvsm_config_t must be 256 bytes");
```

---

### §1.3 RF/ELF Sample (64-byte, Cache-Line Aligned)

```c
// dvsm_rf_elf.h

#pragma pack(push, 1)
typedef struct __attribute__((aligned(64))) {
    uint64_t timestamp_ns;                  // [0:8]   Absolute poll timestamp
    float    rf_power_dbm;                  // [8:12]  RF signal strength
    float    elf_frequency_hz;              // [12:16] ELF band center
    uint32_t bio_feature_flags;             // [16:20] 8 channels × 4 bits
    uint32_t sample_counter;                // [20:24] Monotonic counter (detect drops)
    uint32_t queue_occupancy;               // [24:28] Ring buffer depth snapshot
    uint32_t hash_sample;                   // [28:32] Integrity hash (XOR reduction)
    uint8_t  reserved[32];                  // [32:64] Future extensions
} dvsm_rf_elf_sample_t;

#pragma pack(pop)

_Static_assert(sizeof(dvsm_rf_elf_sample_t) == 64, "RfElfSample must be 64 bytes");
_Static_assert(_Alignof(dvsm_rf_elf_sample_t) == 64, "RfElfSample must be cache-line aligned");

// Integrity check
int dvsm_rf_elf_validate_sample(const dvsm_rf_elf_sample_t *sample) {
    // Returns 1 if hash is valid, 0 otherwise
}
```

---

### §1.4 Ring Buffer (256 × 64-byte samples)

```c
// dvsm_rf_elf_queue.h

typedef struct {
    dvsm_rf_elf_sample_t buffer[256];      // 16 KB fixed capacity
    _Atomic(uint32_t) head;                // Producer write index (atomic)
    _Atomic(uint32_t) tail;                // Consumer read index (atomic)
    _Atomic(uint64_t) last_poll_ns;        // Last successful pop timestamp
    _Atomic(int) stale_flag;               // Set if no activity in threshold
} dvsm_rf_elf_ring_buffer_t;

_Static_assert(sizeof(dvsm_rf_elf_ring_buffer_t) == 16384 + 32, "Ring buffer ~16KB");

// User-facing constructors (user is responsible for thread-safety of allocation)
dvsm_rf_elf_ring_buffer_t* dvsm_rf_elf_ring_buffer_create(void);
void dvsm_rf_elf_ring_buffer_destroy(dvsm_rf_elf_ring_buffer_t *buffer);
```

---

## §2 Core API Functions

### §2.1 Session Initialization (Fail-Fast)

```c
// dvsm_init.h

/**
 * Initialize a DVSM session with runtime configuration.
 *
 * PRECONDITIONS:
 *   - config must be valid (use dvsm_validate_config first)
 *   - if config->enable_rf_elf_coupling: config->rf_elf_buffer must be non-null
 *   - if config->rf_elf_buffer: must point to a valid RfElfRingBuffer
 *
 * POSTCONDITIONS (if result == DVSM_OK):
 *   - *out_session points to valid, initialized session
 *   - session->h_session is bound to config and buffer presence
 *   - All state vectors (Z, S, W) initialized to identity
 *
 * POSTCONDITIONS (if result != DVSM_OK):
 *   - *out_session remains unchanged (not allocated)
 *   - Caller must NOT use out_session
 *
 * ERROR HANDLING:
 *   - DVSM_OK: Success, session ready
 *   - DVSM_ERR_MODALITY_MISSING: enable_rf_elf=true but buffer=null (FATAL)
 *   - DVSM_ERR_INVALID_BUFFER_SIZE: buffer capacity != 256 (FATAL)
 *   - DVSM_ERR_INVALID_CONFIG: config validation failed (FATAL)
 *
 * LIFETIME:
 *   - Caller must call dvsm_session_destroy() to free memory
 */
dvsm_error_code_t dvsm_session_init_ffi(
    const dvsm_config_t *config,
    dvsm_session_t **out_session
);
```

**Implementation Checklist:**
```c
1. Validate config (frame_rate, sync_tier, coefficients)
2. Check RF/ELF precondition: if enable && buffer==null → return ERR_MODALITY_MISSING
3. Check RF/ELF buffer: if provided, capacity must be 256
4. Allocate session struct
5. Initialize state vectors (Z, S, W)
6. Compute H_session = HASH(config ⊕ version ⊕ buffer_presence)
7. Record config immutably in session
8. Return DVSM_OK, populate out_session
```

---

### §2.2 Supervisor Tick (Main Loop)

```c
// dvsm_tick.h

/**
 * Execute one supervisor tick (120 Hz, 8.33 ms budget).
 *
 * PRECONDITIONS:
 *   - session must be initialized (from dvsm_session_init_ffi)
 *   - input_frame must be valid observation frame
 *
 * POSTCONDITIONS:
 *   - session->state updated with new Z, S, W vectors
 *   - session->last_error set (DVSM_OK or non-fatal error code)
 *   - No blocking: even if RF/ELF has error, supervisor completes on-time
 *
 * LATENCY GUARANTEE:
 *   - Critical path: < 7.36 ms (88.3% of 8.33 ms budget)
 *   - Phase I.0.5 (RF/ELF poll): < 50 ns (L1-hit memcpy)
 *   - NO frame drops on modality errors
 *
 * ERROR HANDLING:
 *   - return DVSM_OK: All operations completed, no errors
 *   - return DVSM_ERR_MODALITY_STALE: RF/ELF buffer hasn't updated (non-fatal)
 *   - return DVSM_ERR_MODALITY_OVERFLOW: Ring buffer full, sample dropped (non-fatal)
 *   - return DVSM_ERR_SESSION_HASH_MISMATCH: Config changed (should not happen)
 *
 * NOTE: Non-fatal errors do NOT stop the supervisor. The error is logged
 *       in session->last_error for telemetry/monitoring.
 */
dvsm_error_code_t dvsm_tick(
    dvsm_session_t *session,
    const dvsm_input_frame_t *input_frame
);
```

**Phase Sequence (within dvsm_tick):**
```
Phase A-B: Core state update (Lie bracket, backreaction, clamping)
Phase C: Envelope calculation
Phase D: Display geometry
Phase E: VR quaternion renormalization (if enabled)
Phase F: Frame generation (if enabled)
Phase G: Hash binding (H_t)
Phase H: GPU SRI upload (VRS)
Phase I.0.5: RF/ELF try-pop + coupling (NEW, Model B)
Phase I.1-I.3: GPU synchronization
Phase J: Audit logging (if enabled)
```

---

### §2.3 RF/ELF Modality Poll (Phase I.0.5)

```c
// dvsm_modality.h

/**
 * Poll RF/ELF ring buffer for latest sample and apply coupling.
 *
 * CALLED BY: dvsm_tick, Phase I.0.5
 *
 * PRECONDITIONS:
 *   - session->config.enable_rf_elf_coupling must be true
 *   - session->config.rf_elf_buffer must be non-null (validated at init)
 *
 * BEHAVIOR:
 *   - Attempt non-blocking try_pop() from ring buffer
 *   - If sample available: validate integrity, apply coupling
 *   - If sample stale: skip coupling, set error code
 *   - If buffer overflow: skip, set error code
 *   - ALWAYS: continue to next phase (no blocking)
 *
 * LATENCY:
 *   - Success path: ~20-50 ns (L1-hit memcpy + validation)
 *   - Error path: ~5-10 ns (early-out on empty/stale)
 *
 * ERROR HANDLING:
 *   - Returns latest error code for telemetry
 *   - Non-fatal: supervisor loop continues regardless
 *
 * OUTPUT:
 *   - session->state updated with RF/ELF coupling (if sample valid)
 *   - session->last_error set
 *   - session->modality_error_count incremented (for telemetry)
 */
dvsm_error_code_t dvsm_rf_elf_try_pop(
    dvsm_session_t *session
);
```

**Implementation Outline:**
```c
dvsm_error_code_t dvsm_rf_elf_try_pop(dvsm_session_t *session) {
    if (!session->config.enable_rf_elf_coupling) {
        return DVSM_OK_MODALITY_DISABLED;
    }
    
    dvsm_rf_elf_ring_buffer_t *buffer = 
        (dvsm_rf_elf_ring_buffer_t *)session->config.rf_elf_buffer;
    
    // Phase I.0.5.1: Non-blocking try-pop
    dvsm_rf_elf_sample_t sample;
    int has_sample = dvsm_rf_elf_ring_buffer_try_pop(buffer, &sample);
    
    if (!has_sample) {
        // Check staleness
        if (dvsm_rf_elf_ring_buffer_is_stale(buffer, session->config.rf_elf_stale_threshold_ms)) {
            session->last_error = DVSM_ERR_MODALITY_STALE;
            session->modality_stale_count++;
            return DVSM_ERR_MODALITY_STALE;  // Non-fatal, continue
        }
        return DVSM_OK;  // Empty buffer, normal
    }
    
    // Phase I.0.5.2: Validate integrity
    if (!dvsm_rf_elf_validate_sample(&sample)) {
        session->last_error = DVSM_ERR_MODALITY_CORRUPTED;
        session->modality_corrupted_count++;
        return DVSM_ERR_MODALITY_CORRUPTED;  // Non-fatal, skip coupling
    }
    
    // Phase I.0.5.3: Apply RF/ELF coupling
    dvsm_apply_rf_elf_coupling(session, &sample);
    
    session->last_error = DVSM_OK;
    return DVSM_OK;
}
```

---

### §2.4 Session Cleanup

```c
// dvsm_cleanup.h

/**
 * Destroy a DVSM session and free all allocated memory.
 *
 * PRECONDITIONS:
 *   - session must be initialized (from dvsm_session_init_ffi)
 *
 * POSTCONDITIONS:
 *   - All memory freed
 *   - session pointer is invalid after this call
 *
 * NOTE: User is responsible for destroying RF/ELF ring buffer separately
 *       (dvsm_rf_elf_ring_buffer_destroy).
 */
void dvsm_session_destroy(dvsm_session_t *session);
```

---

## §3 Query Functions (Read-Only)

### §3.1 Get Last Error

```c
// dvsm_query.h

/**
 * Query the last error code from the session.
 *
 * SEMANTICS:
 *   - Returns error code from most recent dvsm_tick()
 *   - Does NOT clear the error (call multiple times safely)
 *   - Use for telemetry/monitoring
 *
 * EXAMPLE:
 *   dvsm_error_code_t err = dvsm_get_last_error(session);
 *   if (err == DVSM_ERR_MODALITY_STALE) {
 *       printf("Warning: RF/ELF producer is stale\n");
 *   }
 */
dvsm_error_code_t dvsm_get_last_error(dvsm_session_t *session);
```

---

### §3.2 Get Session Hash

```c
// dvsm_query.h

/**
 * Query the session's structural hash (H_session).
 *
 * SEMANTICS:
 *   - Returns immutable hash computed at init
 *   - Binds config, protocol version, and buffer presence
 *   - Two sessions with identical config and buffer presence have identical H_session
 *   - Use for cryptographic session validation / peer consensus
 *
 * INVARIANT:
 *   H_session = HASH(config ⊕ protocol_version ⊕ buffer_presence)
 *
 * EXAMPLE:
 *   uint64_t h1 = dvsm_get_session_hash(session);
 *   uint64_t h2 = dvsm_get_session_hash(peer_session);
 *   if (h1 == h2) {
 *       printf("Sessions are structurally identical\n");
 *   }
 */
uint64_t dvsm_get_session_hash(dvsm_session_t *session);
```

---

### §3.3 Get Modality Telemetry

```c
// dvsm_query.h

typedef struct {
    uint64_t last_error;                    // Latest error code
    uint64_t modality_stale_count;          // Cumulative stale errors
    uint64_t modality_overflow_count;       // Cumulative overflow drops
    uint64_t modality_corrupted_count;      // Cumulative corruption errors
    uint64_t samples_received_total;        // Total samples successfully polled
} dvsm_modality_telemetry_t;

/**
 * Query RF/ELF modality telemetry.
 *
 * USE CASE:
 *   - Monitor producer thread health
 *   - Detect patterns (e.g., "overflow_count growing" → producer too fast)
 *   - Dashboard metrics
 *
 * EXAMPLE:
 *   dvsm_modality_telemetry_t tel;
 *   dvsm_get_modality_telemetry(session, &tel);
 *   printf("Stale errors: %lu, Overflows: %lu\n", tel.modality_stale_count, tel.modality_overflow_count);
 */
void dvsm_get_modality_telemetry(
    dvsm_session_t *session,
    dvsm_modality_telemetry_t *out_telemetry
);
```

---

## §4 Configuration Validation

```c
// dvsm_config.h

/**
 * Validate a configuration struct before use.
 *
 * CHECKS:
 *   - frame_rate_hz ∈ {60, 120, 240}
 *   - sync_tier ∈ {1, 2, 3}
 *   - q_mode ∈ {1=Q31, 2=Q16, 3=Q64.64}
 *   - All Q31.32 coefficients in representable range
 *   - protocol_version >= 0x0301
 *
 * RETURNS:
 *   - DVSM_OK if all checks pass
 *   - DVSM_ERR_INVALID_CONFIG if any check fails
 */
dvsm_error_code_t dvsm_validate_config(const dvsm_config_t *config);

/**
 * Create a default configuration (safe to use immediately).
 *
 * DEFAULTS:
 *   - frame_rate_hz = 120
 *   - sync_tier = 1 (proportional, safest)
 *   - q_mode = Q31 (standard)
 *   - All kill switches enabled
 *   - RF/ELF coupling disabled (require explicit opt-in)
 *
 * EXAMPLE:
 *   dvsm_config_t config = dvsm_config_default();
 *   config.frame_rate_hz = 240;  // Customize
 *   dvsm_session_init_ffi(&config, &session);
 */
dvsm_config_t dvsm_config_default(void);
```

---

## §5 Thread Affinity (Recommended for Ally X Z2)

```c
// dvsm_affinity.h (OPTIONAL, platform-specific)

/**
 * Recommended thread affinity for Ally X Z2 Extreme (4-core).
 *
 * TOPOLOGY:
 *   Core 0: Supervisor (DVSM critical path)
 *   Core 1: RF/ELF producer (external, user's responsibility)
 *   Cores 2–3: OS, other workloads
 *
 * RECOMMENDATION:
 *   1. Pin supervisor thread to Core 0
 *   2. Pin external RF/ELF producer thread to Core 1
 *   3. This prevents L1/L2 cache thrashing
 *
 * EXAMPLE (Linux):
 *   cpu_set_t cpuset;
 *   CPU_ZERO(&cpuset);
 *   CPU_SET(0, &cpuset);  // Core 0
 *   pthread_setaffinity_np(supervisor_thread, sizeof(cpu_set_t), &cpuset);
 *
 *   CPU_ZERO(&cpuset);
 *   CPU_SET(1, &cpuset);  // Core 1
 *   pthread_setaffinity_np(producer_thread, sizeof(cpu_set_t), &cpuset);
 */
void dvsm_recommend_thread_affinity_ally_x_z2(void);
// (Informational only; user implements actual pinning)
```

---

## §6 Memory Layout & ABI Safety

### Struct Sizes (Verification)

```c
// Compile-time assertions (prevent silent ABI breaks)
_Static_assert(sizeof(dvsm_config_t) == 256, "config must be 256 bytes");
_Static_assert(sizeof(dvsm_rf_elf_sample_t) == 64, "sample must be 64 bytes");
_Static_assert(_Alignof(dvsm_rf_elf_sample_t) == 64, "sample must be 64-byte aligned");
_Static_assert(sizeof(dvsm_rf_elf_ring_buffer_t) < 20000, "ring buffer ~16KB");
```

---

## §7 Summary Table

| Function | Layer | Signature | Behavior |
|----------|-------|-----------|----------|
| `dvsm_session_init_ffi` | Init | `(config, &session) → error_code` | Fail-fast, validates RF/ELF precondition |
| `dvsm_tick` | Runtime | `(session, input) → error_code` | Main loop, no blocking |
| `dvsm_rf_elf_try_pop` | Runtime (Phase I.0.5) | `(session) → error_code` | ~50ns, non-blocking |
| `dvsm_session_destroy` | Cleanup | `(session) → void` | Free all memory |
| `dvsm_get_last_error` | Query | `(session) → error_code` | Read-only telemetry |
| `dvsm_get_session_hash` | Query | `(session) → uint64_t` | Immutable, consensus-critical |
| `dvsm_get_modality_telemetry` | Query | `(session, &tel) → void` | Modality diagnostics |
| `dvsm_validate_config` | Helper | `(config) → error_code` | Pre-init validation |
| `dvsm_config_default` | Helper | `() → config_t` | Safe defaults |
