# DVSM Gravitational Backreaction

**Author:** Daniel J. Dillberg  
**Date:** 2026-05-19  
**Status:** CRITICAL — integrates into core H_t binding and energy conservation

---

## §1 The Problem: Norm Collapse Under Dissipation

Without backreaction, pure Lie-bracket + dissipation gives:

```
dZ/dt = [Z,S]_κ − λZ

Energy evolution (κ antisymmetric):
d‖Z‖²/dt = Z · dZ/dt + dZ/dt · Z
         = −2λ‖Z‖²         (coupling term vanishes by antisymmetry)

Solution: ‖Z(t)‖² = ‖Z₀‖² · exp(−2λt)
```

**Pathology:** With large λ (strong dissipation), norm → 0 exponentially. System collapses to zero attractor. Dynamical freedom disappears.

**In the DVSM context:** Norm collapse means:
- Z components drop below numerical precision
- Ghost basis vectors cannot be rebirthd (nothing to restore from S)
- Hash H_t becomes sensitive to tiny numerical errors
- Cross-language determinism breaks (different platforms round at different times)

---

## §2 Backreaction: Restoring Force in State Space

### §2.1 Core Equation with Backreaction

```
REVISED:

dZ_k/dt = Σⱼ κ_{kj}(Z_k S_j − Z_j S_k) − λZ_k  +  B_k

where  B_k = −α(‖Z‖² − E_target) · Z_k

         α ∈ ℝ⁺     backreaction strength (tunable, protocol-frozen)
         E_target   target norm squared (typically 1.0)
```

### §2.2 Physical Interpretation

**Without backreaction:** dissipation wins; norm → 0.

**With backreaction:**
- When ‖Z‖² > E_target: B_k < 0 (negative feedback, damps norm)
- When ‖Z‖² < E_target: B_k > 0 (positive feedback, restores norm)
- Equilibrium: ‖Z‖² → E_target as t → ∞ (if stable)

**Energy landscape analogy:**
```
Free energy: F(Z) = ½λ‖Z‖² + ½α(‖Z‖² − E_target)²

dZ/dt is gradient flow on F, plus the Lie-bracket mixer.
Backreaction term ≡ −∇_Z F (gradient of F w.r.t. Z).
```

---

## §3 Energy Conservation Under Backreaction

### §3.1 Revised Energy Dynamics

```
d‖Z‖²/dt = 2Z · dZ/dt
         = 2Z · ([Z,S]_κ − λZ + B)
         = 2Z · [Z,S]_κ − 2λ‖Z‖² + 2Z · B
```

**Term 1:** `2Z · [Z,S]_κ = 0` (antisymmetry of κ)

**Term 2:** `−2λ‖Z‖²` (dissipation, always negative)

**Term 3:** `2Z · B = 2Z · (−α(‖Z‖² − E_target) · Z)`
           `= −2α‖Z‖²(‖Z‖² − E_target)`

### §3.2 Equilibrium Analysis

At equilibrium (dZ/dt = 0):

```
d‖Z‖²/dt = −2λ‖Z‖² − 2α‖Z‖²(‖Z‖² − E_target)
         = −2‖Z‖²[λ + α(‖Z‖² − E_target)]
```

Setting `‖Z‖² = E_target`:

```
d‖Z‖²/dt = −2λE_target < 0    (system still leaks energy to dissipation)
```

**Key insight:** Backreaction alone doesn't conserve energy—it **stabilizes the norm** around E_target. Dissipation λ still runs, but operating at a bounded norm rather than an exponentially decaying one.

---

## §4 Integration into State Tuple & H_t Binding

### §4.1 Extended State

```
σ_t = (μ_t, Z_t, S_t, W_t, κ, λ, α, E_target, protocol_version)

where NEW parameters:
  α ∈ ℝ⁺              backreaction strength (PROTOCOL-FROZEN)
  E_target ∈ ℝ⁺       target norm squared (PROTOCOL-FROZEN, typically 1.0)
```

### §4.2 Revised Hash Binding

```
H_t = HASH(
    μ_t 
    ⊕ Z_t 
    ⊕ S_t 
    ⊕ W_t 
    ⊕ κ 
    ⊕ λ 
    ⊕ α 
    ⊕ E_target
    ⊕ protocol_version
)
```

