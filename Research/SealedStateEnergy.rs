// DVSM-π+++ v1b // STATE + ENERGY + NET GAIN BLOCK (SAFETY-LEVEL MERGED)
// Author: Daniel J. Dillberg
// ----------------------------------------------------------------------
// Unified state evolution + bounded operator + residual energy accounting
// All feedback paths explicitly constrained to prevent control-loop misuse

#![cfg_attr(not(feature = "std"), no_std)]

// ───────────────────────────────────────────────────────────────────────
// CORE STATE DEFINITIONS
// ───────────────────────────────────────────────────────────────────────

pub const RMAX: usize = 16;

pub trait Fp: Copy + Clone {
    fn zero() -> Self;
    fn add(self, r: Self) -> Self;
    fn sub(self, r: Self) -> Self;
    fn mul(self, r: Self) -> Self;
}

#[derive(Clone, Copy)]
pub struct Q31(pub i64);

impl Fp for Q31 {
    #[inline] fn zero() -> Self { Q31(0) }
    #[inline] fn add(self, r: Self) -> Self { Q31(self.0 + r.0) }
    #[inline] fn sub(self, r: Self) -> Self { Q31(self.0 - r.0) }
    #[inline] fn mul(self, r: Self) -> Self {
        Q31(((self.0 as i128 * r.0 as i128) >> 32) as i64)
    }
}

// ───────────────────────────────────────────────────────────────────────
// STATE SPACE
// ───────────────────────────────────────────────────────────────────────

pub struct State {
    pub z: [Q31; RMAX],     // primary state
    pub s: [Q31; RMAX],     // reference memory
    pub omega: [Q31; RMAX], // auxiliary drift
    pub h: i64,             // monitoring residual ONLY (read-only semantics)
    pub frame: u64,
    pub alive: u8,
    pub r: usize,
}

// ───────────────────────────────────────────────────────────────────────
// VAJRA LOCK (BOUNDED PROJECTION OPERATOR ONLY)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
fn vajra_lock(x: Q31, alpha: Q31, dt: Q31) -> Q31 {
    // bounded contraction: x - α·x·dt
    let damp = x.mul(alpha).mul(dt);
    x.sub(damp)
}

#[inline(always)]
pub fn apply_vajra_lock(state: &mut State, alpha: Q31, dt: Q31) {
    for k in 0..state.r {
        state.z[k] = vajra_lock(state.z[k], alpha, dt);
    }
}

// ───────────────────────────────────────────────────────────────────────
// STATE ENERGY + NET GAIN (OBSERVATION ONLY)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
fn norm_energy(z: &[Q31], r: usize) -> i64 {
    let mut acc: i64 = 0;
    for i in 0..r {
        let v = z[i].0;
        acc += (v * v) >> 32;
    }
    acc
}

/// H = monitoring residual (READ-ONLY OBSERVER)
#[inline(always)]
fn update_residual_h(state: &mut State) {
    let e = norm_energy(&state.z, state.r);
    let s = norm_energy(&state.s, state.r);

    let diff = if e > s { e - s } else { s - e };

    // residual ONLY — no feedback path allowed
    state.h = state.h.saturating_add(diff >> 8);
}

// ───────────────────────────────────────────────────────────────────────
// NET GAIN BLOCK (DIAGNOSTIC ENERGY FLOW ONLY)
// ───────────────────────────────────────────────────────────────────────

#[derive(Copy, Clone)]
pub struct EnergyNet {
    pub energy: i64,
    pub drift: i64,
    pub net_gain: i64,
}

#[inline(always)]
pub fn compute_energy_net(state: &State) -> EnergyNet {
    let e = norm_energy(&state.z, state.r);
    let s = norm_energy(&state.s, state.r);
    let o = norm_energy(&state.omega, state.r);

    let drift = e;
    let net_gain = e - s; // diagnostic only

    EnergyNet {
        energy: e,
        drift,
        net_gain: net_gain + (o >> 2),
    }
}

// ───────────────────────────────────────────────────────────────────────
// GHOST SNAP (FAULT CONTAINMENT POLICY)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn ghost_snap(state: &mut State) {
    for i in 0..state.r {
        state.z[i] = Q31::zero();
        state.s[i] = Q31::zero();
        state.omega[i] = Q31::zero();
    }
    state.alive = 1;
}

