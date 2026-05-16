// Compiler flags for deployment:

RUSTFLAGS="-C target-cpu=native -C llvm-args=-force-vector-width=16 -C link-arg=-zrelro -C link-arg=-znow"
cargo build --release

STEP  EQUATION                                          MODULE
─────────────────────────────────────────────────────────────────
1.    ‖Z‖² > U²_MAX for K frames → kill                containment
      mode ← f(entropy, ‖S‖)                           containment
      Z,S,V,Ω ← rebirth(W, mode)                       containment

2.    c = WᵀZ                    O(nr)                  pipeline
      p = Wc                     O(nr)                  pipeline
      R = Z − p                  O(n)                   pipeline

3.    Z += dt·(Σⱼ(ZₖSⱼ−ZⱼSₖ)κₖⱼ − λZₖ)  O(r²)      pipeline

4.    S = αS + (1−α)Z            O(r)                   pipeline

5.    W += η·R⊗(c/‖c‖)          O(nr)                  pipeline
      if drift > 1e-6: MGS(W)   O(nr²)                 manifold
      sign_lock(W, W_prev)      O(nr)                  manifold

7.    V = clamp(V·γ + (R+S)·η)  O(n)                   pipeline
      X += V·dt                  O(n)                   pipeline

8.    Ω = (Ω + Z·α·dt)·decay    O(r)                   pipeline

9.    ghost = f(B,ν,δ,H,Ω/Z)    O(1)                   ghost

11.   emit if |Δν| > ε           O(1)                   trace

// Logical Step 10: State Rotation
state.prev_w.copy_from_slice(&state.w);
state.frame = state.frame.wrapping_add(1);

// src/abi.rs — C FFI boundary (stable, 5 functions)
use crate::core::DvsmCore;
use crate::trace::TraceFrame;
use crate::constants::*;

#[cfg(feature = "std")]
extern crate alloc;

#[no_mangle]
pub extern "C" fn dvsm_init(n: u32, r: u32) -> *mut DvsmCore {
    #[cfg(feature = "std")]
    {
        let mut c = Box::new(unsafe { core::mem::zeroed::<DvsmCore>() });
        c.n = n.min(R as u32) as u16;
        c.r = (r.min(n)).min(R as u32) as u16;
        c.alive = 1;
        c.frames_since_rebirth = u32::MAX;
        let rr = c.r as usize;
        let mut k = 0;
        while k < rr { c.w[k*R+k] = 1.0; k += 1; }
        c.w_prev = c.w;
        c.init_kappa();
        Box::into_raw(c)
    }
    #[cfg(not(feature = "std"))]
    { core::ptr::null_mut() } // bare-metal: use static instance
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_step(
    core: *mut DvsmCore, input: *const f32, len: u32, out: *mut TraceFrame,
) -> i32 {
    let c = match core.as_mut() { Some(c) => c, None => return -1 };
    let n = if (c.n as u32) < len { c.n as usize } else { len as usize };
    if input.is_null() || n == 0 { return -2; }
    let inp = core::slice::from_raw_parts(input, n);
    let tf = c.step(inp);
    if let Some(o) = out.as_mut() { *o = tf; }
    0
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_is_vacuum(core: *const DvsmCore) -> u8 {
    match core.as_ref() { Some(c) => (c.alive == 0) as u8, None => 1 }
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_get_trace(
    _c: *const DvsmCore, f: *const TraceFrame, o: *mut TraceFrame,
) -> i32 {
    match (f.as_ref(), o.as_mut()) { (Some(f), Some(o)) => { *o = *f; 0 }, _ => -1 }
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_free(core: *mut DvsmCore) {
    #[cfg(feature = "std")]
    if !core.is_null() { drop(Box::from_raw(core)); }
}
