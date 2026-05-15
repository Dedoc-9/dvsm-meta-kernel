{
  "dvsm_pi+++_math_summary": {
    "state_space": {
      "mu_t": "empirical measure over R^3 (particle layer)",
      "z_t": "rank-R spectral feature field (non-normal dynamics)",
      "s_t": "EMA memory / hysteresis state",
      "w_t": "Grassmann basis (orthonormal frame on Gr(R,D))"
    },

    "core_dynamics": {
      "particle_layer": "dX_i = b(X_i, μ_t) dt + sqrt(2T) dW_i",
      "mean_field_drift": "b(x, μ) = -∇_x E_full(x, μ, obs)",
      "energy_functional": "E_full = ||g-obs||^2 + α||g||^2 + (λ/N) Σ K(g, g_j)",
      "mcKean_vlasov_limit": "μ_N → μ_t as N → ∞ (propagation of chaos)"
    },

    "spectral_layer": {
      "lie_bracket_flow": "∂_t Z = [Z,S]_A - λZ",
      "antisymmetric_bracket": "[Z,S]_A = Σ (Z_i S_j - Z_j S_i) κ(i,j)",
      "kernel": "κ(i,j) = sin(i·1.37 - j·1.73)",
      "non_normal_effect": "transient growth despite dissipative eigenvalues",
      "memory_loop": "S_t = αS + (1-α)Z"
    },

    "geometry_layer": {
      "grassmann_flow": "W_k ← Normalize(A(W_k))",
      "gram_operator": "A(W)_k = Σ ⟨W_k, W_j⟩ W_j",
      "fixed_point": "A(W*) = W*",
      "orthogonality": "⟨W_i, W_j⟩ = δ_ij"
    },

    "unified_variational_form": {
      "free_energy": "F(μ,Z,W) = E_μ[φ_W(x)]·Z + ||Z-S||^2 + λ||Z||^2",
      "wasserstein_flow": "∂_t μ = -∇_{W2} F(μ)"
    },

    "operator_splitting": {
      "full_step": "T_τ = R_τ ∘ B_τ ∘ L_τ ∘ G_τ",
      "L_τ": "McKean–Vlasov diffusion step",
      "B_τ": "Feynman–Kac / Gibbs reweighting",
      "R_τ": "resampling projection (SMC noise)",
      "G_τ": "Grassmann geometric projection"
    },

    "stability_metrics": {
      "ess": "(Σ w_i)^2 / Σ w_i^2",
      "burst_metric": "B(t) = ||S_t|| / (||Z_t|| + ε)",
      "energy": "||Z||^2",
      "clt_fluctuation": "η_t^N = √N(μ̂_t - μ_t)",
      "resampling_noise": "M = quadratic variation of R_τ"
    },

    "gpu_execution_math": {
      "lie_bracket_discretization": "O(R^2) pairwise antisymmetric coupling",
      "ema_update": "S ← αS + (1-α)Z",
      "gain_clamp": "Z_i ← Z_i * min(1, U/|Z_i|)",
      "kill_switch": "if ||Z||^2 > U_max → Z = 0, atomic_flag = 1",
      "clt_reduction": "parallel sum over Z_i^2 (tree reduction)"
    },

    "manifold_projection": {
      "vr_embedding": "x_i = (i, Z_i, sin(Z_i + W_i))",
      "spectral_to_geometry": "Z → vertex displacement field",
      "basis_influence": "W modulates orientation / curvature"
    },

    "ghost_mode_math": {
      "definition": "transient non-normal amplification dominates damping",
      "condition": "||[Z,S]_A|| > λ||Z||",
      "aliasing_effect": "apparent instability at frame-rate scale",
      "reality": "stable eigenvalues, unstable transient dynamics"
    },

    "temporal_structure": {
      "frame_budget": "Δt = 1/240s",
      "decimation_rule": "W updated at 1/4 to 1/8 rate of Z",
      "gpu_constraint": "single-pass fused kernel required"
    }
  }
}
// ===============================================================
// DVSM-π+++ · STIEFEL-ANCHORED REBIRTH KERNEL (CONSISTENT FORM)
// ===============================================================

pub fn stiefel_rebirth_step(
    z: &mut [f64],
    s: &mut [f64],              // IMPORTANT: memory re-anchoring
    w_stiefel: &[f64],          // W ∈ St(R, D)
    ramp_val: f64,
    seed: f64
) {
    for i in 0..R {

        // ---------------------------------------------
        // 1. Stiefel projection (structured perturbation)
        // ---------------------------------------------
        let mut projection = 0.0;

        for j in 0..D {
            let w = w_stiefel[i * D + j];
            projection += w * gaussian_like(seed + j as f64);
        }

        // ---------------------------------------------
        // 2. Controlled spectral injection
        // (NOT raw overwrite — preserves Lie structure)
        // ---------------------------------------------
        let injected = ramp_val * NOISE_EPSILON * projection;

        // ---------------------------------------------
        // 3. Rebirth blending (critical stability step)
        // ---------------------------------------------
        let old_z = z[i];

        z[i] = 0.85 * old_z + 0.15 * injected;

        // ---------------------------------------------
        // 4. Memory anchoring (prevents "cold start ghost")
        // ---------------------------------------------
        s[i] = 0.95 * s[i] + 0.05 * z[i];
    }
}
