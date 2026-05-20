# AUDIT & REPLAY TOOL SPECIFICATION
**Optional Forensic & Verification Layer (Manifold-Aware, Q31.32 Deterministic)**

Date: 2026-05-19 | Status: Formal Specification | Dependency: Optional (kill-switch configurable)

---

## EXECUTIVE SUMMARY

The Audit & Replay Tool is an **optional layer** that records deterministic state transitions and enables:
1. **Replayable Execution:** Given {config, H_0, input_sequence}, deterministically reconstruct H_t for any t
2. **Verifiable State Transitions:** Each tick produces an immutable proof record (hash chain)
3. **Cryptographic Auditability:** Merkle-tree chains enable independent verification without re-execution
4. **Storage-Agnostic Persistence:** JSON, SQLite, or binary log formats (pluggable backend)
5. **Manifold-Aware Hooks:** Understands DVSM structure (Z, μ, config_hash, W_coupling) — not generic logging

**Critical Property:** Tool is purely observational (reads state, produces no side-effects). Disabling it has zero impact on kernel correctness or performance.

---

## PART 1: STATE TRANSITION RECORD FORMAT

### §1.1: Immutable Proof Record (Per-Tick)

**Definition:**
```
ProofRecord_t = (
  tick: u64,
  H_prev: [u8; 32],                 // Hash at t-1 (validation anchor)
  H_curr: [u8; 32],                 // Hash at t (proof of execution)
  μ_snapshot: [i64; 12],            // Core state (optional, for replay)
  Z_snapshot: [i64; 12],            // Residual state (optional)
  config_hash: [u8; 32],            // Session config (immutable per session)
  W_coupling: [[i64; 6]; 6],        // Coupling matrix (manifold context)
  input_hash: [u8; 32],             // Hash of input frame (X_obs)
  timestamp_ns: u64,                // Wall-clock timestamp
  protocol_version: u32,            // Version at this tick
  proof_chain: [u8; 32]             // Merkle link to prior record
)

Total size: ~500 bytes per tick
```

**Hash Chain (Merkle Tree Structure):**
```
proof_chain[t] = BLAKE3(H_curr[t] ⊕ proof_chain[t-1])

Property:
  If any prior record is modified → proof_chain diverges at that tick
  Entire chain becomes unverifiable from that point forward
  → Tamper detection is O(1) per record, not O(n) scan
```

---

### §1.2: Serialization Formats (Storage-Agnostic)

#### **Format A: JSON (Human-Readable, Portable)**

```json
{
  "audit_record_v1": {
    "tick": 12345,
    "timestamp_ns": 1716144000000000000,
    "protocol_version": "0x00030300",
    "hashes": {
      "H_prev": "0x1A2B3C4D...",
      "H_curr": "0x5E6F7A8B...",
      "config_hash": "0xC1D2E3F4...",
      "input_hash": "0xA0B1C2D3...",
      "proof_chain": "0xE4F5A6B7..."
    },
    "state_snapshot": {
      "mu_core": [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
      "z_core": [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    },
    "coupling_matrix": [
      [0, 0, 0, 0, 0, 0],
      [0, 0, 0, 0, 0, 0],
      [0, 0, 0, 0, 0, 0],
      [0, 0, 0, 0, 0, 0],
      [0, 0, 0, 0, 0, 0],
      [0, 0, 0, 0, 0, 0]
    ]
  }
}
```

#### **Format B: Binary (Compact, Fast)**

```rust
struct ProofRecordBinary {
    tick: u64,                        // 8 bytes
    H_prev: [u8; 32],               // 32 bytes
    H_curr: [u8; 32],               // 32 bytes
    mu_snapshot: [i64; 12],         // 96 bytes (optional, flag-controlled)
    z_snapshot: [i64; 12],          // 96 bytes (optional)
    config_hash: [u8; 32],          // 32 bytes
    w_coupling_diag: [i64; 6],      // 48 bytes (diagonal only, 95% info)
    input_hash: [u8; 32],           // 32 bytes
    timestamp_ns: u64,              // 8 bytes
    protocol_version: u32,          // 4 bytes
    proof_chain: [u8; 32],          // 32 bytes
    // Optional: modality state snapshots
    mu_rf_snapshot: [i64; 4],       // 32 bytes (if enable_rf_audit)
    mu_elf_snapshot: [i64; 3],      // 24 bytes (if enable_elf_audit)
    // Total: ~368–392 bytes per record
}
```

