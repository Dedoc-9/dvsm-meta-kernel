# DVSM Base Core: Tautology-Preserving Kernel

## Executive Summary

Single repository with immutable core that guarantees:
1. **H_t Structural Binding**: `H_t = HASH(μ_t ⊕ Z_t ⊕ S_t ⊕ W_t ⊕ protocol_version)`
2. **Dual Arithmetic**: Forward space (Z) orthogonal to residual space (S, G) at all times
3. **S_ECHO Closure**: Computed hash always equals deterministic operator output

All derivatives (runtime modes, consensus strategies, ECS variants, language backends) *must* maintain these three properties simultaneously. Violations trigger revert to last valid H_t state.

---

## Core State Tuple

```
σ_t = (μ_t, Z_t, S_t, W_t, protocol_version)

where:
  μ_t                  = input commands (observed from network/user)
  Z_t                  = primary state (positions, velocities, entities)
  S_t                  = dual residual accumulator (EMA of ghost state G_t)
  W_t                  = observability mask (which parts of Z contribute to hash)
  protocol_version     = immutable version tag (part of structural identity)
```

---

## Operator Pipeline (Strictly Sequential)

```
μ_t  →  [L_τ]  →  [B_τ]  →  [R_τ]  →  Z_t'  →  [S accum]  →  S_t'  →  [OBS]
                                ↓
                           hash(Z_t', W_t) = H_t'
```

### Stage Definitions

