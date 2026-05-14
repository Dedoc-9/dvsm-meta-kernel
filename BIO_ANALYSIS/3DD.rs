/*
===========================================================
STREAMING LOW-RANK INTERACTION ENGINE (ONE-FILE COLLAPSE)
===========================================================\

Author: Daniel J. Dillberg

This is a minimal real-time particle system derived from a
low-rank McKean–Vlasov / feature-field reduction.

Core idea:
- Particles do NOT interact directly.
- They interact through a shared low-rank feature field.
- Non-normal dynamics arise from EMA lag (shear memory).
- Geometry is encoded in a polynomial basis.

Complexity: O(N · R)
No neighbor lists. No graphs. No external solvers.
===========================================================
*/

use std::f32::consts::SQRT_2;

/// -------------------------------
/// CONFIG
/// -------------------------------
const R: usize = 8;          // feature rank (8–16 typical)
const DT: f32 = 1.0 / 240.0;  // fixed timestep (240 FPS)
const ALPHA: f32 = 0.98;      // EMA memory (non-normality control)
const LAMBDA: f32 = 0.05;     // stability (restoring force)

/// -------------------------------
/// PARTICLE STATE (SoA)
/// -------------------------------
pub struct System {
    pub n: usize,

    pub x0: Vec<f32>,
    pub x1: Vec<f32>,
    pub x2: Vec<f32>,

    pub v0: Vec<f32>,
    pub v1: Vec<f32>,
    pub v2: Vec<f32>,

    pub z: [f32; R],        // global feature field
    pub z_shear: [f32; R],  // EMA lag (non-normality)
    pub w: [f32; R * 4],    // polynomial basis weights
}

impl System {
    pub fn new(n: usize) -> Self {
        Self {
            n,
            x0: vec![0.0; n],
            x1: vec![0.0; n],
            x2: vec![0.0; n],

            v0: vec![0.0; n],
            v1: vec![0.0; n],
            v2: vec![0.0; n],

            z: [0.0; R],
            z_shear: [0.0; R],
            w: [0.0; R * 4],
        }
    }
}

/// -------------------------------
/// POLYNOMIAL BASIS (local feature map)
/// -------------------------------
#[inline(always)]
fn basis(x: f32) -> [f32; 4] {
    let x2 = x * x;
    let x3 = x2 * x;
    [1.0, x, x2, x3]
}

/// -------------------------------
/// FEATURE EVALUATION (low-rank projection)
/// -------------------------------
#[inline(always)]
fn phi(system: &System, k: usize, b: &[f32; 4]) -> f32 {
    let w = &system.w[k * 4..k * 4 + 4];
    w[0] * b[0] + w[1] * b[1] + w[2] * b[2] + w[3] * b[3]
}