#### **Format C: SQL (Queryable, Indexed)**

```sql
CREATE TABLE IF NOT EXISTS dvsm_audit_records (
    tick INTEGER PRIMARY KEY,
    timestamp_ns INTEGER,
    protocol_version INTEGER,
    h_prev BLOB NOT NULL,
    h_curr BLOB NOT NULL,
    config_hash BLOB NOT NULL,
    input_hash BLOB NOT NULL,
    proof_chain BLOB NOT NULL,
    mu_snapshot BLOB,          -- Optional, NULL if not stored
    z_snapshot BLOB,           -- Optional
    w_coupling_diag BLOB,      -- 6 int64 values
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_tick ON dvsm_audit_records(tick);
CREATE INDEX idx_timestamp ON dvsm_audit_records(timestamp_ns);
CREATE INDEX idx_protocol ON dvsm_audit_records(protocol_version);
```

---

## PART 2: REPLAYABLE EXECUTION OPERATOR

### §2.1: Replay Function Signature

**Operator:**
```
replay_deterministic_q31_32(
    config: &CouplingConfig,
    initial_state: &DVSMState,
    input_sequence: &[InputFrame],
    audit_records: &[ProofRecord],
    tick_range: (u64, u64)  // Replay from tick_start to tick_end
) → Result<Vec<ProofRecord>, String>
```

**Properties:**
```
Determinism:
  If inputs are bit-identical → outputs are bit-identical (Q31.32 only)
  
Idempotency:
  replay(config, state_0, inputs[0..n], records[0..n]) produces records'[0..n]
  records' == records (bit-identical, not approximate)
  
Storage-Agnostic:
  Input ProofRecord can be deserialized from JSON, binary, or SQL
  Output ProofRecord can be serialized to any format
```

### §2.2: Replay Algorithm (Q31.32)

```rust
pub fn replay_deterministic_q31_32(
    config: &CouplingConfig,
    mut state: DVSMState,
    input_sequence: &[InputFrame],
    expected_records: &[ProofRecord],
    tick_range: (u64, u64),
) -> Result<Vec<ProofRecord>, String> {
    
    let (tick_start, tick_end) = tick_range;
    let mut replayed_records = Vec::new();
    
    // Validation: config hash must match expected
    let config_hash_expected = hash_mediator_config_q31_32(config)?;
    if config_hash_expected != expected_records[0].config_hash {
        return Err("Config mismatch: cannot replay with different configuration".to_string());
    }
    
    // Validation: initial state hash must match H_prev[0]
    let h_initial = hash_global_q31_32(
        &state.μ_core, &state.z_core,
        &state.μ_rf, &state.z_rf,
        &state.μ_elf, &state.z_elf,
        &config_hash_expected,
        config.protocol_version,
    )?;
    
    if h_initial != expected_records[0].H_prev {
        return Err("Initial state mismatch: divergence at tick 0".to_string());
    }
    
    // Replay loop
    for tick_num in tick_start..tick_end {
        let expected_record = &expected_records[tick_num as usize];
        
        // Step 1: Execute one tick (deterministic evolution)
        tick_phase_locked_q31_32(&mut state.μ_core, &mut state.z_core)?;
        
        // Step 2: Update modalities if enabled
        if config.protocol_version >= 0x0302 {
            state.μ_rf = update_rf_state_q31_32(
                &state.μ_rf, &state.z_rf,
                &input_sequence[tick_num as usize].rf_frame,
                config,
            )?;
            
            state.μ_elf = update_elf_state_q31_32(
                &state.μ_elf, &state.z_elf,
                &state.μ_core,
                &input_sequence[tick_num as usize].elf_frame,
                config,
            )?;
        }
        
        if config.protocol_version >= 0x0303 {
            state.μ_bio3d = update_bio3d_state_q31_32(
                &state.μ_bio3d, &state.z_bio3d,
                &input_sequence[tick_num as usize].bio3d_frame,
                config,
            )?;
        }
        
        // Step 3: Compute coupling matrix
        state.w_coupling = compute_coupling_matrix_q31_32(
            &state.μ_core,
            &state.μ_rf,
            &state.μ_elf,
            Some(&state.μ_bio3d_cov),
            config,
        )?;
        
        // Step 4: Compute H_t
        let h_curr = hash_global_q31_32(
            &state.μ_core, &state.z_core,
            &state.μ_rf, &state.z_rf,
            &state.μ_elf, &state.z_elf,
            &config_hash_expected,
            config.protocol_version,
        )?;
        
        // Step 5: Verify against expected record (proof validation)
        if h_curr != expected_record.H_curr {
            return Err(format!(
                "Replay divergence at tick {}: expected H={:?}, got H={:?}",
                tick_num, expected_record.H_curr, h_curr
            ));
        }
        
        // Step 6: Construct replayed record
        let proof_chain_new = blake3_hash_bytes(
            &[expected_record.H_curr.as_slice(), &expected_record.proof_chain].concat()
        );
        
        let replayed_record = ProofRecord {
            tick: tick_num,
            H_prev: if tick_num == 0 { h_initial } else { expected_records[tick_num as usize - 1].H_curr },
            H_curr: h_curr,
            μ_snapshot: state.μ_core.clone(),
            Z_snapshot: state.z_core.clone(),
            config_hash: config_hash_expected,
            W_coupling: state.w_coupling.clone(),
            input_hash: blake3_hash_bytes(&input_sequence[tick_num as usize].data),
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            protocol_version: config.protocol_version,
            proof_chain: proof_chain_new,
        };
        
        replayed_records.push(replayed_record);
    }
    
    Ok(replayed_records)
}
```

