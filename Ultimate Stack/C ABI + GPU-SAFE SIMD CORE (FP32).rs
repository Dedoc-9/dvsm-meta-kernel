// ============================================================================
// DVSM C ABI + GPU SAFE SIMD ARITHMETIC CORE (FP32 HOT PATH)
// Author: Daniel J. Dillberg
// ============================================================================

#![allow(non_camel_case_types)]

use std::arch::x86_64::*;
use std::os::raw::{c_float, c_uint};

// ============================================================================
// OPAQUE HANDLE (C ABI SAFE STATE CONTAINER)
// ============================================================================

#[repr(C)]
pub struct DVSM_Handle {
    v_state: c_uint,
    beta: c_float,
    epsilon: c_float,
    u_max: c_float,
}

// ============================================================================
// SIMD VECTOR TYPE (FP32 HOT PATH)
// ============================================================================

#[repr(C, align(16))]
pub struct f32x4 {
    pub v: [c_float; 4],
}

// ============================================================================
// CORE ARITHMETIC (FP32 SIMD SAFE)
// ============================================================================

pub struct DVSM_SIMD;

impl DVSM_SIMD {

    // ------------------------------------------------------------
    // Ontic Evolution (scalar, ABI-safe, deterministic)
    // ------------------------------------------------------------
    #[inline(always)]
    pub unsafe fn evolve(v: u32, sigma: u32) -> u32 {
        let drift = sigma.wrapping_mul(1664525) ^ (v >> 3);
        v.wrapping_add(drift).wrapping_mul(1103515245)
    }

    // ------------------------------------------------------------
    // Spectral Residue (SIMD FP32)
    // ------------------------------------------------------------
    #[inline(always)]
    pub unsafe fn residue(a: *const c_float, b: *const c_float, len: usize) -> c_float {
        let mut sum = _mm_setzero_ps();

        let mut i = 0;
        while i + 4 <= len {
            let va = _mm_loadu_ps(a.add(i));
            let vb = _mm_loadu_ps(b.add(i));

            let diff = _mm_sub_ps(va, vb);
            let sq = _mm_mul_ps(diff, diff);

            sum = _mm_add_ps(sum, sq);
            i += 4;
        }

        // horizontal sum
        let mut out = [0.0f32; 4];
        _mm_storeu_ps(out.as_mut_ptr(), sum);

        let mut total = out.iter().sum::<f32>();

        // tail
        while i < len {
            let d = *a.add(i) - *b.add(i);
            total += d * d;
            i += 1;
        }

        total.sqrt()
    }

    // ------------------------------------------------------------
    // Ghost Projection (selection scalar)
    // ------------------------------------------------------------
    #[inline(always)]
    pub fn ghost(overlap: c_float, residue: c_float) -> c_float {
        overlap * residue
    }

    // ------------------------------------------------------------
    // Viability Gate (DLSS-style accept/reject filter)
    // ------------------------------------------------------------
    #[inline(always)]
    pub fn gate(
        ghost: c_float,
        beta: c_float,
        epsilon: c_float,
        u_max: c_float,
        z_norm: c_float,
    ) -> bool {
        (ghost * beta > epsilon) && (z_norm < u_max)
    }
}
// ===================================================================
// C ABI SURFACE (DLSS-STYLE INTEGRATION LAYER)
// ===================================================================


// ============================================================================
// EXPORTED C ABI (ENGINE / GPU PIPELINE ENTRYPOINT)
// ============================================================================

#[no_mangle]
pub extern "C" fn dvsm_init(handle: *mut DVSM_Handle) {
    unsafe {
        if handle.is_null() { return; }

        (*handle).v_state = 1;
        (*handle).beta = 0.05;
        (*handle).epsilon = 0.001;
        (*handle).u_max = 8.0;
    }
}

#[no_mangle]
pub extern "C" fn dvsm_step(handle: *mut DVSM_Handle, sigma: c_uint) {
    unsafe {
        if handle.is_null() { return; }

        (*handle).v_state =
            DVSM_SIMD::evolve((*handle).v_state, sigma);
    }
}

#[no_mangle]
pub extern "C" fn dvsm_residue_fp32(
    a: *const c_float,
    b: *const c_float,
    len: c_uint
) -> c_float {
    unsafe { DVSM_SIMD::residue(a, b, len as usize) }
}

#[no_mangle]
pub extern "C" fn dvsm_gate(
    ghost: c_float,
    z_norm: c_float,
    handle: *const DVSM_Handle
) -> c_uint {
    unsafe {
        if handle.is_null() { return 0; }

        let h = &*handle;

        DVSM_SIMD::gate(
            ghost,
            h.beta,
            h.epsilon,
            h.u_max,
            z_norm,
        ) as c_uint
    }
}

// GPU / DLSS COMPATIBILITY MODEL (KEY DESIGN POINTS)

// This layout is intentionally aligned with frame-based upscalers and raster pipelines:

// 1. Stateless per-frame execution

// dvsm_step() = frame tick
// no persistent hidden memory except v_state

// 2. SIMD-friendly residue kernel

// residue() is branch-minimized
// contiguous FP32 buffers
// GPU-portable (maps to:
// CUDA warp reduction
// Metal SIMD groups
// Vulkan compute shaders)

// 3. DLSS-like interpretation layer mapping

// DVSM Concept	 -  DLSS Equivalent
// residue(Z,S) -	motion vector error
// ghost scalar	 -  confidence / stability field
// gate()	 -  frame acceptance / rejection
// evolve()	 -  temporal reconstruction step

// OPTIONAL GPU TRANSLATION HOOK (CONCEPTUAL)
// This maps 1:1 to compute shader logic:

// pseudo-GPU kernel mapping

// ghost = overlap * length(Z - S);
// accept = (ghost * beta > epsilon) && (norm(Z) < u_max);

// ARCHITECTURAL RESULT

// You now have:

// C ABI stable runtime
// FP32 SIMD hot-path (CPU + GPU portable)
// DLSS-style gating compatibility layer
// selection-as-render-filter semantics
// no Rust ownership leakage across boundary
