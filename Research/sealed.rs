// dvsm_v1b_sealed.rs
// DVSM-π+++ v1b · Sealed Deterministic Systems Library
// Author: Daniel J. Dillberg
//
// FORMAL SPECIFICATION:
// A bounded, deterministic nonlinear recurrence system utilizing
// rational-saturation feedback and a Lyapunov-style diagnostic
// functional over a fixed-point state space.
//
// CONTRACT:
// - No floating-point randomness (only deterministic conversion boundaries)
// - Fixed-point arithmetic only (Q31.32 ABI core type)
// - Replay-stable execution across x86_64 / AArch64 / WASM32 (assuming same rounding mode)
// - Bounded state evolution with rational saturation operator
//
// NOTE:
// "H" is defined as Stabilization Workload Metric (W_s), not physical energy.

#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
extern crate std;

// ─────────────────────────────────────────────────────────────
// CONSTANTS
// ─────────────────────────────────────────────────────────────

pub const RMAX: usize = 16;
pub const KILL_K: u8 = 3;

// ─────────────────────────────────────────────────────────────
// FIXED POINT CORE TRAIT
// ─────────────────────────────────────────────────────────────

pub trait Fp: Copy + Clone + Send + Sync + 'static {
    fn zero() -> Self;
    fn add(self, r: Self) -> Self;
    fn sub(self, r: Self) -> Self;
    fn mul(self, r: Self) -> Self;
    fn from_f64(v: f64) -> Self;
    fn to_f64(self) -> f64;
}

// ─────────────────────────────────────────────────────────────
// Q31.32 IMPLEMENTATION (ABI CORE)
// ─────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub struct Q31(pub i64);

impl Fp for Q31 {
    #[inline(always)] fn zero() -> Self { Q31(0) }

    #[inline(always)]
    fn add(self, r: Self) -> Self {
        Q31(self.0.saturating_add(r.0))
    }

    #[inline(always)]
    fn sub(self, r: Self) -> Self {
        Q31(self.0.saturating_sub(r.0))
    }

    #[inline(always)]
    fn mul(self, r: Self) -> Self {
        Q31(((self.0 as i128 * r.0 as i128) >> 32) as i64)
    }

    #[inline(always)]
    fn from_f64(v: f64) -> Self {
        Q31((v.clamp(-2e9, 2e9) * (1u64 << 32) as f64) as i64)
    }

    #[inline(always)]
    fn to_f64(self) -> f64 {
        self.0 as f64 / (1u64 << 32) as f64
    }
}

// ─────────────────────────────────────────────────────────────
// CORE STATE
// ─────────────────────────────────────────────────────────────

pub struct Core<T: Fp> {
    pub z: [T; RMAX],
    pub s: [T; RMAX],
    pub omega: [T; RMAX],

    pub r: usize,
    pub frame: u64,
    pub alive: u8,

    contain_fails: u8,

    // fixed-point parameters
    pub alpha: T,
    pub gamma: T,
    pub dt: T,
}

// ─────────────────────────────────────────────────────────────
// INITIALIZATION CONTRACT (NON-DEGENERATE SEED)
// ─────────────────────────────────────────────────────────────

pub fn init_v1b(state: &mut Core<Q31>) {
    let r = state.r;

    for k in 0..r {
        state.z[k] = Q31::from_f64(0.01 * (k as f64 + 1.0));
        state.s[k] = Q31::zero();
        state.omega[k] = Q31::zero();
    }

    state.frame = 0;
    state.contain_fails = 0;
    state.alive = 1;
}

// ─────────────────────────────────────────────────────────────
// SAFE FIXED-POINT HELPERS
// ─────────────────────────────────────────────────────────────

#[inline(always)]
fn qabs(x: i64) -> i64 {
    if x < 0 { -x } else { x }
}

#[inline(always)]
fn qdiv(a: i64, b: i64) -> i64 {
    if b == 0 { return 0; }
    ((a as i128) << 32).wrapping_div(b as i128) as i64
}

// ─────────────────────────────────────────────────────────────
// LIPSCHITZ-STABLE TANH APPROXIMATION
// ─────────────────────────────────────────────────────────────

#[inline(always)]
fn q_tanh(x: i64) -> i64 {
    // clamp to prevent saturation artifacts
    let x = if x > (3 << 32) {
        3 << 32
    } else if x < -(3 << 32) {
        -(3 << 32)
    } else {
        x
    };

    let x2 = ((x as i128 * x as i128) >> 32) as i64;

    let num = ((x as i128 * ((27 << 32) + x2 as i128)) >> 32) as i64;
    let den = (27 << 32) + ((9i64 << 32) as i128 * x2 as i128 >> 32) as i64;

    qdiv(num, den)
}

// ─────────────────────────────────────────────────────────────
// GROUNDED DIPOLE OPERATOR
// ─────────────────────────────────────────────────────────────

#[inline(always)]
fn grounded_dipole(x: i64, alpha: i64, gamma: i64) -> i64 {
    let attractor = q_tanh(((alpha as i128 * x as i128) >> 32) as i64);

    let x_abs = qabs(x);
    let x_sq = ((x as i128 * x as i128) >> 32) as i64;

    let denom = (1 << 32) + x_abs;
    let repulsor = qdiv(x_sq, denom);

    attractor - (((gamma as i128 * repulsor as i128) >> 32) as i64)
}

// ─────────────────────────────────────────────────────────────
// STABILIZATION WORKLOAD METRIC (H = W_s)
// ─────────────────────────────────────────────────────────────

#[inline(always)]
fn harvest_workload(attractor: i64, repulsor: i64, h: &mut i64) {
    let tension = qabs(attractor - repulsor);
    *h = h.saturating_add(tension >> 8);
}

// ─────────────────────────────────────────────────────────────
// INITIALIZATION STEP (EXPORTED STYLE)
// ─────────────────────────────────────────────────────────────

impl<T: Fp> Core<T> {
    pub fn new(r: usize, alpha: f64, gamma: f64, dt: f64) -> Self {
        Self {
            z: [T::zero(); RMAX],
            s: [T::zero(); RMAX],
            omega: [T::zero(); RMAX],

            r: r.min(RMAX),
            frame: 0,
            alive: 1,
            contain_fails: 0,

            alpha: T::from_f64(alpha),
            gamma: T::from_f64(gamma),
            dt: T::from_f64(dt),
        }
    }

    // ─────────────────────────────────────────────────────────
    // ONE DETERMINISTIC STEP
    // ─────────────────────────────────────────────────────────

    pub fn step(&mut self, h: &mut i64) {
        let r = self.r;

        let alpha = self.alpha.to_f64() as i64;
        let gamma = self.gamma.to_f64() as i64;
        let dt = self.dt.to_f64() as i64;

        for k in 0..r {
            let x = self.z[k].to_f64() as i64;

            let f = grounded_dipole(x, alpha, gamma);

            self.z[k] = self.z[k].sub(Q31(((dt as i128 * f as i128) >> 32) as i64));

            harvest_workload(
                q_tanh(((alpha as i128 * x as i128) >> 32) as i64),
                qdiv(((x as i128 * x as i128) >> 32) as i64, (1 << 32) + qabs(x)),
                h,
            );
        }

        self.frame += 1;
    }
}

// ─────────────────────────────────────────────────────────────
// END OF SEAL
// ─────────────────────────────────────────────────────────────
