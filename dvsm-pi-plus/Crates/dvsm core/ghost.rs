// src/ghost.rs — classification (diagnostic only, never branches core)
use crate::constants::*;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ghost {
    Nominal=0, Collapse=1, Diffuse=2, Echo=3,
    Burst=4, Trap=5, Vacuum=6, Denatured=7,
}

pub fn classify(
    stress: f32, novelty: f32, drift: f32, entropy: f32, omega_ratio: f32,
    killed: bool, denatured_ramp: bool,
) -> Ghost {
    if killed                                   { Ghost::Vacuum }
    else if denatured_ramp                      { Ghost::Denatured }
    else if stress > 1.5                        { Ghost::Burst }
    else if novelty < EPS && entropy < 0.1      { Ghost::Collapse }
    else if novelty > 0.9 && entropy > 2.0      { Ghost::Diffuse }
    else if entropy < 0.3 && stress < 0.1       { Ghost::Echo }
    else if omega_ratio > 1.0 || drift > 0.01   { Ghost::Trap }
    else                                        { Ghost::Nominal }
}