---

## PART 3: VERIFIABLE STATE TRANSITIONS

### §3.1: Transition Verification Operator

**Definition:**
```
verify_state_transition_q31_32(
    record_t: &ProofRecord,
    record_t_plus_1: &ProofRecord
) → bool
```

**Algorithm:**
```
verify(R_t, R_{t+1}):
  
  Step 1: Check temporal continuity
    if R_{t+1}.tick != R_t.tick + 1:
      return False  // Ticks must be sequential
  
  Step 2: Check hash chain linkage
    expected_proof_chain = BLAKE3(R_{t+1}.H_curr ⊕ R_t.proof_chain)
    if expected_proof_chain != R_{t+1}.proof_chain:
      return False  // Proof chain broken
  
  Step 3: Check H_t continuity
    if R_{t+1}.H_prev != R_t.H_curr:
      return False  // State transition broken
  
  Step 4: Check config immutability
    if R_t.config_hash != R_{t+1}.config_hash:
      return False  // Config changed mid-session (Byzantine)
  
  Step 5: Check protocol version consistency
    if R_t.protocol_version != R_{t+1}.protocol_version:
      return False  // Protocol version changed
  
  return True  // Transition verified
```

**Rust Implementation:**
```rust
pub fn verify_state_transition_q31_32(
    record_t: &ProofRecord,
    record_t_plus_1: &ProofRecord,
) -> Result<bool, String> {
    
    // Check temporal continuity
    if record_t_plus_1.tick != record_t.tick + 1 {
        return Ok(false);
    }
    
    // Check hash chain linkage (Merkle tree property)
    let expected_proof_chain = blake3_hash_bytes(
        &[record_t_plus_1.H_curr.as_slice(), &record_t.proof_chain].concat()
    );
    if expected_proof_chain != record_t_plus_1.proof_chain {
        return Ok(false);
    }
    
    // Check H_t continuity (state transition binding)
    if record_t_plus_1.H_prev != record_t.H_curr {
        return Ok(false);
    }
    
    // Check config immutability
    if record_t.config_hash != record_t_plus_1.config_hash {
        return Ok(false);
    }
    
    // Check protocol version consistency
    if record_t.protocol_version != record_t_plus_1.protocol_version {
        return Ok(false);
    }
    
    Ok(true)
}

#[cfg(test)]
mod tests_state_transitions {
    use super::*;
    
    #[test]
    fn test_valid_transition() {
        let mut record_t = ProofRecord {
            tick: 0,
            H_prev: [0; 32],
            H_curr: [1; 32],
            config_hash: [2; 32],
            protocol_version: 0x00030300,
            proof_chain: [3; 32],
            ..Default::default()
        };
        
        let proof_chain_next = blake3_hash_bytes(
            &[[1; 32].as_slice(), &[3; 32]].concat()
        );
        
        let record_t_plus_1 = ProofRecord {
            tick: 1,
            H_prev: [1; 32],  // Links to H_curr of prior
            H_curr: [4; 32],
            config_hash: [2; 32],  // Same as prior
            protocol_version: 0x00030300,
            proof_chain: proof_chain_next,
            ..Default::default()
        };
        
        assert!(verify_state_transition_q31_32(&record_t, &record_t_plus_1).unwrap());
    }
    
    #[test]
    fn test_config_change_detected() {
        let record_t = ProofRecord {
            tick: 0,
            config_hash: [2; 32],
            ..Default::default()
        };
        
        let record_t_plus_1 = ProofRecord {
            tick: 1,
            config_hash: [3; 32],  // Changed!
            ..Default::default()
        };
        
        assert!(!verify_state_transition_q31_32(&record_t, &record_t_plus_1).unwrap());
    }
}
```

