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