**Critical:** α and E_target are **immutable per protocol version**. All peers with same protocol_version must use identical α, E_target.

### §4.3 Power-Rail Telemetry Integration

From the DVSM-v3 code: λ and α are **dynamically scaled** based on available power:

```
b = actual_watts / tdp_ceiling  ∈ [0, 1]

λ_actual = λ_base · (0.5 + 0.5 · b)      [never fully disabled]
α_actual = α_base · b                    [scales to zero at low power]
```

**Hash implication:**
- Does telemetry affect H_t?

**Answer:** NO. λ_base and α_base are frozen in protocol_version. Telemetry creates **runtime parameter mutations**, not **state mutations**. Z_t, S_t, H_t remain deterministic if μ_t is identical and {λ_base, α_base} are frozen.

However, **cross-node consensus requires:** all peers apply **identical telemetry scaling** or exclude telemetry from consensus (local throttling, not global). See §5 below.

---

## §5 Dual Arithmetic: Z · S Under Backreaction

### §5.1 Orthogonality Question

**Claim:** Backreaction preserves Z ⊥ S orthogonality.

```
Z_t = primary trajectory
S_t = EMA memory: S_t = βS_{t-1} + (1−β)Z_{t-1}

Z · S = Z_t · (sum of past Z_i with exponential weights)
```

**With backreaction:**

dZ includes term −α(‖Z‖² − E_target)·Z. This is **radial** (parallel to Z).

```
d(Z · S)/dt = dZ/dt · S + Z · dS/dt
            = ([Z,S]_κ − λZ + B) · S + Z · (β·dS/dt + (1−β)dZ/dt)
```

**First term:** `[Z,S]_κ · S` = 0 (antisymmetric bracket)
**Second term:** `−λZ · S` (dissipation of orthogonality)
**Third term:** `B · S = −α(‖Z‖² − E_target) · Z · S` (radial force × orthogonality)

**Result:** Backreaction's radial component does NOT directly break orthogonality. However, by stabilizing ‖Z‖², it **preserves the magnitude contrast** needed for Z ⊥ S to remain meaningful.

**Without backreaction:** Z → 0, so Z · S → 0 trivially (collapse, not orthogonality).
**With backreaction:** Z stays bounded, so Z · S ≈ 0 is a **robust invariant**.

### §5.2 Verification Test

```rust
#[test]
fn test_backreaction_orthogonality() {
    // Run 10k ticks with backreaction enabled
    for tick in 0..10_000 {
        dvsm_step(&mut state, &profile_with_backreaction);
        
        let dot = state.z.iter().zip(state.s.iter())
                       .map(|(zi, si)| zi * si)
                       .sum::<f32>();
        
        // Z · S should stay near zero, not decay to zero
        assert!(dot.abs() < 1e-6, "Orthogonality broken at tick {}: Z·S = {}", tick, dot);
        
        // Norm should oscillate around E_target, not decay
        assert!((state.norm_sq - profile.e_target).abs() < 0.1, 
            "Norm escaped E_target at tick {}: ‖Z‖² = {}", tick, state.norm_sq);
    }
}
```

---

## §6 Ghost Rebirth with Backreaction

### §6.1 Why Ghosts Matter

A "ghost" = Z_k that collapsed to near-zero but should stay alive.

