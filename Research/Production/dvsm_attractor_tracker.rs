// dvsm_attractor_tracker.rs
// DVSM-π+++ V20.4 · Phase-Space Trajectory Analysis Addendum
// Author: Daniel J. Dillberg ·
//
// ════════════════════════════════════════════════════════════════
// WHAT THIS DOES (real, grounded)
// ════════════════════════════════════════════════════════════════
//
// Tracks where the state Z(t) goes in its phase space over time.
// Detects:
//   · which regions of state space Z visits repeatedly (basins)
//   · when Z transitions between basins (attractor switching)
//   · recurrence patterns (periodic orbits, quasi-periodic, chaotic)
//   · trajectory divergence rate (largest Lyapunov exponent proxy)
//
// REAL USES:
//   RF:   detect frequency-hopping pattern by tracking Z trajectory
//         through spectral phase space. Each hop = basin transition.
//   Bio:  track protein conformational state. Each basin = a fold.
//         Transition event = allosteric switch or denaturation onset.
//   Game: state-space coverage analysis. How much of the dynamics
//         has the system explored? Useful for procedural generation.
//
// WHAT THIS DOES NOT DO:
//   ✗ Does not track filesystem locations
//   ✗ Does not determine where data is stored on disk
//   ✗ Does not interact with hardware storage
//   ✗ Rössler/rose curves here are ANALYSIS TOOLS, not storage maps
//
// ════════════════════════════════════════════════════════════════
// MATHEMATICAL FOUNDATION
// ════════════════════════════════════════════════════════════════
//
// The DVSM state Z(t) ∈ ℝ^R evolves under the Lie bracket:
//   dZ/dt = [Z,S]_κ − λZ
//
// This trajectory Z(0), Z(1), Z(2), ... traces a path through
// R-dimensional phase space. The path may:
//   · converge to a fixed point (Ghost::Collapse)
//   · orbit a limit cycle (Ghost::Echo)
//   · fill a strange attractor (chaotic regime)
//   · transition between basins (Ghost::Burst at transition)
//
// We discretize phase space into cells and track:
//   · visit count per cell (histogram)
//   · transition matrix between cells (Markov chain)
//   · recurrence time (frames between revisits)
//   · trajectory divergence (distance between Z and shadow copy)
//
// ════════════════════════════════════════════════════════════════

#![cfg_attr(not(feature = "std"), no_std)]

// re-use core constants
pub const RMAX: usize = 16;
pub const GRID: usize = 8;       // cells per dimension (total cells = GRID^2 for 2D projection)
pub const GRID2: usize = GRID * GRID;
pub const MAX_TRANSITIONS: usize = 256;
pub const RECURRENCE_WINDOW: usize = 64;

// ── Phase-space cell (2D projection of R-dimensional Z) ─────
// We project Z onto its two highest-energy modes for visualization.
// This is a Poincaré section, not the full phase space.

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Cell {
    pub ix: u8,  // grid x index
    pub iy: u8,  // grid y index
}

impl Cell {
    #[inline]
    pub fn id(&self) -> usize { self.ix as usize * GRID + self.iy as usize }
}

// ── Transition event (basin switching) ──────────────────────

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Transition {
    pub frame: u64,
    pub from: Cell,
    pub to: Cell,
    pub energy_at_transition: f64,
    pub stress_at_transition: f64,
}

