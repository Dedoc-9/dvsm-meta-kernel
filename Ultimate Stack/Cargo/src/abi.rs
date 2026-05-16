//! ============================================================
//! DVSM-π+++ / DQSDv2 · ABI LAYER (FINAL)
//! ============================================================
//! PURPOSE:
//! - Stable FFI boundary for UE5 / C / external engines
//! - Zero physics logic (pure interface layer)
//! - Ownership + safety enforcement
//! ============================================================

use core::{ptr, slice};

use crate::{
    CoreState,
    DVSM_Params,
    DVSM_TraceFrame,
    DVSM_Handle,
    DVSM_NOMINAL,
    RMAX,
};

// ============================================================
// SAFETY CONTRACT
// ============================================================
//
// RULES:
// - no allocation here
// - no math here
// - no branching physics logic here
// - only state forwarding
//
// ============================================================

// ============================================================
// INIT
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dvsm_init(
    params: *const DVSM_Params
) -> *mut DVSM_Handle {

    if params.is_null() {
        return ptr::null_mut();
    }

    let p = *params;

    if p.r == 0 || p.r as usize > RMAX {
        return ptr::null_mut();
    }

    let mut state = Box::new(CoreState {
        params: p,
        frame: 0,

        W: [0.0; RMAX * RMAX],
        W_prev: [0.0; RMAX * RMAX],

        Z: [0.0; RMAX],
        S: [0.0; RMAX],
        V: [0.0; RMAX],
        Omega: [0.0; RMAX],

        ghost: DVSM_NOMINAL,
        contained: true,
        fail_counter: 0,
    });

    // identity initialization (deterministic basis seed)
    let r = p.r as usize;
    let mut i = 0;
    while i < r {
        state.W[i * RMAX + i] = 1.0;
        state.W_prev[i * RMAX + i] = 1.0;
        i += 1;
    }

    Box::into_raw(state) as *mut DVSM_Handle
}

// ============================================================
// STEP
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dvsm_step(
    handle: *mut DVSM_Handle,
    input: *const f32,
    trace_out: *mut DVSM_TraceFrame,
) -> i32 {

    if handle.is_null() || input.is_null() {
        return -1;
    }

    let state = &mut *(handle as *mut CoreState);
    let r = state.params.r as usize;

    let input_slice = slice::from_raw_parts(input, r);

    let mut trace = DVSM_TraceFrame {
        frame: 0,
        stress: 0.0,
        novelty: 0.0,
        drift: 0.0,
        entropy: 0.0,
        energy: 0.0,
        ghost: DVSM_NOMINAL,
        contained: 0,
    };

    // delegated execution (NO LOGIC HERE)
    crate::core_step(state, input_slice, &mut trace);

    if !trace_out.is_null() {
        *trace_out = trace;
    }

    0
}

// ============================================================
// RECALIBRATION
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dvsm_recalibrate(
    handle: *mut DVSM_Handle
) -> i32 {

    if handle.is_null() {
        return -1;
    }

    let state = &mut *(handle as *mut CoreState);

    crate::maintain_manifold(state, state.params.r as usize);

    0
}

// ============================================================
// VACUUM CHECK
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dvsm_is_vacuum(
    handle: *const DVSM_Handle
) -> u8 {

    if handle.is_null() {
        return 1;
    }

    let state = &*(handle as *const CoreState);

    (state.ghost == crate::DVSM_VACUUM) as u8
}

// ============================================================
// FREE
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dvsm_free(
    handle: *mut DVSM_Handle
) {

    if !handle.is_null() {
        let _ = Box::from_raw(handle as *mut CoreState);
    }
}