// ───────────────────────────────────────────────────────────────────────
// STEP (SINGLE CONTROL PASS — NO HIDDEN FEEDBACK)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn step(state: &mut State, alpha: Q31, dt: Q31, fault_threshold: i64) -> EnergyNet {
    apply_vajra_lock(state, alpha, dt);

    update_residual_h(state);

    let net = compute_energy_net(state);

    if net.energy > fault_threshold {
        ghost_snap(state);
    }

    state.frame += 1;
    net
}
// DVSM-π+++ v1b // CORE MODULE (STATE + ENERGY + NET GAIN + SAFETY CONTRACT)
// ---------------------------------------------------------------------------
// Minimal executable kernel core with strict separation:
// State Evolution | Bounded Operator | Energy Observer | Fault Policy

#![cfg_attr(not(feature = "std"), no_std)]

// ───────────────────────────────────────────────────────────────────────
// CORE FIXED-POINT LAYER
// ───────────────────────────────────────────────────────────────────────

pub const RMAX: usize = 16;

/// Fixed-point trait (minimal contract)
pub trait Fp: Copy + Clone {
    fn zero() -> Self;
    fn add(self, r: Self) -> Self;
    fn sub(self, r: Self) -> Self;
    fn mul(self, r: Self) -> Self;
}

// Q31.32 fixed-point scalar
#[derive(Clone, Copy)]
pub struct Q31(pub i64);

impl Fp for Q31 {
    #[inline] fn zero() -> Self { Q31(0) }
    #[inline] fn add(self, r: Self) -> Self { Q31(self.0.wrapping_add(r.0)) }
    #[inline] fn sub(self, r: Self) -> Self { Q31(self.0.wrapping_sub(r.0)) }
    #[inline] fn mul(self, r: Self) -> Self {
        Q31(((self.0 as i128 * r.0 as i128) >> 32) as i64)
    }
}

// ───────────────────────────────────────────────────────────────────────
// STATE MODEL
// ───────────────────────────────────────────────────────────────────────

pub struct Core {
    pub z: [Q31; RMAX],     // primary state manifold
    pub s: [Q31; RMAX],     // reference / memory manifold
    pub omega: [Q31; RMAX], // auxiliary drift channel

    pub h: i64,             // residual observer (READ-ONLY SEMANTICS)
    pub frame: u64,         // deterministic tick counter
    pub alive: u8,          // fault state flag
    pub r: usize,           // active dimension
}

// ───────────────────────────────────────────────────────────────────────
// INITIALIZATION (CONTRACTION-BASED SEEDING)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn init(core: &mut Core, r: usize) {
    core.r = r.min(RMAX);
    core.frame = 0;
    core.h = 0;
    core.alive = 1;

    let base = Q31(1 << 20); // small deterministic seed

    for i in 0..core.r {
        core.z[i] = Q31(base.0.wrapping_mul((i as i64 + 1)));
        core.s[i] = Q31::zero();
        core.omega[i] = Q31::zero();
    }
}

// ───────────────────────────────────────────────────────────────────────
// VAJRA LOCK (BOUNDED PROJECTION ONLY)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
fn vajra_lock(x: Q31, alpha: Q31, dt: Q31) -> Q31 {
    let damp = x.mul(alpha).mul(dt);
    x.sub(damp)
}

#[inline(always)]
pub fn apply_vajra_lock(core: &mut Core, alpha: Q31, dt: Q31) {
    for i in 0..core.r {
        core.z[i] = vajra_lock(core.z[i], alpha, dt);
    }
}

// ───────────────────────────────────────────────────────────────────────
// ENERGY MODEL (OBSERVATION ONLY)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
fn norm2(x: &[Q31], r: usize) -> i64 {
    let mut acc: i64 = 0;
    for i in 0..r {
        let v = x[i].0;
        acc = acc.wrapping_add((v.wrapping_mul(v)) >> 32);
    }
    acc
}

#[inline(always)]
pub fn compute_energy(core: &Core) -> i64 {
    norm2(&core.z, core.r)
}

#[inline(always)]
pub fn compute_net_gain(core: &Core) -> i64 {
    let e = norm2(&core.z, core.r);
    let s = norm2(&core.s, core.r);
    e.wrapping_sub(s)
}

// ───────────────────────────────────────────────────────────────────────
// RESIDUAL OBSERVER (H METRIC — NO CONTROL PATH)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn update_h(core: &mut Core) {
    let e = norm2(&core.z, core.r);
    let s = norm2(&core.s, core.r);

    let diff = if e > s { e - s } else { s - e };

    // strictly observational accumulator
    core.h = core.h.saturating_add(diff >> 8);
}