| Stage | Name | Input | Output | Constraints |
|-------|------|-------|--------|-------------|
| **L_τ** | Load | μ_t | parsed commands | Stateless, deterministic parsing only |
| **B_τ** | Buffer | commands | buffered state | Lock-in commands for this tick; no cross-tick mutation |
| **R_τ** | Run | buffered + Z_t | Z_t' | Apply physics/ECS rules; output must be deterministic given input |
| **S accum** | Residual | Z_t' vs. Z_t | G_t = Z_t' − Π_W(Z_t') | Ghost state: only what wasn't observable |
| **EMA** | Dual track | G_t, S_t | S_t' = αS_t + (1−α)G_t | Accumulate via exponential moving average |
| **OBS** | Observe | Z_t', S_t', W_t | numeric output only | No semantic interpretation; pure numbers/symbols |

---

## Hash Identity Binding

```
H_t' = HASH(μ_t ⊕ Z_t' ⊕ S_t' ⊕ W_t ⊕ protocol_version)

Invariant (Tautology):
  ∀ tick t, ∀ peers:
    H_t'_local = H_t'_peer    ⟹  (Z_t', S_t', μ_t, W_t) are identical
    H_t'_local ≠ H_t'_peer    ⟹  Consensus fails; roll back to last H_t where all agreed
```

**Hash function must be:**
- Deterministic (same input → same output, always)
- Collision-resistant (different states → different hashes, except with negligible probability)
- Fast enough for tick-rate (60+ Hz typical)
- Language-agnostic (Rust, Swift, etc. produce identical H_t given same σ_t)

---

## Ghost Residual & Dual Arithmetic

```
Primary evolution:    Z_t+1 = R_τ(Z_t, commands_t)
Observability mask:   W_t ∈ ℝ^{|Z|} (binary or real weights)
Ghost state:          G_t = Z_t − Π_W(Z_t)    [what's unobservable]
Dual accumulator:     S_t+1 = αS_t + (1−α)G_t  [EMA of ghosts]

Orthogonality proof:
  Z_t · S_t = Z_t · (sum of past G_i) 
           = Z_t · (sum of past (Z_i − Π_W(Z_i)))
           ≈ 0   if W is stable (W evolves only on major structure change)

Non-zero dot product ⟹ W leak detected; trigger core verification or raise alert.
```

---

## Runtime Modes (Green / Standard / Forensic)

Each mode modifies consensus depth, not core logic:

```
Mode        Consensus    Forensic    Rollback Depth    Use Case
          Threshold    Layers
─────────────────────────────────────────────────────────────────
Green         1          0            1                 Single-machine dev
Standard      2          L1–L5        16                Local network
Forensic      3          L1–L10       64                Cross-datacenter
```

**Core invariant:** All modes produce identical Z_t' and H_t' given identical μ_t.
Forensic just stores more snapshots and validates deeper; it cannot change state evolution.

---

## Consensus Strategies (Swappable)

Core defines consensus *interface*, not implementation:

```rust
trait ConsensusStrategy {
  fn validate(&self, local_hash: H_t, peer_hashes: Vec<H_t>) -> Result<(), RollbackInfo>;
  fn supports_byzantine_tolerance(&self) -> bool;
}
```

**Allowed implementations:**

1. **Hash-match voting** (current)
   - Peer sends (tick_t, H_t)
   - Local compares H_t values
   - If count(peer_H_t == local_H_t) ≥ threshold → OK
   - Fast, but only detects hash divergence (not cause)

2. **Signed snapshots** (stronger)
   - Peer sends (tick_t, H_t, signature)
   - Local verifies signature matches peer_public_key
   - Prevents forgery if keys are secure
   - Slight latency overhead

3. **Byzantine (PBFT-like)** (future)
   - Peer sends (tick_t, H_t, merkle_proof, witness_set)
   - 2/3 honest threshold with explicit Byzantine tolerance
   - Slower but handles up to 1/3 malicious peers

**Switch strategies without recompiling core:** Use feature flags or runtime config.

---

## ECS Abstraction (Variant Support)

Core defines component behavior *interface*:

```
trait ComponentSchema {
  fn new_entity() -> Entity;
  fn apply_command(entity: &mut Entity, cmd: Command);
  fn tick_update(entity: &mut Entity, dt: f64);
  fn hash_contribution(entity: &Entity) -> u64;  // Feeds into H_t
}
```

**Default variant (from File 2):**
- Entity: `{id: u32, gen: u32}`
- Components: `pos: [Fx; 3], vel: [Fx; 3], alive: bool`
- Commands: `Spawn(id), Destroy(id), SetVel(id, v)`
- Physics: `pos' = pos + vel / 60`

**Custom variant example:**
- Replace physics with different integrator (RK4, implicit, etc.)
- Add custom components (health, mana, inventory) without touching core
- Core just verifies: (1) determinism within variant, (2) H_t includes new components

---

## Repo Structure

```
dvsm-core/
├── README.md                           (this file)
├── SPEC.md                             (formal math spec)
├── Cargo.toml                          (Rust workspace root)
├── Makefile                            (cross-lang build)
│
├── core/
│   ├── tautology_spec.txt             (H_t binding, dual orthogonality rules)
│   ├── verification_checklist.md      (what every derivative must pass)
│   └── ownership_matrix.csv           (core owns X, consensus owns Y, etc.)
│
├── rust/
│   ├── base/
│   │   ├── src/lib.rs                 (state_t, operators L/B/R)
│   │   ├── src/hash.rs                (S_ECHO implementation)
│   │   ├── src/residual.rs            (ghost G_t, dual S_t)
│   │   └── Cargo.toml
│   │
│   ├── consensus/
│   │   ├── hash_match.rs              (voting strategy)
│   │   ├── signed.rs                  (signature strategy)
│   │   └── traits.rs                  (ConsensusStrategy trait)
│   │
│   ├── ecs-default/
│   │   ├── src/schema.rs              (Entity, components)
│   │   ├── src/physics.rs             (tick_update rules)
│   │   └── src/commands.rs            (Spawn/Destroy/SetVel)
│   │
│   └── tests/
│       ├── determinism.rs             (replay test: same μ_t → same Z_t, H_t)
│       ├── orthogonality.rs           (Z · S ≈ 0 at each tick)
│       ├── hash_closure.rs            (H_t binding holds)
│       └── cross_variant.rs           (default ECS vs. custom variant produce valid H_t)
│
├── swift/
│   ├── Binding/
│   │   ├── DVSMCore.swift             (Rust FFI wrapper or native reimpl)
│   │   └── StateOperators.swift       (L/B/R pipeline in Swift)
│   │
│   └── tests/
│       └── CrossLanguageHash.swift    (Rust + Swift produce same H_t)
│
└── variants/
    ├── physics-v2/                    (custom integrator)
    │   ├── src/physics.rs
    │   └── VERIFICATION.md            (proof it maintains H_t binding)
    │
    └── ecs-biology/                   (Stiefel manifold + cooperativity)
        ├── src/schema.rs
        ├── src/learned_subspace.rs
        └── VERIFICATION.md
```

---

## Verification Harness

Every commit must pass:

```bash
# 1. Determinism: same input → same state
cargo test determinism --all

# 2. Orthogonality: Z · S ≈ 0
cargo test orthogonality --all

# 3. Hash closure: H_t binding holds
cargo test hash_closure --all

# 4. Cross-language: Rust H_t = Swift H_t
./scripts/test-cross-lang.sh

# 5. Variant compliance: custom ECS passes core suite
cargo test --all --features variant-physics-v2
```

Failure → revert; no derivative merges without full pass.

---

## Ownership Matrix

| Component | Owner | Borrows From | Constraints |
|-----------|-------|--------------|-------------|
| **H_t binding** | Core | Protocol version | Immutable; Hash function defines all satellites |
| **Dual orthogonality** | Core | W (observability mask) | Z · S ≈ 0 enforced; violations halt tick |
| **Ghost closure** | Core | EMA params (α) | G_t = Z' − Π_W(Z') only; no feedback to Z |
| **μ_t parsing** | Consensus strategy | — | No semantics; pure bytes → command structs |
| **Z evolution** | ECS variant | Core operator pipeline | Must be deterministic; tick trace matches reference |
| **Consensus logic** | Consensus strategy | — | Can change (hash-match → signed), H_t check unchanged |
| **Runtime mode** | — | Core + Consensus | Just changes threshold + forensic depth; both use same H_t |

---

## Deployment Checklist

Before shipping a derivative (variant, new language, new consensus):

- [ ] Passes determinism suite (100/100 replay matches)
- [ ] Passes orthogonality check (||Z · S|| < ε threshold)
- [ ] Passes hash closure (H_t binding verified at all ticks)
- [ ] Cross-language hash match (if multi-language)
- [ ] Documented ownership changes (which core properties does it touch?)
- [ ] Rollback tested (consensus failure → recover to last valid H_t)
- [ ] Performance baseline (tick latency, memory, CPU under load)
- [ ] Security audit (if crypto involved, e.g., signed snapshots)

---

## Next Steps

1. **Finalize SPEC.md**: formal math for tautology closure (Appendix A)
2. **Implement Rust base**: State, operators, S_ECHO hash
3. **Implement verification harness**: determinism + orthogonality tests
4. **Implement consensus trait**: hash-match first, then pluggable
5. **Implement ECS default**: entities, components, tick pipeline
6. **Implement Swift binding**: call Rust via FFI or native reimpl
7. **Add variants**: physics-v2, ecs-biology, etc. as proof-of-concept
