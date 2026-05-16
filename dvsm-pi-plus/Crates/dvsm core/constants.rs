// src/constants.rs — all tuning parameters in one place
#![allow(dead_code)]

pub const R: usize = 16;
pub const R2: usize = R * R;
pub const DT: f32 = 4.166_667e-3;       // 1/240
pub const ALPHA: f32 = 0.98;
pub const LAMBDA: f32 = 0.05;
pub const ETA: f32 = 0.01;
pub const DAMPING: f32 = 0.98;
pub const U_MAX: f32 = 100.0;
pub const U_MAX2: f32 = U_MAX * U_MAX;
pub const EPS: f32 = 1e-8;
pub const OMEGA_DECAY: f32 = 0.999;
pub const KILL_K: u8 = 3;
pub const RAMP_FRAMES: u32 = 120;
pub const TRACE_DELTA_EPS: f32 = 1e-4;
pub const KAPPA_A: f32 = 1.37;          // κ frequency pair (coprime ratio)
pub const KAPPA_B: f32 = 1.73;
