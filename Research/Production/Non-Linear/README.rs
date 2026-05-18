# DVSM-π+++ / DQSDv2

**Deterministic Spectral Arbitration Kernel**

A bounded nonlinear recurrence engine with indexed antisymmetric
Lie-bracket coupling, exponential memory, and optional nonlinear
operators. Fixed-point arithmetic (Q16/Q31/Q64) for cross-platform
deterministic replay. Zero heap allocation. ABI-stable binary output.

Author: Daniel J. Dillberg · License: ALGP-3

---

## Core Equation

```
Z_k += dt · (Σ_j (Z_k·S_j − Z_j·S_k) · κ_{kj} − λ·Z_k)

d‖Z‖²/dt = −2λ‖Z‖²   (κ antisymmetric → coupling is energy-neutral)
```

---

## Repository Layout

```
dvsm-pi-plus/
├── Cargo.toml                    workspace root
├── README.md                     this file
├── include/
│   ├── dvsm.h                   C ABI header
│   └── dvsm_v2.h                tightened header (unified TraceFrame)
├── shaders/
│   └── dvsm_gpu.wgsl            GPU compute contract (3 kernels)
├── crates/
│   ├── dvsm-core/
│   │   ├── Cargo.toml            [cdylib+rlib+staticlib, no deps]
│   │   └── src/
│   │       ├── lib.rs            crate root + feature gates
│   │       ├── constants.rs      all tunables
│   │       ├── math.rs           pipeline primitives (project, lie_step, ema, etc.)
│   │       ├── manifold.rs       MGS, drift, sign_lock
│   │       ├── core.rs           DvsmCore struct
│   │       ├── pipeline.rs       11-stage step orchestration
│   │       ├── containment.rs    kill/rebirth/denaturation
│   │       ├── ghost.rs          8-variant classifier
│   │       ├── trace.rs          TraceFrame + delta gate
│   │       └── abi.rs            5 stable C FFI functions
│   ├── dvsm-gpu/  Cargo.toml     [cdylib, wgpu]
│   ├── dvsm-rf/   Cargo.toml     [cdylib+rlib]
│   ├── dvsm-bio/  Cargo.toml     [cdylib+rlib]
│   └── dvsm-gaming/ Cargo.toml   [cdylib, UE5/Unity]
├── standalone/
│   ├── dvsm_v20_final.rs         PRIMARY: all Q backends, full pipeline, binary ABI
│   ├── dvsm_one_file.rs          consolidated kernel + SDK block + hooks map
│   ├── dvsm_attractor_tracker.rs phase-space observer + online PCA + Lyapunov proxy
│   ├── v17r_render.rs            V17-R: Z → RGB/depth/curvature (6 render modes)
│   ├── allyx_profiler.rs         ROG Ally X benchmark (actual pipeline)
│   ├── dvsm_core_baremetal.rs    no_std, no libm, all math approximated
│   ├── dvsm_core_final.rs        denaturation-aware containment
│   ├── dvsm_v20_onefile.rs       V20 + polar constraint + vault export
│   └── v20_terminal_spec.rs      5 theorems, ISS class, ABI layout, legal
├── specs/
│   ├── dvsm_dfe_review.json      DFE arithmetic audit
│   ├── rp1_flaws_debate.json     RP1 10-flaw debate
│   ├── a10_review.json           A10 3 critical errors + fixes
│   ├── dfe_math_core_review.json math spec review
│   ├── dfe_integration_deepdive.json  7-path integration architecture
│   ├── dvsm_kernel_hardened.json hardened system spec
│   └── dvsm_bioscience_gameplan.json  5-route bioscience debate
└── docs/
    ├── dvsm_whitepaper.html      full technical whitepaper
    ├── dvsm_theory_deepdive.md   MIT-level analysis
    ├── engine_deepdive.md        V2.2 + acoustic + V1-V16
    └── rf_refinements_review.md  15 RF refinements ranked
```

---

## Pipeline (11 stages)

```
 1. CONTAINMENT      ‖Z‖²>U²_MAX for K frames → kill/rebirth
 2. PROJECTION       c=WᵀZ; p=Wc; R=Z−p (two-stage, correct)
 3. LIE EVOLUTION    Z += dt·([Z,S]_κ − λZ)
 4. EMA MEMORY       S = αS + (1−α)Z (frozen during containment)
 5. BASIS ADAPT      W += η·R⊗(c/‖c‖) (weighted per-column)
 6. MANIFOLD         MGS orthonormalize + sign_lock(W, W_prev)
 7. VELOCITY+OMEGA   V=clamp(V·γ+(R+S)·η); X+=V·dt; X*=(1−damp·dt)
                     Ω=(Ω+Z·α·dt)·decay (no Ω→V backfeed)
 8. CLASSIFY         ghost=f(B,ν,δ,H,‖Ω‖/‖Z‖)
 9. STATE COMMIT     W_prev←W; frame+=1 (AFTER all evolution)
10. STIFFNESS PROBE  K=|Δ‖Z‖²/Δε| (shadow, read-only)
11. EMIT             TraceFrame if |Δν|>ε (delta-encoded)
```

---

## Boundedness

| # | Result | Condition |
|---|--------|-----------|
| T1 | d‖Z‖²/dt = −2λ‖Z‖² | κ antisymmetric |
| T2 | ‖WᵀW−I‖ = O(ε_mach) | MGS per frame |
| T3 | ‖X‖ bounded | V clamped + X damped |
| T4 | ‖S‖ ≤ sup‖Z‖ | convex combination |
| T5 | ‖Ω‖ bounded | geometric series |

**ISS (Input-to-State Stable)** under bounded input.

---

## Fixed-Point Backends

| Format | Type | Range | Target |
|--------|------|-------|--------|
| Q16.16 | i32 | ±32K | WASM, embedded |
| Q31.32 | i64 | ±2G | PC, gaming, RF |
| Q64.64 | i128 | ±9.2E18 | archival, deep-space |

---

## Build

```bash
cargo build --release --features std
RUSTFLAGS="-C target-cpu=native" cargo build --release  # SIMD
cargo build --release --target wasm32-unknown-unknown    # WASM
```

---

## Domains

| Domain | Key Metric | Rebirth Mode |
|--------|------------|--------------|
| Gaming/VR | B(t) → DLSS gate | Structured |
| RF/SIGINT | B(t) + kurtosis | Structured |
| Deep Space | stiffness K | GhostSnap |
| Submarine VLF | B(t) + Ω drift | GhostSnap |
| Bioscience | entropy + transitions | HighEntropy |

---

## Research Priority

| Week | Route | Output |
|------|-------|--------|
| 1 | Energy audit + bifurcation scan | Validates T1, maps parameter space |
| 2 | Shadow dimensionality + Markov surprise | Validates PCA, flags anomalies |
| 3+ | Sectional curvature + MSF | Geometric cooperativity |
| 4+ | OP5: hemoglobin/PNMT FEL | Novel biophysics if validates |

---

## Novel Results

1. Energy conservation under antisymmetric Lie-bracket + EMA
2. Tangent projection redundancy theorem
3. B(t) as non-normal amplification proxy
4. GhostSnap rebirth preserving memory continuity
5. OP5: curvature-cooperativity (unvalidated)

---

## What This Is Not

NOT physics, NOT cryptographic, NOT manifold-preserving,
NOT infinitely stable, NOT quantum, NOT holographic.

Processes float arrays. Emits float arrays.
Everything else is interpretation.