**Without backreaction:**
- Z_k → 0 by dissipation
- S_k → some value (EMA lag)
- On rebirth attempt: Z_k := S_k · scale
- But S_k is also tiny (it's a history of Z_k)
- Rebirth fails; ghost stays dead

**With backreaction:**
- Z_k oscillates around some non-zero equilibrium
- Backreaction pulls it back if it drops too far
- S_k maintains a higher baseline (history of non-zero Z_k)
- Rebirth: Z_k := S_k · scale has meaningful value
- Ghost comes back to life; basis dimension preserved

### §6.2 Ghost Guard Logic

From DVSM-v3:

```rust
pub fn scan_and_rebirth(&mut self, state: &mut DVSMState) -> u32 {
    let mut reborn = 0u32;
    for k in 0..DIM {
        if state.z[k].abs() < collapse_threshold {
            // Rebirth from memory: S_k is a lagged average
            state.z[k] = state.s[k] * rebirth_scale;
            reborn += 1;
        }
    }
    reborn
}
```

**With backreaction enabled:**
- Threshold can be lower (because Z_k won't collapse to machine epsilon)
- Rebirth scale can be lower (because S_k has more "mass")
- Rebirths happen less frequently (system self-stabilizes)

### §6.3 Determinism: Rebirth as State Mutation

**Critical for H_t binding:**

Ghost rebirth **mutates Z**, so it **affects H_t**.

```
Before rebirth:  Z_k = 0.0001
After rebirth:   Z_k = 0.5 (from S_k)
                 ΔH_t = HASH_diff

Question: Do all peers rebirth at same tick?
```

**Answer:** YES, if:
1. Collapse threshold is identical (protocol-frozen)
2. S_k evolution is deterministic (EMA is, given same β)
3. Rebirth scale is identical (protocol-frozen)

**Verification test:**

```rust
#[test]
fn test_backreaction_rebirth_determinism() {
    // Run two instances with identical initial state
    let mut state1 = DVSMState::new_identity();
    let mut state2 = DVSMState::new_identity();
    
    for tick in 0..1000 {
        dvsm_step(&mut state1, &profile);
        dvsm_step(&mut state2, &profile);
        
        ghost_guard.scan_and_rebirth(&mut state1);
        ghost_guard.scan_and_rebirth(&mut state2);
        
        // Z and S must match exactly
        for k in 0..DIM {
            assert_eq!(state1.z[k].to_bits(), state2.z[k].to_bits(),
                "Z[{}] diverged after rebirth at tick {}", k, tick);
        }
        
        // H_t must match
        assert_eq!(state1.replay_hash, state2.replay_hash,
            "Hash diverged at tick {} (rebirth?)", tick);
    }
}
```

---

## §7 Backreaction in the Operator Pipeline

### §7.1 Revised R_τ Stage

```
Input:  Z_t, S_t, κ, λ, α, E_target
Output: Z_t+1, ‖Z_t+1‖²

Step 1: Lie bracket
   acc[k] := Σⱼ κ_{kj}(Z_k S_j − Z_j S_k)

Step 2: Backreaction term
   B[k] := −α(‖Z_t‖² − E_target) · Z_t[k]

Step 3: Euler update
   Z_t+1[k] := Z_t[k] + dt · (acc[k] − λ·Z_t[k] + B[k])

Step 4: Norm
   ‖Z_t+1‖² := Σₖ Z_t+1[k]²

Step 5: Memory
   S_t+1[k] := β·S_t[k] + (1−β)·Z_t[k]
```

### §7.2 ABC Pipeline (DFE Variant)

Backreaction can be inserted into the **Transform** (B) layer:

```
A: OBSERVE
   z_obs := z

B: TRANSFORM (Lie + Backreaction)
   z_enc := lie_bracket(z, s, κ) − λ·z + backreaction(z, α, E_target)

C: ADAPT
   s_new := α_ema · s + (1 − α_ema) · z
   w := QR(w + η · residual ⊗ coeff)
```

**Note:** α_ema (EMA decay) ≠ α (backreaction strength). Avoid name collision.

---

## §8 Cross-Language Implementation

### §8.1 Rust (Reference)

Already provided in DVSM-v3 src/lib.rs. Key function:

```rust
pub fn dvsm_step(state: &mut DVSMState, p: &WattageProfile) {
    // ... lie_bracket ...
    let backreaction_coeff = -p.alpha * (state.norm_sq - p.e_target);
    for k in 0..DIM {
        let b_k = backreaction_coeff * state.z[k];
        let dz = p.dt * (acc[k] - p.lambda * state.z[k] + b_k);
        state.z[k] += dz;
        // ...
    }
}
```

### §8.2 Swift (Cross-Language Contract)

```swift
struct BackreactionParams {
    var alpha: Float
    var e_target: Float
}

func backreactionTerm(_ z: Vector, _ norm_sq: Float, _ params: BackreactionParams) -> Vector {
    let coeff = -params.alpha * (norm_sq - params.e_target)
    return z.map { zk in coeff * zk }
}

// In dvsm_step:
let b = backreactionTerm(state.z, state.norm_sq, br_params)
let dz = profile.dt * (lieAccum - profile.lambda * state.z + b)
state.z += dz
```

### §8.3 Numerical Equivalence Requirement

**Critical:** Backreaction must produce **bit-identical** results across Rust, Swift, Python, etc.

```
Checklist:
✓ α, E_target are protocol-frozen (same value everywhere)
✓ norm_sq computation is deterministic (same Z → same ‖Z‖²)
✓ Scalar multiplication: (norm_sq - E_target) · Z_k
  → Must use same rounding (IEEE 754 round-to-nearest)
✓ Addition: old_Z_k + dt * backreaction_term
  → Must use same FMA semantics or explicit order
```

**Verification:**

```rust
#[test]
fn test_backreaction_cross_language() {
    let z_rust = dvsm_step_rust(&state, &profile);
    let z_swift = call_swift_dfe_step(&state, &profile);  // FFI
    
    for k in 0..DIM {
        assert_eq!(z_rust[k].to_bits(), z_swift[k].to_bits(),
            "Backreaction divergence at Z[{}]: Rust={}, Swift={}",
            k, z_rust[k], z_swift[k]);
    }
}
```

---

## §9 Power-Rail Telemetry & Consensus

### §9.1 Dynamic Throttling (from DVSM-v3)

```
b = actual_watts / tdp_ceiling
λ_actual = λ_base · (0.5 + 0.5b)
α_actual = α_base · b
```

### §9.2 Hash Implication

**Question:** Does throttled λ_actual, α_actual affect H_t?

**Answer:** **NO**, if consensus operates on **protocol-level** parameters.

**Consensus rule:**
```
H_t = HASH(μ_t ⊕ Z_t ⊕ S_t ⊕ κ ⊕ λ_base ⊕ α_base ⊕ protocol_version)

NOT: H_t = HASH(...λ_actual...α_actual...)
```

**Why:** Different peers may have different power budgets (Xbox at 130W, Ally X at 35W). If actual throttling parameters affected consensus, peers would disagree on Z_t and fail consensus.

**What actually happens:**
1. All peers compute Z_t+1 using their own {λ_actual, α_actual}
2. Z_t+1 will **differ** across peers with different power
3. Consensus **fails** (hashes disagree)
4. Rollback to last known good state
5. OR: use **local-only** throttling (don't send Z to peers; only observations)

**Option A (strict consensus):** λ_base, α_base used for consensus. Throttling is local feedback only.

**Option B (relaxed consensus):** Each peer sends its own (Z_actual, power_budget) and peers agree on Z_base ignoring throttle.

---

## §10 Verification Checklist: Backreaction

Before shipping any variant with backreaction:

- [ ] **Energy conservation test**: ‖Z‖² stabilizes near E_target, not → 0
- [ ] **Orthogonality test**: Z · S stays bounded away from zero
- [ ] **Ghost rebirth determinism**: all peers rebirth at same ticks
- [ ] **Cross-language equivalence**: Rust ≈ Swift ≈ Python (bit-identical Z)
- [ ] **Norm dynamics test**: d‖Z‖²/dt ≈ −2λE_target at equilibrium
- [ ] **Hash chain integrity**: replay_hash matches after each backreaction update
- [ ] **Power throttle safety**: Z_base (protocol) vs Z_actual (throttled) documented
- [ ] **Stability margin**: choose α, λ such that system doesn't diverge or cycle
- [ ] **Floating-point precision**: reorthonormalize W every ~100 ticks if needed

---

## §11 Final Contract

**With gravitational backreaction:**

```
Tautology holds if:

1. H_t includes κ, λ, α, E_target (all protocol-frozen)
2. Backreaction term B_k = −α(‖Z‖² − E_target)·Z_k computed identically
3. Ghost rebirth threshold and scale are identical across peers
4. Norm ‖Z‖² remains bounded: ‖Z‖² ∈ [0.5·E_target, 2·E_target] (typical)
5. Power-rail throttling does NOT affect H_t (only local λ_actual, α_actual)
6. All languages (Rust, Swift, etc.) produce bit-identical Z_t given same μ_t
```

---

## References

- **DVSM-v3 implementation:** `/rust/base/src/lib.rs` (provided above)
- **Core architecture:** `CORE_ARCHITECTURE.md` § Operator Pipeline
- **DFE integration:** `DFE_INTEGRATION_SPEC.md` § Transform Layer