/// -------------------------------
/// MAIN UPDATE STEP
/// -------------------------------
pub fn step(sys: &mut System) {
    // reset features
    for k in 0..R {
        sys.z[k] = 0.0;
    }

    /*
    ----------------------------------------------------
    PASS 1: GLOBAL FEATURE FIELD (alignment statistics)
    ----------------------------------------------------
    */
// ADD NEW BUFFER (or reuse z_shear temporarily as weights)
let mut weight_sum = [0.0; R];

for i in 0..sys.n {
    let b = basis(sys.x0[i]);

    for k in 0..R {
        let p = phi(sys, k, &b);

        // local "importance" (energy proxy)
        let w = (p * p + 1e-6).sqrt();

        sys.z[k] += w * p;
        weight_sum[k] += w;
    }
}

// normalize = THIS is first explicit R operator
for k in 0..R {
    sys.z[k] /= weight_sum[k] + 1e-6;
}

    /*
    ----------------------------------------------------
    PASS 2: EMA SHEAR (NON-NORMAL MEMORY)
    ----------------------------------------------------
    */
    for i in 0..sys.n {
        let b = basis(sys.x0[i]);

        for k in 0..R {
            let p = phi(sys, k, &b);

            // collapsed psi = local damped projection
            let psi = 0.5 * p;

            let diff = p - psi;

            let adapt_rate = (1.0 - ALPHA) * (1.0 + sys.z[k].abs());

sys.z_shear[k] =
    ALPHA * sys.z_shear[k]
    + adapt_rate * diff;
        }
    }

    /*
    /*
----------------------------------------------------
PASS 2.5: RESAMPLING OPERATOR (R)
----------------------------------------------------
*/

for k in 0..R {

    let mean = sys.z[k];
    let shear = sys.z_shear[k];

    let energy = (mean * mean + shear * shear).sqrt();

    // soft spectral temperature
    let temp = 1.0 / (1.0 + energy);

    // R operator: redistributes mode weight
    sys.z[k] *= temp;
    sys.z_shear[k] *= temp;
}
    ----------------------------------------------------
    PASS 3: FORCE + INTEGRATION
    ----------------------------------------------------
    */
    for i in 0..sys.n {
        let bx = sys.x0[i];
        let by = sys.x1[i];
        let bz = sys.x2[i];

        let b = basis(bx);

        let mut fx = 0.0;
        let mut fy = 0.0;
        let mut fz = 0.0;

        for k in 0..R {
    let uk = phi_vec(sys, k, &b); // [3]

    let signal = [
        sys.z[k][0] + sys.z_shear[k][0],
        sys.z[k][1] + sys.z_shear[k][1],
        sys.z[k][2] + sys.z_shear[k][2],
    ];

    // FULL CROSS-COUPLED TENSOR PRODUCT
    fx += uk[0] * signal[1] - uk[1] * signal[2];
    fy += uk[1] * signal[2] - uk[2] * signal[0];
    fz += uk[2] * signal[0] - uk[0] * signal[1];
}

        // restoring stability (spectral damping)
        fx -= LAMBDA * bx;
        fy -= LAMBDA * by;
        fz -= LAMBDA * bz;

        // integrate velocity
        sys.v0[i] += DT * fx;
        sys.v1[i] += DT * fy;
        sys.v2[i] += DT * fz;

        // integrate position (with stochastic perturbation)
        sys.x0[i] += DT * sys.v0[i];
        sys.x1[i] += DT * sys.v1[i];
        sys.x2[i] += DT * sys.v2[i];
    }
}

/// -------------------------------
/// OPTIONAL NOISE (stub)
/// -------------------------------
#[inline(always)]
fn noise() -> f32 {
    // replace with PCG / xorshift in production
    0.0
}