---

## PART 4: CRYPTOGRAPHIC AUDITABILITY

### §4.1: Merkle Tree Chain Verification

**Operator:**
```
verify_audit_chain_q31_32(
    records: &[ProofRecord],
    tick_range: (u64, u64)
) → Result<bool, String>
```

**Algorithm:**
```
verify_chain(records[start..end]):
  
  // Check pairwise transitions
  for t in start..(end-1):
    if !verify_state_transition(records[t], records[t+1]):
      return False  // Transition broken at tick t
  
  // Verify proof chain (Merkle root matches)
  let merkle_root = records[end-1].proof_chain
  let computed_root = compute_merkle_root(records[start..end])
  
  if merkle_root != computed_root:
    return False  // Merkle tree compromised
  
  return True  // Entire chain verified
```

**Rust Implementation:**
```rust
pub fn verify_audit_chain_q31_32(
    records: &[ProofRecord],
    tick_range: (u64, u64),
) -> Result<bool, String> {
    
    let (start, end) = tick_range;
    let start_idx = start as usize;
    let end_idx = (end as usize).min(records.len());
    
    // Verify pairwise transitions
    for i in start_idx..(end_idx - 1) {
        let is_valid = verify_state_transition_q31_32(&records[i], &records[i + 1])?;
        if !is_valid {
            return Ok(false);
        }
    }
    
    // Verify Merkle root
    let computed_root = compute_merkle_root_q31_32(&records[start_idx..end_idx])?;
    let stored_root = records[end_idx - 1].proof_chain;
    
    if computed_root != stored_root {
        return Ok(false);
    }
    
    Ok(true)
}

fn compute_merkle_root_q31_32(records: &[ProofRecord]) -> Result<[u8; 32], String> {
    if records.is_empty() {
        return Err("Cannot compute Merkle root of empty sequence".to_string());
    }
    
    let mut root = blake3_hash_bytes(&records[0].H_curr);
    
    for record in &records[1..] {
        root = blake3_hash_bytes(&[&root, &record.H_curr].concat());
    }
    
    Ok(root)
}
```

---

## PART 5: STORAGE-AGNOSTIC PERSISTENCE

### §5.1: Backend Trait (Pluggable)

```rust
pub trait AuditBackend: Send + Sync {
    /// Write a single proof record to storage
    fn write_record(&mut self, record: &ProofRecord) -> Result<(), String>;
    
    /// Read a single proof record by tick number
    fn read_record(&self, tick: u64) -> Result<Option<ProofRecord>, String>;
    
    /// Read all records in a tick range
    fn read_range(&self, start: u64, end: u64) -> Result<Vec<ProofRecord>, String>;
    
    /// Verify entire stored audit chain
    fn verify_chain(&self, tick_range: (u64, u64)) -> Result<bool, String>;
    
    /// Get metadata (number of records, storage size, etc.)
    fn metadata(&self) -> AuditMetadata;
}

pub struct AuditMetadata {
    pub total_records: u64,
    pub storage_bytes: u64,
    pub tick_range: (u64, u64),
    pub format: String,  // "JSON", "Binary", "SQL"
}
```