// ── Attractor summary (what the trajectory looks like) ──────

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AttractorType {
    Unknown    = 0,
    FixedPoint = 1,  // Z converges: 1 cell dominates >90% of visits
    LimitCycle = 2,  // Z oscillates: 2-8 cells visited in repeating order
    Torus      = 3,  // quasi-periodic: many cells, low recurrence variance
    Strange    = 4,  // chaotic: many cells, high divergence rate
    Transient  = 5,  // hasn't settled yet (< RECURRENCE_WINDOW frames)
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AttractorReport {
    pub atype: AttractorType,
    pub cells_visited: u16,        // unique cells with visit > 0
    pub dominant_cell: Cell,       // most-visited cell
    pub dominant_pct: f32,         // fraction of time in dominant cell
    pub mean_recurrence: f32,      // avg frames between revisits to same cell
    pub divergence_rate: f32,      // Lyapunov exponent proxy
    pub total_transitions: u16,    // basin-switching events logged
}

// ── Tracker state ───────────────────────────────────────────

pub struct Tracker {
    // visit histogram (GRID x GRID)
    pub visits: [u32; GRID2],
    // last visit frame per cell (for recurrence time)
    last_visit: [u64; GRID2],
    // recurrence time accumulator
    recurrence_sum: [u64; GRID2],
    recurrence_count: [u32; GRID2],
    // transition log (circular buffer)
    pub transitions: [Transition; MAX_TRANSITIONS],
    pub transition_head: usize,
    pub transition_count: usize,
    // current and previous cell
    pub current_cell: Cell,
    prev_cell: Cell,
    // divergence tracking (shadow trajectory)
    shadow_z: [f64; RMAX],
    shadow_initialized: bool,
    divergence_acc: f64,
    divergence_samples: u32,
    // online PCA (EMA covariance + power iteration)
    cov: [f64; RMAX * RMAX],     // EMA covariance matrix (symmetric, upper tri used)
    pc0: [f64; RMAX],            // first principal component
    pc1: [f64; RMAX],            // second principal component (deflated)
    pca_alpha: f64,              // EMA decay for covariance (0.98 default)
    pca_warmup: u32,             // frames before PCA projection activates
    // total frames tracked
    pub frames_tracked: u64,
    // config
    pub z_range: f64,  // expected ‖Z‖ range for grid mapping
}

impl Tracker {
    pub fn new(z_range: f64) -> Self {
        // init PC0/PC1 to axis-aligned (will converge within ~50 frames)
        let mut pc0 = [0.0f64; RMAX]; pc0[0] = 1.0;
        let mut pc1 = [0.0f64; RMAX]; if RMAX > 1 { pc1[1] = 1.0; }
        Self {
            visits: [0; GRID2],
            last_visit: [0; GRID2],
            recurrence_sum: [0; GRID2],
            recurrence_count: [0; GRID2],
            transitions: [Transition::default(); MAX_TRANSITIONS],
            transition_head: 0,
            transition_count: 0,
            current_cell: Cell::default(),
            prev_cell: Cell::default(),
            shadow_z: [0.0; RMAX],
            shadow_initialized: false,
            divergence_acc: 0.0,
            divergence_samples: 0,
            cov: [0.0; RMAX * RMAX],
            pc0, pc1,
            pca_alpha: 0.98,
            pca_warmup: 50,
            frames_tracked: 0,
            z_range: z_range.max(1e-6),
        }
    }

    /// Call once per frame after core.step(). Pass Z as f64 slice.
    pub fn track(&mut self, z: &[f64], r: usize, frame: u64, energy: f64, stress: f64) {
        let r = r.min(RMAX);
        let a = self.pca_alpha;

        // ── 1. Update EMA covariance matrix (O(r²), symmetric) ──
        let mut i = 0;
        while i < r {
            let mut j = i;
            while j < r {
                let sample = z[i] * z[j];
                let idx = i * RMAX + j;
                self.cov[idx] = a * self.cov[idx] + (1.0 - a) * sample;
                if i != j { self.cov[j * RMAX + i] = self.cov[idx]; } // symmetric
                j += 1;
            }
            i += 1;
        }

        // ── 2. Power iteration for PC0 (5 iterations, O(r) each) ──
        if self.frames_tracked >= self.pca_warmup as u64 {
            // PC0: dominant eigenvector of cov
            let mut k = 0;
            while k < 5 {
                let mut new_pc = [0.0f64; RMAX];
                let mut ii = 0;
                while ii < r {
                    let mut sum = 0.0;
                    let mut jj = 0;
                    while jj < r { sum += self.cov[ii * RMAX + jj] * self.pc0[jj]; jj += 1; }
                    new_pc[ii] = sum;
                    ii += 1;
                }
                // normalize
                let mut norm2 = 0.0;
                ii = 0; while ii < r { norm2 += new_pc[ii] * new_pc[ii]; ii += 1; }
                let inv = if norm2 > 1e-30 { 1.0 / norm2.sqrt() } else { 1.0 };
                ii = 0; while ii < r { self.pc0[ii] = new_pc[ii] * inv; ii += 1; }
                k += 1;
            }

            // PC1: deflate cov by PC0, then power iterate
            // deflated_cov = cov - (cov·pc0)(pc0ᵀ) [rank-1 subtraction]
            // Instead of building deflated matrix, just orthogonalize result against pc0
            k = 0;
            while k < 5 {
                let mut new_pc = [0.0f64; RMAX];
                let mut ii = 0;
                while ii < r {
                    let mut sum = 0.0;
                    let mut jj = 0;
                    while jj < r { sum += self.cov[ii * RMAX + jj] * self.pc1[jj]; jj += 1; }
                    new_pc[ii] = sum;
                    ii += 1;
                }
                // subtract projection onto pc0 (Gram-Schmidt)
                let mut dot0 = 0.0;
                ii = 0; while ii < r { dot0 += new_pc[ii] * self.pc0[ii]; ii += 1; }
                ii = 0; while ii < r { new_pc[ii] -= dot0 * self.pc0[ii]; ii += 1; }
                // normalize
                let mut norm2 = 0.0;
                ii = 0; while ii < r { norm2 += new_pc[ii] * new_pc[ii]; ii += 1; }
                let inv = if norm2 > 1e-30 { 1.0 / norm2.sqrt() } else { 1.0 };
                ii = 0; while ii < r { self.pc1[ii] = new_pc[ii] * inv; ii += 1; }
                k += 1;
            }
        }

        // ── 3. Project Z onto PC0 and PC1 (adaptive Poincaré section) ──
        let mut proj0 = 0.0f64;
        let mut proj1 = 0.0f64;
        i = 0;
        while i < r { proj0 += z[i] * self.pc0[i]; proj1 += z[i] * self.pc1[i]; i += 1; }

        // ── 4. Map to grid cell ─────────────────────────────
        let ix = self.to_grid(proj0);
        let iy = self.to_grid(proj1);
        self.current_cell = Cell { ix, iy };
        let cid = self.current_cell.id();

        // ── 3. Update visit histogram ───────────────────────
        self.visits[cid] = self.visits[cid].saturating_add(1);

        // ── 4. Update recurrence time ───────────────────────
        if self.last_visit[cid] > 0 && frame > self.last_visit[cid] {
            let dt = frame - self.last_visit[cid];
            self.recurrence_sum[cid] = self.recurrence_sum[cid].saturating_add(dt);
            self.recurrence_count[cid] = self.recurrence_count[cid].saturating_add(1);
        }
        self.last_visit[cid] = frame;

        // ── 5. Detect basin transition ──────────────────────
        if self.frames_tracked > 0 && self.current_cell.id() != self.prev_cell.id() {
            let t = Transition {
                frame,
                from: self.prev_cell,
                to: self.current_cell,
                energy_at_transition: energy,
                stress_at_transition: stress,
            };
            self.transitions[self.transition_head] = t;
            self.transition_head = (self.transition_head + 1) % MAX_TRANSITIONS;
            if self.transition_count < MAX_TRANSITIONS {
                self.transition_count += 1;
            }
        }
        self.prev_cell = self.current_cell;

        // ── 6. Divergence rate (Lyapunov proxy) ─────────────
        // Shadow trajectory: Z_shadow = Z + ε at init, then evolves freely.
        // Divergence = ‖Z − Z_shadow‖ / ε over time.
        // We approximate by tracking ‖ΔZ‖ frame-to-frame.
        if !self.shadow_initialized {
            let mut k = 0;
            while k < r { self.shadow_z[k] = z[k] + 1e-8; k += 1; }
            self.shadow_initialized = true;
        } else {
            let mut dist2 = 0.0;
            let mut k = 0;
            while k < r {
                let d = z[k] - self.shadow_z[k];
                dist2 += d * d;
                // shadow follows Z with small lag (simplified)
                self.shadow_z[k] = z[k] + 1e-8 * (self.shadow_z[k] - z[k]).signum();
                k += 1;
            }
            if dist2 > 0.0 {
                self.divergence_acc += dist2.sqrt().ln();
                self.divergence_samples += 1;
            }
        }

        self.frames_tracked += 1;
    }

    /// Generate attractor classification report.
    pub fn report(&self) -> AttractorReport {
        let total_visits: u32 = self.visits.iter().sum();
        if total_visits == 0 || self.frames_tracked < RECURRENCE_WINDOW as u64 {
            return AttractorReport {
                atype: AttractorType::Transient,
                cells_visited: 0, dominant_cell: Cell::default(),
                dominant_pct: 0.0, mean_recurrence: 0.0,
                divergence_rate: 0.0, total_transitions: 0,
            };
        }

        // cells visited
        let cells_visited = self.visits.iter().filter(|&&v| v > 0).count() as u16;

        // dominant cell
        let mut max_v = 0u32;
        let mut max_id = 0usize;
        for i in 0..GRID2 {
            if self.visits[i] > max_v { max_v = self.visits[i]; max_id = i; }
        }
        let dominant_cell = Cell { ix: (max_id / GRID) as u8, iy: (max_id % GRID) as u8 };
        let dominant_pct = max_v as f32 / total_visits as f32;

        // mean recurrence
        let mut rec_total = 0.0f64;
        let mut rec_n = 0u32;
        for i in 0..GRID2 {
            if self.recurrence_count[i] > 0 {
                rec_total += self.recurrence_sum[i] as f64 / self.recurrence_count[i] as f64;
                rec_n += 1;
            }
        }
        let mean_recurrence = if rec_n > 0 { (rec_total / rec_n as f64) as f32 } else { 0.0 };

        // divergence rate (Lyapunov proxy)
        let divergence_rate = if self.divergence_samples > 0 {
            (self.divergence_acc / self.divergence_samples as f64) as f32
        } else { 0.0 };

        // classify attractor type
        let atype = if dominant_pct > 0.90 {
            AttractorType::FixedPoint
        } else if cells_visited <= 8 && mean_recurrence > 0.0 && mean_recurrence < 50.0 {
            AttractorType::LimitCycle
        } else if divergence_rate < 0.01 && cells_visited > 8 {
            AttractorType::Torus
        } else if divergence_rate > 0.1 {
            AttractorType::Strange
        } else {
            AttractorType::Unknown
        };

        AttractorReport {
            atype, cells_visited, dominant_cell, dominant_pct,
            mean_recurrence, divergence_rate,
            total_transitions: self.transition_count as u16,
        }
    }

    // ── grid mapping ────────────────────────────────────────
    #[inline]
    fn to_grid(&self, val: f64) -> u8 {
        // map [-z_range, +z_range] → [0, GRID-1]
        let normalized = (val / self.z_range + 1.0) * 0.5; // [0, 1]
        let clamped = normalized.max(0.0).min(0.999);
        (clamped * GRID as f64) as u8
    }
}

// (find_top_two_modes removed — replaced by online PCA projection)

// ── C ABI ───────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn dvsm_tracker_new(z_range: f64) -> *mut Tracker {
    #[cfg(feature = "std")]
    { std::boxed::Box::into_raw(std::boxed::Box::new(Tracker::new(z_range))) }
    #[cfg(not(feature = "std"))]
    { core::ptr::null_mut() }
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_tracker_track(
    t: *mut Tracker, z: *const f64, r: u32, frame: u64, energy: f64, stress: f64,
) -> i32 {
    let t = match t.as_mut() { Some(t) => t, None => return -1 };
    if z.is_null() { return -2; }
    let zs = core::slice::from_raw_parts(z, r.min(RMAX as u32) as usize);
    t.track(zs, r as usize, frame, energy, stress);
    0
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_tracker_report(t: *const Tracker, out: *mut AttractorReport) -> i32 {
    let t = match t.as_ref() { Some(t) => t, None => return -1 };
    let o = match out.as_mut() { Some(o) => o, None => return -2 };
    *o = t.report();
    0
}

#[no_mangle]
pub unsafe extern "C" fn dvsm_tracker_free(t: *mut Tracker) {
    #[cfg(feature = "std")]
    if !t.is_null() { drop(std::boxed::Box::from_raw(t)); }
}