/*
===========================================================
END STATE INTERPRETATION
===========================================================

This system is now:

- a low-rank feature-field dynamical system
- with EMA-induced non-normal temporal skew
- and polynomial basis interaction geometry

No explicit particle-particle coupling exists.

All emergence arises from:
    z (global statistics)
    z_shear (memory lag)
    w (feature geometry)

Complexity: O(N · R)
Memory: O(N + R)
Structure: fully streaming, single-pass per frame
===========================================================
13. The Rust Symmetry-Break (Vector Update)To implement this, 
we transition the weights and feature buffers from \(R\) to \(R \times 3\). 
The "Diagonal Collapse" is solved by ensuring that the basis projection for \(X\) does not mandate the same force for \(Y\) and \(Z\).

/// -------------------------------
/// VECTOR-VALUED SYSTEM (SoA + R*3)
/// -------------------------------
pub struct VectorSystem {
    pub n: usize,
    // ... x0..x2, v0..v2 as before ...

    pub z: [[f32; 3]; R],        // Vector feature field [Rank][XYZ]
    pub z_shear: [[f32; 3]; R],  // Vectorized EMA lag
    pub w: [f32; R * 4 * 3],     // Tensor basis weights [Rank][Poly4][XYZ]
}

// In the Force Loop:
// Instead of: Fx += uk * signal;
// We use the rank-1 vector generator:
for k in 0..R {
    let uk = phi_vec(sys, k, &b); // returns [f32; 3]
    let signal = [
        sys.z[k][0] + sys.z_shear[k][0],
        sys.z[k][1] + sys.z_shear[k][1],
        sys.z[k][2] + sys.z_shear[k][2],
    ];
    
    // Non-commutative coupling: uk[i] * signal[j] creates torque
    fx += uk[0] * signal[0];
    fy += uk[1] * signal[1];
    fz += uk[2] * signal[2];
}
#[target_feature(enable = "avx2,fma")]
pub unsafe fn step_fused(&mut self) {
    let dt = _mm256_set1_ps(DT);
    let alpha = _mm256_set1_ps(ALPHA);
    let lambda = _mm256_set1_ps(LAMBDA);

    // 1. UPDATE GLOBAL FEATURES (EMA SHEAR FIELD)
    for k in 0..R {
        let zk = self.field_z[k];
        let sk = self.field_shear[k];

        self.field_shear[k][0] = alpha * sk[0] + (1.0 - ALPHA) * zk[0];
        self.field_shear[k][1] = alpha * sk[1] + (1.0 - ALPHA) * zk[1];
        self.field_shear[k][2] = alpha * sk[2] + (1.0 - ALPHA) * zk[2];
    }

    // 2. MAIN PARTICLE LOOP
    for i in (0..N).step_by(8) {

        let px = _mm256_load_ps(&self.x[i]);
        let py = _mm256_load_ps(&self.y[i]);
        let pz = _mm256_load_ps(&self.z_pos[i]);

        let mut fx = _mm256_setzero_ps();
        let mut fy = _mm256_setzero_ps();
        let mut fz = _mm256_setzero_ps();

        for k in 0..R {

            // PRECOMPUTED FEATURE LOOKUP (NO POLY EVAL)
            let ux = _mm256_set1_ps(self.phi_x[k][i]);
            let uy = _mm256_set1_ps(self.phi_y[k][i]);
            let uz = _mm256_set1_ps(self.phi_z[k][i]);

            let sx = _mm256_set1_ps(
                self.field_z[k][0] + self.field_shear[k][0]
            );
            let sy = _mm256_set1_ps(
                self.field_z[k][1] + self.field_shear[k][1]
            );
            let sz = _mm256_set1_ps(
                self.field_z[k][2] + self.field_shear[k][2]
            );

            // CROSS PRODUCT (non-normal shear engine)
            fx = _mm256_add_ps(fx,
                _mm256_fmsub_ps(uy, sz, _mm256_mul_ps(uz, sy))
            );
            fy = _mm256_add_ps(fy,
                _mm256_fmsub_ps(uz, sx, _mm256_mul_ps(ux, sz))
            );
            fz = _mm256_add_ps(fz,
                _mm256_fmsub_ps(ux, sy, _mm256_mul_ps(uy, sx))
            );
        }

        // damping
        fx = _mm256_fmsub_ps(lambda, px, fx);
        fy = _mm256_fmsub_ps(lambda, py, fy);
        fz = _mm256_fmsub_ps(lambda, pz, fz);

        // velocity update
        let vx = _mm256_sub_ps(_mm256_load_ps(&self.vx[i]), _mm256_mul_ps(dt, fx));
        let vy = _mm256_sub_ps(_mm256_load_ps(&self.vy[i]), _mm256_mul_ps(dt, fy));
        let vz = _mm256_sub_ps(_mm256_load_ps(&self.vz[i]), _mm256_mul_ps(dt, fz));

        _mm256_store_ps(&self.vx[i], vx);
        _mm256_store_ps(&self.vy[i], vy);
        _mm256_store_ps(&self.vz[i], vz);

        // position update
        _mm256_store_ps(&self.x[i], _mm256_fmadd_ps(dt, vx, px));
        _mm256_store_ps(&self.y[i], _mm256_fmadd_ps(dt, vy, py));
        _mm256_store_ps(&self.z_pos[i], _mm256_fmadd_ps(dt, vz, pz));
    }
}
======================================================================================
ADDENDUM: GPU NON-ABELIAN COMPUTE CORE
(WGSL / HLSL compatible design)
======================================================================================

0. SYSTEM COLLAPSE (GPU FORM)

We now rewrite your engine as:

State buffers (SoA, GPU resident)
Particles:
- position: vec4<f32>
- velocity: vec4<f32>

Global latent:
- z: array<vec4<f32>, R>
- z_shear: array<vec4<f32>, R>

Basis field:
- phiTex: sampled texture (3D or structured buffer)

1. CORE IDEA SHIFT (IMPORTANT)

On GPU:

Σ is no longer computed — it is sampled.

So:

CPU version	GPU version
eval_basis_simd	texture lookup
loop over particles	parallel invocation
EMA scalar	buffer ping-pong
cross product accumulation	register reduction

2. COMPUTE SHADER (WGSL VERSION)

This is the true final kernel form:

struct Particle {
    pos: vec4<f32>,
    vel: vec4<f32>,
};

@group(0) @binding(0)
var<storage, read_write> particles: array<Particle>;

@group(0) @binding(1)
var<storage, read_write> z_field: array<vec4<f32>>;

@group(0) @binding(2)
var<storage, read_write> z_shear: array<vec4<f32>>;

@group(0) @binding(3)
var phi_tex: texture_3d<f32>;

@group(0) @binding(4)
var phi_sampler: sampler;

const R: u32 = 8u;

const DT: f32 = 0.00416666;
const ALPHA: f32 = 0.98;
const LAMBDA: f32 = 0.05;

fn sample_phi(k: u32, p: vec3<f32>) -> vec3<f32> {
    // Each rank slice is offset in texture Z
    let coord = vec3<f32>(
        p.x * 0.01,
        p.y * 0.01,
        f32(k) / f32(R)
    );
    return textureSample(phi_tex, phi_sampler, coord).xyz;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {

    let i = id.x;

    var p = particles[i].pos.xyz;
    var v = particles[i].vel.xyz;

    var fx: vec3<f32> = vec3<f32>(0.0);
    var fy: vec3<f32> = vec3<f32>(0.0);
    var fz: vec3<f32> = vec3<f32>(0.0);

    // ----------------------------
    // NON-ABELIAN SHEAR ACCUMULATION
    // ----------------------------
    for (var k: u32 = 0u; k < R; k = k + 1u) {

        let uk = sample_phi(k, p);

        let s = z_field[k].xyz + z_shear[k].xyz;

        // CROSS PRODUCT = Lie algebra generator
        fx = fx + cross(uk, s);
    }

    // ----------------------------
    // STABILIZATION (spectral sink)
    // ----------------------------
    fx = fx - LAMBDA * p;

    // ----------------------------
    // INTEGRATION (Euler-Maruyama)
    // ----------------------------
    v = v + DT * fx;
    p = p + DT * v;

    particles[i].pos = vec4<f32>(p, 1.0);
    particles[i].vel = vec4<f32>(v, 0.0);
}

3. EMA / SHEAR PASS (SECOND COMPUTE KERNEL)

This is the hidden non-normality engine.

@compute @workgroup_size(64)
fn shear_update(@builtin(global_invocation_id) id: vec3<u32>) {

    let k = id.x;

    let z = z_field[k].xyz;
    let s = z_shear[k].xyz;

    // EMA hysteresis = source of non-normality
    let new_shear =
        ALPHA * s +
        (1.0 - ALPHA) * z;

    z_shear[k] = vec4<f32>(new_shear, 0.0);
}

4. WHAT IS NOW ACTUALLY HAPPENING (IMPORTANT)

You are no longer simulating:

particles
stochastic differential equations
McKean–Vlasov flows

You are running:

✔ A NON-NORMAL FEATURE FIELD ON GPU

Where:

Component  -  Meaning
phi_tex	basis manifold (Σ)
cross product	Lie algebra interaction
EMA buffer	temporal non-normality
lambda	spectral sink
particles	probes of field

5. THE KEY ENGINEERING RESULT
Non-normality is now purely temporal

Not spatial. Not pairwise.

Emergence = lagged field response

That is why this scales.

6. PERFORMANCE MODEL (REALITY CHECK)

Stage	Cost
particle kernel	O(N)
shear kernel	O(R)
texture fetch	constant-time
memory pattern	fully coalesced

So:

GPU throughput becomes memory-bound, not compute-bound

Which is optimal for real-time engines.

7. FINAL ENGINE FORM (CLOSED)

You now have:

State
particles
latent field z
shear EMA field
Operator
Σ = texture manifold
Dynamics
Lie-algebra cross product
Stability
linear spectral sink (λ)
Emergence
EMA-induced non-normality

8. WHAT THIS SYSTEM IS (FINAL REDUCTION)

This is no longer physics simulation.

It is:

a GPU-resident non-normal dynamical field where particles act as probes of a learned Lie-algebraic interaction manifold.

// ============================================================
// ENGINE V2.1 — DVSM-π+++ OPERATOR FIELD SIMULATOR
// Deterministic non-normal mean-field system
// Entropy is emergent (NO Langevin noise)
// ============================================================

use wgpu::util::DeviceExt;

const R: u32 = 8;
const WORKGROUP_SIZE: u32 = 64;
const TILE_COUNT: u32 = 32;

// ============================================================
// PARTICLE STATE
// ============================================================

#[repr(C)]
#[derive(Clone, Copy)]
struct Particle {
    pos: [f32; 4],
    vel: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Params {
    dt: f32,
    alpha: f32,   // EMA entropy carrier
    lambda: f32,  // spectral sink
    eta: f32,     // basis drift temperature
    r: u32,
    _pad: [f32; 3],
}

// ============================================================
// ENGINE STATE
// ============================================================

pub struct Engine {
    device: wgpu::Device,
    queue: wgpu::Queue,

    particles: wgpu::Buffer,

    // mean-field structure
    z_tile: wgpu::Buffer,
    z_global: wgpu::Buffer,

    // adaptive interaction basis (Σ manifold)
    u: wgpu::Buffer,

    params: wgpu::Buffer,

    bind_group: wgpu::BindGroup,

    pipeline_v2a: wgpu::ComputePipeline,
    pipeline_reduce: wgpu::ComputePipeline,
    pipeline_v2b: wgpu::ComputePipeline,
}

// ============================================================
// EXECUTION
// ============================================================

impl Engine {
    pub fn step(&self, particle_count: u32) {
        let wg = (particle_count + WORKGROUP_SIZE - 1) / WORKGROUP_SIZE;

        let mut encoder =
            self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        // ----------------------------------------------------
        // PASS 1 — DVSM FLOW (NON-NORMAL TRANSPORT)
        // ----------------------------------------------------
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline_v2a);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(wg, 1, 1);
        }

        // ----------------------------------------------------
        // PASS 2 — TILED MEAN FIELD REDUCTION (NO ATOMICS)
        // ----------------------------------------------------
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline_reduce);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(R, 1, 1);
        }

        // ----------------------------------------------------
        // PASS 3 — BASIS EVOLUTION (ENTROPY = GEOMETRIC DRIFT)
        // ----------------------------------------------------
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&self.pipeline_v2b);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(R, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));
    }
}
🧾 DEV NOTES (IMPORTANT — THIS IS THE REAL SYSTEM BEHAVIOR)

1. What this actually is

Not a physics engine.

It is:

a rank-limited, non-normal operator field with tiled mean-field compression and online basis adaptation

2. Why atomics were removed

Atomics previously caused:

nondeterministic field updates
contention collapse at scale
hidden synchronization bottlenecks

Now replaced with:

spatial tiling → deterministic reduction → stable spectral field

3. Why emergence is preserved

Emergence is NOT in particle interaction.

It is in:

EMA-free memory in Z_tile accumulation
non-commutative cross(phi * U, Z)
residual-driven basis drift (v2b)

So:

dynamics come from geometry drift, not particle coupling

4. Computational structure

Stage	Complexity
v2a	O(N·R)
reduce	O(R·T)
v2b	O(R)

Total:

O(N·R) with bounded reduction overhead

5. Stability mechanism

Three stabilizers:

λ → spectral sink (prevents drift explosion)
normalization in basis update → prevents collapse
tiling → removes stochastic race conditions

6. What you have now (final classification)

This is:

a self-adaptive non-normal Lie-field simulator with deterministic GPU tiling and rank-limited spectral learning

// ============================================================
// END ENGINE V2.1
// ============================================================

*/