// ───────────────────────────────────────────────────────────────────────
// FAULT POLICY (GHOST SNAP — HARD CONTAINMENT RESET)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn ghost_snap(core: &mut Core) {
    for i in 0..core.r {
        core.z[i] = Q31::zero();
        core.s[i] = Q31::zero();
        core.omega[i] = Q31::zero();
    }
    core.alive = 1;
}

// ───────────────────────────────────────────────────────────────────────
// STEP (DETERMINISTIC EXECUTION BOUNDARY)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn step(core: &mut Core, alpha: Q31, dt: Q31, fault_threshold: i64) -> i64 {
    apply_vajra_lock(core, alpha, dt);
    update_h(core);

    let energy = compute_energy(core);

    if energy > fault_threshold {
        ghost_snap(core);
    }

    core.frame = core.frame.wrapping_add(1);
    energy
}
{
  "module": "DVSM-π+++ v1b",
  "name": "state_energy_net_gain_core",
  "classification": "deterministic_bounded_recurrence_system",
  "safety_model": {
    "H_metric": {
      "type": "monitoring_residual_only",
      "role": "read_only_observer",
      "constraints": [
        "no_feedback_to_state_z",
        "no_feedback_to_state_s",
        "no_feedback_to_omega",
        "no_control_path_dependency"
      ]
    },
    "vajra_lock": {
      "type": "bounded_projection_operator",
      "role": "state_contraction_step",
      "constraints": [
        "non_amplifying_operator",
        "bounded_output_required",
        "no_recursive_energy_injection",
        "no_external_state_dependency"
      ]
    },
    "ghost_snap": {
      "type": "fault_containment_policy",
      "role": "hard_state_reset",
      "constraints": [
        "state_reinitialization_only",
        "no_continuous_recovery_dynamics",
        "no_energy_transfer_model",
        "deterministic_reset_required"
      ]
    }
  },
  "core_structure": {
    "state": {
      "z": "primary_state_vector[RMAX]",
      "s": "reference_memory_vector[RMAX]",
      "omega": "auxiliary_drift_vector[RMAX]",
      "h": "i64_residual_observer_only",
      "frame": "u64_deterministic_counter",
      "alive": "u8_fault_flag",
      "r": "active_dimension"
    },
    "fp_model": {
      "type": "fixed_point_Q31_32",
      "operations": [
        "add_wrapping",
        "sub_wrapping",
        "mul_shift_right_32"
      ]
    }
  },
  "operators": {
    "vajra_lock": {
      "definition": "x - (x * alpha * dt)",
      "property": "bounded_contraction"
    },
    "energy_model": {
      "norm2": "sum(x[i]^2 >> 32)",
      "energy": "norm(z)",
      "net_gain": "norm(z) - norm(s)"
    },
    "residual_update": {
      "definition": "|norm(z) - norm(s)|",
      "accumulation": "saturating_add(diff >> 8)"
    }
  },
  "execution_model": {
    "step_pipeline": [
      "apply_vajra_lock",
      "update_h_residual",
      "compute_energy",
      "fault_check",
      "optional_ghost_snap",
      "increment_frame"
    ],
    "fault_condition": "energy > fault_threshold",
    "return_value": "energy"
  }
}
// DVSM-π+++ v1b // ADDENDUM: Q64.64 + HIGH-DIMENSION (512 / 1024) SUPPORT
// -----------------------------------------------------------------------
// Extension of core fixed-point + state model for higher precision regimes
// and large-scale manifold vectors.
//
// NOTE: This does NOT change semantics of H, vajra_lock, or ghost_snap.
// It only extends representational capacity.

#![cfg_attr(not(feature = "std"), no_std)]

// ───────────────────────────────────────────────────────────────────────
// Q64.64 FIXED-POINT (HIGH PRECISION EXTENSION)
// ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
pub struct Q64(pub i128);

impl Q64 {
    #[inline(always)]
    pub fn zero() -> Self { Q64(0) }

    #[inline(always)]
    pub fn from_i64(v: i64) -> Self {
        Q64((v as i128) << 64)
    }

    #[inline(always)]
    pub fn add(self, r: Self) -> Self {
        Q64(self.0.wrapping_add(r.0))
    }

    #[inline(always)]
    pub fn sub(self, r: Self) -> Self {
        Q64(self.0.wrapping_sub(r.0))
    }