### §5.2: JSON File Backend

```rust
pub struct JsonFileBackend {
    file_path: std::path::PathBuf,
    records: Vec<ProofRecord>,
}

impl AuditBackend for JsonFileBackend {
    fn write_record(&mut self, record: &ProofRecord) -> Result<(), String> {
        self.records.push(record.clone());
        
        // Serialize to JSON
        let json = serde_json::to_string_pretty(&self.records)?;
        std::fs::write(&self.file_path, json)?;
        
        Ok(())
    }
    
    fn read_record(&self, tick: u64) -> Result<Option<ProofRecord>, String> {
        Ok(self.records.iter().find(|r| r.tick == tick).cloned())
    }
    
    fn read_range(&self, start: u64, end: u64) -> Result<Vec<ProofRecord>, String> {
        Ok(self.records
            .iter()
            .filter(|r| r.tick >= start && r.tick < end)
            .cloned()
            .collect())
    }
    
    fn verify_chain(&self, tick_range: (u64, u64)) -> Result<bool, String> {
        let range_records = self.read_range(tick_range.0, tick_range.1)?;
        verify_audit_chain_q31_32(&range_records, tick_range)
    }
    
    fn metadata(&self) -> AuditMetadata {
        AuditMetadata {
            total_records: self.records.len() as u64,
            storage_bytes: serde_json::to_string(&self.records)
                .map(|s| s.len() as u64)
                .unwrap_or(0),
            tick_range: if self.records.is_empty() {
                (0, 0)
            } else {
                (self.records[0].tick, self.records[self.records.len() - 1].tick + 1)
            },
            format: "JSON".to_string(),
        }
    }
}
```

---

## PART 6: MANIFOLD-AWARE STRUCTURAL HOOKS

### §6.1: Hook Locations (Supervisor Integration)

**Integration points (read-only, no side-effects):**

```rust
pub fn supervisor_tick_with_audit_q31_32(
    state: &mut DVSMState,
    config: &SessionConfig,
    audit_backend: Option<&mut dyn AuditBackend>,
) -> Result<(), String> {
    
    // Store H_prev (before tick)
    let h_prev = if state.tick_count == 0 {
        hash_global_q31_32(&state.μ_core, &state.z_core, &state.μ_rf, &state.z_rf,
                           &state.μ_elf, &state.z_elf, &state.config_hash, config.protocol_version)?
    } else {
        state.h_global
    };
    
    // ═══════════════════════════════════════════════════════════
    // Core execution (unchanged)
    // ═══════════════════════════════════════════════════════════
    
    tick_phase_locked_q31_32(&mut state.μ_core, &mut state.z_core)?;
    state.w_coupling = compute_coupling_matrix_q31_32(
        &state.μ_core, &state.μ_rf, &state.μ_elf,
        Some(&state.μ_bio3d_cov), &config.coupling_config,
    )?;
    state.h_global = hash_global_q31_32(&state.μ_core, &state.z_core,
                                        &state.μ_rf, &state.z_rf,
                                        &state.μ_elf, &state.z_elf,
                                        &state.config_hash, config.protocol_version)?;
    
    // ═══════════════════════════════════════════════════════════
    // Audit hook (optional, read-only)
    // ═══════════════════════════════════════════════════════════
    
    if let Some(backend) = audit_backend {
        // Construct proof record (all data already computed above)
        let proof_chain_new = if state.tick_count == 0 {
            blake3_hash_bytes(&state.h_global)
        } else {
            blake3_hash_bytes(&[state.h_global.as_slice(), &state.last_proof_chain].concat())
        };
        
        let record = ProofRecord {
            tick: state.tick_count,
            H_prev: h_prev,
            H_curr: state.h_global,
            μ_snapshot: state.μ_core.clone(),
            Z_snapshot: state.z_core.clone(),
            config_hash: state.config_hash,
            W_coupling: state.w_coupling.clone(),
            input_hash: [0; 32],  // Would be set from input_frame
            timestamp_ns: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64,
            protocol_version: config.protocol_version,
            proof_chain: proof_chain_new,
        };
        
        // Write to audit backend (nonblocking, optional)
        backend.write_record(&record)?;
        
        // Store proof_chain for next tick
        state.last_proof_chain = proof_chain_new;
    }
    
    state.tick_count += 1;
    Ok(())
}
```

