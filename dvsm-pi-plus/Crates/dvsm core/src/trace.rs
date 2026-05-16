// src/trace.rs — unified ABI frame + delta compression
use crate::constants::*;

#[repr(C, align(32))]
#[derive(Clone, Copy)]
pub struct TraceFrame {
    pub frame: u64, pub stress: f32, pub novelty: f32,
    pub drift: f32, pub entropy: f32, pub energy: f32,
    pub omega_norm: f32, pub ghost: u8, pub contained: u8,
    pub emitted: u8, _pad: u8,
}

impl TraceFrame {
    pub const ZERO: Self = Self {
        frame:0, stress:0.0, novelty:0.0, drift:0.0, entropy:0.0,
        energy:0.0, omega_norm:0.0, ghost:0, contained:0, emitted:0, _pad:0,
    };
}

#[inline]
pub fn should_emit(novelty: f32, prev: f32, killed: bool, frame: u64) -> bool {
    let d = novelty - prev;
    d > TRACE_DELTA_EPS || d < -TRACE_DELTA_EPS || killed || frame < 2
}
