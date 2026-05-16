[package]
name = "dvsm-core"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
description = "DVSM-π+++ Layer 0 — bare-metal spectral kernel"

[lib]
name = "dvsm_core"
crate-type = ["cdylib", "rlib", "staticlib"]
path = "src/lib.rs"

[features]
default = ["std"]
std = []                 # enables Box-based ABI init/free
no_std_alloc = []        # bare-metal with #[global_allocator]
deterministic = []       # locks RNG seeds for reproducibility
gaming = []              # enables velocity/omega (VR inertia path)
bio = []                 # enables denaturation/ghost-snap rebirth
rf = []                  # enables delta-trace compression
simd_hint = []           # adds #[target_feature] on projection

[dependencies]
# zero — no_std compatible
