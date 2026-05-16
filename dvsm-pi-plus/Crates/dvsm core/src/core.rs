// src/core.rs — state struct only, no logic
use crate::constants::*;
use crate::containment::RebirthMode;

#[repr(C, align(4096))]
pub struct DvsmCore {
    // hot (320 bytes = 5 cache lines)
    pub z: [f32; R], pub s: [f32; R], pub v: [f32; R],
    pub x: [f32; R], pub omega: [f32; R],
    // basis (2KB)
    pub w: [f32; R2], pub kappa: [f32; R2],
    // scratch (never crosses ABI)
    pub(crate) w_prev: [f32; R2],
    pub(crate) c: [f32; R], pub(crate) p: [f32; R], pub(crate) res: [f32; R],
    // scalars
    pub n: u16, pub r: u16, pub frame: u64, pub alive: u8,
    pub contain_fails: u8, pub rebirth_mode: RebirthMode,
    pub frames_since_rebirth: u32, pub(crate) prev_novelty: f32,
}