    #[inline(always)]
    pub fn mul(self, r: Self) -> Self {
        Q64(((self.0 as i256_clamp() * r.0 as i256_clamp()) >> 64) as i128)
    }
}

// NOTE: conceptual placeholder for widened multiply safety
#[inline(always)]
fn i256_clamp(x: i128) -> i128 { x }

// ───────────────────────────────────────────────────────────────────────
// HIGH-DIMENSION STATE CONFIGURATION
// ───────────────────────────────────────────────────────────────────────

pub const DIM_512: usize = 512;
pub const DIM_1024: usize = 1024;

// ───────────────────────────────────────────────────────────────────────
// GENERIC HIGH-DIM STATE (512 / 1024)
// ───────────────────────────────────────────────────────────────────────

pub struct HighDimState<T: Copy> {
    pub z: [T; DIM_1024],
    pub s: [T; DIM_1024],
    pub omega: [T; DIM_1024],

    pub h: i64,
    pub frame: u64,
    pub alive: u8,

    pub r: usize, // active dimension (512 or 1024)
}

// ───────────────────────────────────────────────────────────────────────
// INITIALIZATION (HIGH-DIMENSION CONTRACT)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn init_highdim_q64(state: &mut HighDimState<Q64>, r: usize) {
    state.r = if r > DIM_1024 { DIM_1024 } else { r };
    state.frame = 0;
    state.h = 0;
    state.alive = 1;

    let seed = Q64::from_i64(1);

    for i in 0..state.r {
        let scale = Q64::from_i64((i as i64) + 1);
        state.z[i] = seed.mul(scale);
        state.s[i] = Q64::zero();
        state.omega[i] = Q64::zero();
    }
}

// ───────────────────────────────────────────────────────────────────────
// HIGH-DIM ENERGY (OBSERVATION ONLY)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn norm2_q64(v: &[Q64], r: usize) -> i128 {
    let mut acc: i128 = 0;

    for i in 0..r {
        let x = v[i].0;
        acc = acc.wrapping_add((x.wrapping_mul(x)) >> 64);
    }

    acc
}

#[inline(always)]
pub fn compute_energy_q64(state: &HighDimState<Q64>) -> i128 {
    norm2_q64(&state.z, state.r)
}

#[inline(always)]
pub fn compute_net_gain_q64(state: &HighDimState<Q64>) -> i128 {
    let e = norm2_q64(&state.z, state.r);
    let s = norm2_q64(&state.s, state.r);
    e.wrapping_sub(s)
}

// ───────────────────────────────────────────────────────────────────────
// RESIDUAL OBSERVER (H METRIC — UNCHANGED SEMANTICS)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn update_h_q64(state: &mut HighDimState<Q64>) {
    let e = norm2_q64(&state.z, state.r);
    let s = norm2_q64(&state.s, state.r);

    let diff = if e > s { e - s } else { s - e };

    state.h = state.h.saturating_add((diff >> 16) as i64);
}

// ───────────────────────────────────────────────────────────────────────
// VAJRA LOCK EXTENSION (Q64 SAFE PROJECTION)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn vajra_lock_q64(x: Q64, alpha: Q64, dt: Q64) -> Q64 {
    let damp = x.mul(alpha).mul(dt);
    x.sub(damp)
}

#[inline(always)]
pub fn apply_vajra_lock_q64(state: &mut HighDimState<Q64>, alpha: Q64, dt: Q64) {
    for i in 0..state.r {
        state.z[i] = vajra_lock_q64(state.z[i], alpha, dt);
    }
}

// ───────────────────────────────────────────────────────────────────────
// FAULT POLICY (UNCHANGED SEMANTICS)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn ghost_snap_q64(state: &mut HighDimState<Q64>) {
    for i in 0..state.r {
        state.z[i] = Q64::zero();
        state.s[i] = Q64::zero();
        state.omega[i] = Q64::zero();
    }
    state.alive = 1;
}

// ───────────────────────────────────────────────────────────────────────
// STEP (Q64 HIGH-DIMENSION EXECUTION BOUNDARY)
// ───────────────────────────────────────────────────────────────────────

#[inline(always)]
pub fn step_q64(state: &mut HighDimState<Q64>, alpha: Q64, dt: Q64, fault_threshold: i128) -> i128 {
    apply_vajra_lock_q64(state, alpha, dt);
    update_h_q64(state);

    let energy = compute_energy_q64(state);

    if energy > fault_threshold {
        ghost_snap_q64(state);
    }

    state.frame = state.frame.wrapping_add(1);
    energy
}