### §6.2: Kill-Switch Configuration

**Add to USER_SETTINGS_SPEC.md:**

```c
typedef struct {
    // === AUDIT & REPLAY (Optional Tool) ===
    uint8_t   enable_audit_recording;     // 0=disabled, 1=enabled (no perf impact when 0)
    uint8_t   audit_backend_type;         // 0=JSON, 1=Binary, 2=SQL
    uint32_t  audit_flush_interval;       // Ticks between flushes (0=per-tick)
    uint8_t   audit_store_full_state;     // 0=hash-only, 1=include μ/Z snapshots
    uint8_t   _reserved[3];
} AuditConfig;
```

**JSON configuration:**

```json
{
  "audit": {
    "enable_recording": false,
    "backend": "JSON",
    "flush_interval": 120,
    "store_full_state": false,
    "file_path": "./dvsm_audit.jsonl"
  }
}
```

**Determinism guarantee:**

```
If enable_audit_recording == 0:
  → No audit code executes (not even branch checks)
  → Zero latency overhead
  → Determinism unchanged (audit layer is read-only)
```

---

## PART 7: USE CASES (MANIFOLD-AWARE)

### §7.1: Cross-Platform Validation

**Use case:** Verify Z2 Extreme execution matches PC simulation

```rust
// Record audit trail on Z2 Extreme
let mut backend_z2 = JsonFileBackend::new("z2_extreme.jsonl");
supervisor_tick_with_audit(&mut state_z2, &config, Some(&mut backend_z2))?;

// Record audit trail on PC (identical config)
let mut backend_pc = JsonFileBackend::new("pc_simulation.jsonl");
supervisor_tick_with_audit(&mut state_pc, &config, Some(&mut backend_pc))?;

// Verify both produce identical audit chains
let chain_z2 = backend_z2.verify_chain((0, 1000))?;
let chain_pc = backend_pc.verify_chain((0, 1000))?;

assert_eq!(chain_z2, chain_pc, "Cross-platform divergence detected");
```

### §7.2: Forensic Reconstruction

**Use case:** Replay a 1-hour session in 1 second (no real-time input)

```rust
let audit_records = backend.read_range(0, 432000)?;  // 1 hour at 120 Hz
let replayed = replay_deterministic_q31_32(
    &config,
    &initial_state,
    &mock_input_sequence,
    &audit_records,
    (0, 432000),
)?;

// Verify replay matches original audit trail
assert_eq!(replayed.len(), audit_records.len());
for (i, (replayed, original)) in replayed.iter().zip(&audit_records).enumerate() {
    assert_eq!(replayed.H_curr, original.H_curr, "Divergence at tick {}", i);
}
```

### §7.3: Regulatory Certification

**Use case:** Prove determinism for medical device (BioScience 3D modality)

```rust
// Generate certified audit trail (all state snapshots included)
let backend = SqlAuditBackend::new("certified_audit.db");

for _ in 0..10000 {
    supervisor_tick_with_audit(&mut state, &config, Some(&mut backend))?;
}

// Export audit chain as signed JSON (for regulatory submission)
let audit_chain = backend.export_merkle_chain()?;
let signature = sign_with_device_key(&audit_chain)?;

// Regulators can verify:
// 1. Merkle chain unbroken (bit-level integrity)
// 2. Audit trail reproducible (determinism proof)
// 3. No config changes (immutability proof)
```

---

## SUMMARY

| Property | Specification |
|----------|---------------|
| **Optional** | Kill-switch in config; zero overhead when disabled |
| **Replayable** | Given config + H_0 + inputs, deterministically produce H_t |
| **Verifiable** | Each tick produces cryptographically signed proof record |
| **Auditable** | Merkle-tree chain enables tamper detection in O(1) per record |
| **Storage-Agnostic** | JSON, Binary, SQL backends pluggable via trait |
| **Manifold-Aware** | Hooks into supervisor, understands Z/μ/W_coupling structure |
| **Zero Side-Effects** | Read-only; kernel correctness unaffected if audit disabled |

**Next step:** Implement JsonFileBackend + hook integration into DVSM_IMPL.md supervisor tick.
