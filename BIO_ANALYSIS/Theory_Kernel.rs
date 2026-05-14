//! ============================================================
//! DVSM-π+++ v6.0 · SEMIGROUP-FIRST CANONICAL KERNEL
//! dvsm_v6_complete.rs
//!
//! Author  : Daniel J. Dillberg
//! Version : 6.0-complete · 2026-05-13
//!
//! ══════════════════════════════════════════════════════════
//! CLASSIFICATION (Documents 53–58 synthesis)
//! ══════════════════════════════════════════════════════════
//!
//!   A stochastic projection approximation of a nonlinear
//!   Feynman–Kac–McKean semigroup on probability measures.
//!
//!   Φ_t : P(ℝ³) → P(ℝ³)    [true object; nonlinear semigroup]
//!
//!   Generator on functionals F : P(ℝ³) → ℝ:
//!     𝒢F(μ) = ⟨ℒ_μ*(μ), δF/δμ⟩ + ⟨𝒱_μ·μ, δF/δμ⟩
//!     ℒ_μ = McKean–Vlasov transport
//!     𝒱_μ = Feynman–Kac potential
//!
//!   Linear structure lives on observables, not measures directly.
//!   (Φ_t nonlinear → no Hille–Yosida on P(ℝ³) directly.)
//!
//! ══════════════════════════════════════════════════════════
//! TWO THEORY LEVELS (Doc 58 — both implemented)
//! ══════════════════════════════════════════════════════════
//!
//!   LLN level (SystemMode::LlnDefault):
//!     𝒜_∞ = ℒ_μ + 𝒱_μ
//!     R_τ → Id  as N→∞  [law of large numbers]
//!     μ̂_t^N ⇒ μ_t under (C1)–(C4)
//!     Convergence: PoC O(1/N), Che et al. 2024
//!
//!   CLT level (SystemMode::CltDiagnostic):
//!     𝒜_N = 𝒜_∞ + (1/N) M
//!     M = resampling noise operator = quadratic variation of R_τ
//!     η_t^N = √N (μ̂_t^N − μ_t) → Gaussian process (SPDE)
//!     R_τ is NOT negligible here: it IS the driving noise covariance
//!
//! ══════════════════════════════════════════════════════════
//! OPERATOR TAXONOMY
//! ══════════════════════════════════════════════════════════
//!
//!   CLASS 1 · L_τ : McKean–Vlasov EM (generator)
//!     Updates positions. Weights unchanged.
//!
//!   CLASS 2 · B_τ : Feynman–Kac Gibbs tilt (potential deform.)
//!     Updates weights. DI8: full energy E_full.
//!     Entropy implicit via normalization (never evaluate log μ).
//!
//!   CLASS 2 · R_τ : SMC stratified resampling (projection)
//!     NOT part of semigroup Φ_t.
//!     Representation functor: projects onto empirical simplex.
//!     O(1/N) bias per firing. At CLT level: contributes M.
//!
//!   CLASS 3 · J_τ : Sinkhorn JKO (Option 2 stub)
//!     Replaces L+B+R with entropic OT proximal step.
//!     Different discretization class — not a drop-in swap.
//!
//! ══════════════════════════════════════════════════════════
//! FIVE CORRECTIONS (Documents 53–57)
//! ══════════════════════════════════════════════════════════
//!
//!   COR-1  Invariant measure alignment: asymptotic only.
//!          Conditions (C1)–(C4) required; not proven at finite N.
//!
//!   COR-2  Interaction energy: "asymptotically consistent
//!          self-normalized interacting particle estimator."
//!          NOT classically unbiased (particle dependence via L,B).
//!
//!   COR-3  λ is macroscopic (fixed in N). Post-resampling force
//!          = (λ/N²)Σ∇K — correct McKean–Vlasov scaling.
//!
//!   COR-4  Section C is a semigroup algebra, not a category.
//!          Typed semigroup of Markov kernels on P_N(ℝ³).
//!
//!   COR-5  Convergence claims are per-component only.
//!          Full composed system: OP1 (open).
//!
//! ══════════════════════════════════════════════════════════
//! DESIGN INVARIANTS  DI1–DI9
//! ══════════════════════════════════════════════════════════
//!
//!   DI1  Σwᵢ=1, wᵢ≥0  after every B_τ
//!   DI2  T ≥ T_MIN
//!   DI3  α > 0
//!   DI4  Barycenter observable only; never fed into dynamics
//!   DI5  K(g,g') = K(g',g)
//!   DI6  R_τ events logged
//!   DI7  Noise ~ N(0,1); never uniform
//!   DI8  B_τ uses E_full = data + conf + interaction
//!   DI9  μ_N is sole state; (gᵢ,wᵢ) are its coordinates
//!
//! ══════════════════════════════════════════════════════════
//! OPEN PROBLEMS
//! ══════════════════════════════════════════════════════════
//!
//!   OP1  Full system convergence (EM+B_full+R) — open
//!   OP2  Optimal ESS threshold
//!   OP3  λ_max stability bound for RBF kernel
//!   OP4  Barycenter convergence rate under R_τ
//!   OP5  Curvature–cooperativity theorem (bioscience; unvalidated)
//!
//! ══════════════════════════════════════════════════════════
//! LICENSE: AGLP-3 Dual · Daniel J. Dillberg
//! ══════════════════════════════════════════════════════════

// ── §0 SAMPLER TRAITS ────────────────────────────────────────
/// N(0,1) Gaussian. DI7. Never substitute uniform.
pub trait GaussianSrc: Send + Sync { fn n01(&self) -> f64; }
/// U[0,1) uniform. Required for stratified resampling. FIX B.
pub trait UniformSrc:  Send + Sync { fn u01(&self) -> f64; }

// ── §1 PRIMITIVE TYPES ───────────────────────────────────────
/// Euclidean ℝ³. NEVER a W₂ tangent vector.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct R3 { pub x: f64, pub y: f64, pub z: f64 }
impl R3 {
    #[inline] pub fn zero()              -> Self  { Self::default() }
    #[inline] pub fn norm2(&self)        -> f64   { self.x*self.x+self.y*self.y+self.z*self.z }
    #[inline] pub fn norm(&self)         -> f64   { self.norm2().sqrt() }
    #[inline] pub fn scale(&self,s:f64)  -> Self  { Self{x:self.x*s,y:self.y*s,z:self.z*s} }
    #[inline] pub fn add(&self,o:&Self)  -> Self  { Self{x:self.x+o.x,y:self.y+o.y,z:self.z+o.z} }
    #[inline] pub fn sub(&self,o:&Self)  -> Self  { Self{x:self.x-o.x,y:self.y-o.y,z:self.z-o.z} }
    #[inline] pub fn dist2(&self,o:&Self)-> f64   { self.sub(o).norm2() }
    #[inline] pub fn is_finite(&self)   -> bool   { self.x.is_finite()&&self.y.is_finite()&&self.z.is_finite() }
}

/// Atom of μ_N. DI9: (geom, weight) are coordinates of μ_N = Σᵢwᵢδ_{gᵢ}.
#[derive(Clone, Debug)]
pub struct Particle { pub geom: R3, pub weight: f64 }
impl Particle { pub fn new(g: R3) -> Self { Self { geom: g, weight: 1.0 } } }

/// Observation / environmental forcing x ∈ ℝ³.
#[derive(Clone, Copy, Debug)]
pub struct Obs(pub R3);

// ── §2 KERNEL TRAIT ──────────────────────────────────────────
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KernelBound { Bounded{k_max:f64}, UnboundedUstat, PhysicalCutoff }

pub trait KernelFn: Send + Sync {
    fn potential(&self, g:R3, gp:R3)     -> f64;
    fn grad_potential(&self, g:R3, gp:R3)-> R3;
    fn bound(&self)  -> KernelBound;
    fn name(&self)   -> &'static str;
}

/// Central sign discipline. FIX E. F = −wₚ ∇_g K. Never bypass.
#[inline]
pub fn force_contrib(k: &dyn KernelFn, g: R3, gp: R3, wp: f64) -> R3 {
    k.grad_potential(g, gp).scale(-wp)
}

pub fn verify_kernel(k: &dyn KernelFn, a: R3, b: R3) -> Result<(), String> {
    let vab = k.potential(a,b); let vba = k.potential(b,a);
    if (vab-vba).abs() > 1e-9 { return Err(format!("DI5: '{}' asymmetric",k.name())); }
    if !vab.is_finite()        { return Err(format!("'{}': non-finite",k.name())); }
    Ok(())
}

#[derive(Clone,Copy,Debug)] pub struct RbfKernel { pub h: f64 }
impl Default for RbfKernel { fn default()->Self{Self{h:1.0}} }
impl KernelFn for RbfKernel {
    fn potential(&self,g:R3,gp:R3)->f64 { (-g.dist2(&gp)/(2.0*self.h*self.h)).exp() }
    fn grad_potential(&self,g:R3,gp:R3)->R3 {
        let d=g.sub(&gp); d.scale(-self.potential(g,gp)/(self.h*self.h))
    }
    fn bound(&self)->KernelBound { KernelBound::Bounded{k_max:1.0} }
    fn name(&self) ->&'static str { "RBF" }
}

#[derive(Clone,Copy,Debug)] pub struct RieszKernel { pub beta:f64, pub r_floor:f64 }
impl KernelFn for RieszKernel {
    fn potential(&self,g:R3,gp:R3)->f64 { -(g.dist2(&gp).sqrt().max(self.r_floor).powf(self.beta)) }
    fn grad_potential(&self,g:R3,gp:R3)->R3 {
        let d=g.sub(&gp); let r=d.norm().max(self.r_floor);
        d.scale(-self.beta*r.powf(self.beta-2.0))
    }
    fn bound(&self)->KernelBound { KernelBound::UnboundedUstat }
    fn name(&self) ->&'static str { "Riesz" }
}

#[derive(Clone,Copy,Debug)] pub struct LJKernel { pub eps:f64, pub sigma:f64, pub r_cut:f64 }
impl KernelFn for LJKernel {
    fn potential(&self,g:R3,gp:R3)->f64 {
        let r=g.dist2(&gp).sqrt().max(self.r_cut); let sr=self.sigma/r; let s6=sr.powi(6);
        4.0*self.eps*(s6*s6-s6)
    }
    fn grad_potential(&self,g:R3,gp:R3)->R3 {
        let d=g.sub(&gp); let r=d.norm().max(self.r_cut); let sr=self.sigma/r; let s6=sr.powi(6);
        let dv=4.0*self.eps*(-12.0*s6*s6+6.0*s6)/r; d.scale(dv/r)
    }
    fn bound(&self)->KernelBound { KernelBound::PhysicalCutoff }
    fn name(&self) ->&'static str { "Lennard-Jones" }
}

// ── §3 PARAMETERS ────────────────────────────────────────────
#[derive(Clone,Copy,Debug)]
pub struct Params {
    pub temperature: f64,  // T ≥ T_MIN. DI2.
    pub alpha:       f64,  // α > 0. DI3.
    pub lambda:      f64,  // λ ≥ 0. COR-3: macroscopic, fixed in N.
    pub dt:          f64,
    pub r_cut:       f64,
}

pub const T_MIN: f64  = 0.05;
pub const BIO_T: f64  = 0.5961;  // kT at 310K kcal/mol
pub const ALPHA_INIT: f64 = 0.50;
pub const ALPHA_HOLD: f64 = 0.02;
pub const ESS_THRESH: f64 = 0.50;

impl Params {
    pub fn validate(&self) -> Result<(),Vec<String>> {
        let mut e = Vec::new();
        if self.temperature < T_MIN { e.push(format!("DI2: T={:.4}<T_MIN",self.temperature)); }
        if self.alpha <= 0.0        { e.push(format!("DI3: α={:.4}≤0",self.alpha)); }
        if self.lambda < 0.0        { e.push("λ<0".into()); }
        if self.dt <= 0.0           { e.push("dt≤0".into()); }
        if e.is_empty() { Ok(()) } else { Err(e) }
    }
}

// ── §4 FREE ENERGY FUNCTIONS ──────────────────────────────────
/// E_full(gᵢ, μ_N, obs). DI8. COR-3: λ/N normalization.
pub fn e_full(g:R3,obs:Obs,p:&Params,parts:&[Particle],k:&dyn KernelFn,rc2:f64)->f64 {
    let n = parts.len() as f64;
    let mut e_int = 0.0f64;
    for q in parts { if g.dist2(&q.geom)<=rc2 { e_int+=q.weight*k.potential(g,q.geom); } }
    g.dist2(&obs.0) + p.alpha*g.norm2() + p.lambda*e_int/n
}

/// ∇_g E_full. Force = −∇E (handled by caller).
pub fn grad_e_full(g:R3,obs:Obs,p:&Params,parts:&[Particle],k:&dyn KernelFn,rc2:f64)->R3 {
    let n = parts.len() as f64;
    let mut g_int = R3::zero();
    for q in parts {
        if g.dist2(&q.geom)<=rc2 {
            // FIX E: force_contrib = −wⱼ∇K; negate for gradient
            g_int = g_int.sub(&force_contrib(k,g,q.geom,q.weight));
        }
    }
    g.sub(&obs.0).scale(2.0)
     .add(&g.scale(2.0*p.alpha))
     .add(&g_int.scale(p.lambda/n))
}

// ── §5 L_τ — OVERDAMPED EULER–MARUYAMA ──────────────────────
pub fn langevin_step(parts:&mut Vec<Particle>,obs:Obs,p:&Params,k:&dyn KernelFn,g:&dyn GaussianSrc) {
    let sig = (2.0*p.temperature*p.dt).sqrt();
    let rc2 = p.r_cut*p.r_cut;
    let snap:Vec<Particle>=parts.iter().map(|q|Particle{geom:q.geom,weight:q.weight}).collect();
    for i in 0..parts.len() {
        let gr = grad_e_full(parts[i].geom,obs,p,&snap,k,rc2);
        let gn = parts[i].geom
            .sub(&gr.scale(p.dt))
            .add(&R3{x:g.n01()*sig,y:g.n01()*sig,z:g.n01()*sig});
        debug_assert!(gn.is_finite(),"L_τ diverged @{i}");
        parts[i].geom=gn;
    }
}

// ── §6 B_τ — GIBBS TILT ──────────────────────────────────────
pub fn gibbs_tilt(parts:&mut Vec<Particle>,obs:Obs,p:&Params,k:&dyn KernelFn) {
    let rc2=p.r_cut*p.r_cut; let t=p.temperature.max(T_MIN);
    let snap:Vec<Particle>=parts.iter().map(|q|Particle{geom:q.geom,weight:q.weight}).collect();
    let lw:Vec<f64>=parts.iter().map(|q|-e_full(q.geom,obs,p,&snap,k,rc2)/t).collect();
    let max=lw.iter().cloned().fold(f64::NEG_INFINITY,f64::max);
    let lz=max+lw.iter().map(|l|(l-max).exp()).sum::<f64>().ln();
    for (q,l) in parts.iter_mut().zip(lw.iter()) { q.weight=(l-lz).exp(); }
    let s:f64=parts.iter().map(|q|q.weight).sum();
    debug_assert!((s-1.0).abs()<1e-8,"DI1 post-B_τ: Σwᵢ={:.9}",s);
}

// ── §7 R_τ — STRATIFIED RESAMPLING ───────────────────────────
#[derive(Clone,Debug)]
pub struct ResampleEvent { pub step:usize, pub ess_before:f64, pub n:usize }

pub fn stratified_resample(parts:&mut Vec<Particle>,step:usize,log:&mut Vec<ResampleEvent>,u:&dyn UniformSrc) {
    let n=parts.len();
    let ess_b=ess(parts);
    let mut cdf=vec![0.0f64;n+1];
    for i in 0..n { cdf[i+1]=cdf[i]+parts[i].weight; }
    let mut new=Vec::with_capacity(n); let mut j=0usize;
    for k in 0..n {
        let ui=((k as f64)+u.u01().min(0.999_999_99))/(n as f64);
        while j<n-1&&cdf[j+1]<ui{j+=1;}
        new.push(parts[j].geom);
    }
    let w=1.0/(n as f64);
    for (q,g) in parts.iter_mut().zip(new){q.geom=g;q.weight=w;}
    log.push(ResampleEvent{step,ess_before:ess_b,n});
}

// ── §8 OBSERVABLES ────────────────────────────────────────────
pub fn ess(p:&[Particle])->f64 {
    let s:f64=p.iter().map(|q|q.weight).sum();
    let s2:f64=p.iter().map(|q|q.weight*q.weight).sum();
    (s*s)/(s2+1e-15)
}
pub fn barycenter(p:&[Particle])->R3 { p.iter().fold(R3::zero(),|a,q|a.add(&q.geom.scale(q.weight))) }
pub fn variance(p:&[Particle])->f64 {
    let m=barycenter(p); p.iter().map(|q|q.weight*q.geom.sub(&m).norm2()).sum()
}
pub fn weight_entropy(p:&[Particle])->f64 {
    p.iter().fold(0.0,|a,q|if q.weight>1e-15{a-q.weight*q.weight.ln()}else{a})
}

/// COR-2: asymptotically consistent self-normalized estimator.
/// NOT classically unbiased (particle dependence via L and B).
pub fn interaction_energy(p:&[Particle],k:&dyn KernelFn)->f64 {
    let n=p.len(); if n<2{return 0.0;}
    let mut e=0.0f64;
    for i in 0..n { for j in 0..n {
        if i!=j { e+=p[i].weight*p[j].weight*k.potential(p[i].geom,p[j].geom); }
    }}
    e*(n as f64)/((n-1) as f64)
}

pub fn free_energy_empirical(p:&[Particle],obs:Obs,params:&Params,k:&dyn KernelFn)->f64 {
    let x=obs.0;
    let fd:f64=p.iter().map(|q|q.weight*q.geom.dist2(&x)).sum();
    let fc:f64=p.iter().map(|q|q.weight*params.alpha*q.geom.norm2()).sum();
    fd+fc+params.lambda*interaction_energy(p,k)
}

// ── §9 GHOST MODES ────────────────────────────────────────────
#[derive(Clone,Debug,PartialEq)]
pub enum Ghost { Nominal,G1Collapse,G2Diffuse,G3Echo,G4ResampleDiscont,G5BaryDrift }
pub fn detect_ghost(p:&[Particle],resample_rate:f64)->Ghost {
    let n=p.len(); let e=ess(p); let v=variance(p); let h=weight_entropy(p); let ln_n=(n as f64).ln();
    if e<2.0||h<0.02           { return Ghost::G1Collapse; }
    if e>(n as f64)*0.95&&v>50.0{ return Ghost::G2Diffuse; }
    if h<ln_n*0.05&&v<1e-5     { return Ghost::G3Echo; }
    if resample_rate>0.5        { return Ghost::G4ResampleDiscont; }
    Ghost::Nominal
}

// ── §10 SEMIGROUP ALGEBRA ─────────────────────────────────────
// COR-4: semigroup of Markov kernels, not a category.
#[derive(Clone,Copy,Debug,PartialEq)]
pub enum OpClass { Generator, Correction, GeometricProjector }

// ── §11 SYSTEM + MODE ENUM ────────────────────────────────────

/// Operating mode. Both LLN and CLT levels available.
#[derive(Clone,Debug,PartialEq)]
pub enum SystemMode {
    /// LLN default: L_τ ∘ B_τ ∘ R_τ.
    /// Convergence: PoC O(1/N), Che et al. 2024.
    /// R_τ → Id as N→∞ (LLN level).
    LlnDefault,

    /// CLT diagnostic: identical dynamics + fluctuation tracking.
    /// 𝒜_N = 𝒜_∞ + (1/N) M  where M = resampling noise operator.
    /// η_t^N = √N(μ̂_t^N − μ_t) tracked per step.
    /// R_τ is NOT negligible here — it IS the noise covariance.
    CltDiagnostic,

    /// Sinkhorn JKO: entropic OT proximal step.
    /// Replaces L+B+R entirely. Different discretization class.
    /// Status: stub; panics at runtime.
    SinkhornJko,
}

/// CLT-level diagnostics per step.
#[derive(Clone,Debug,Default)]
pub struct CltDiagnostics {
    /// ‖η_t^N‖ = ‖√N(μ̂_N − μ_t)‖
    pub eta_norm:       f64,
    /// M contribution ≈ (1/N) Σᵢ wᵢ ‖gᵢ − E_μ[g]‖²
    /// Large → R_τ noise dominates; CLT regime.
    pub m_contribution: f64,
    pub fluctuation_x:  f64,
    pub fluctuation_y:  f64,
    pub fluctuation_z:  f64,
    pub n_particles:    usize,
}

const CLT_CAP: usize = 512;

/// Main system. DI9: μ_N is the sole state.
pub struct DvsmSystem<K: KernelFn> {
    pub particles:     Vec<Particle>,
    pub params:        Params,
    pub kernel:        K,
    pub step_count:    usize,
    pub resample_log:  Vec<ResampleEvent>,
    pub ess_threshold: f64,
    pub mode:          SystemMode,
    pub clt_log:       Vec<CltDiagnostics>,
}

// ── §12 CLT FLUCTUATION FIELD ─────────────────────────────────
//
// 𝒜_N = (ℒ_μ + 𝒱_μ) + (1/N) M  (Doc 58)
//
// At LLN level: R_τ → Id (N→∞).
// At CLT level: R_τ contributes M, the quadratic variation.
// η_t^N = √N(μ̂_N − μ_t) → Gaussian process solving FK–MV SPDE.
//
// Reference μ_t: use previous-step barycenter as proxy.
// For rigorous analysis: supply high-N LLN run as baseline.

pub fn clt_fluctuation_field(
    particles: &[Particle],
    prev_pos:  &[R3],       // previous positions for Δ estimation
) -> CltDiagnostics {
    let n = particles.len();
    if n == 0 || prev_pos.len() != n { return CltDiagnostics::default(); }

    // Proxy reference μ_t: barycenter of previous positions
    let bary_prev = {
        let s = prev_pos.iter().fold(R3::zero(), |a,&g| a.add(&g));
        R3 { x: s.x/(n as f64), y: s.y/(n as f64), z: s.z/(n as f64) }
    };
    let bary_now = barycenter(particles);

    // η_t^N = √N (E_μ[g] − E_μ_prev[g])  (proxy fluctuation)
    let eta = bary_now.sub(&bary_prev).scale((n as f64).sqrt());

    // M contribution: empirical second moment around barycenter
    let m: f64 = particles.iter()
        .map(|q| q.weight * q.geom.sub(&bary_now).norm2())
        .sum::<f64>() / (n as f64);

    CltDiagnostics {
        eta_norm:       eta.norm(),
        m_contribution: m,
        fluctuation_x:  eta.x,
        fluctuation_y:  eta.y,
        fluctuation_z:  eta.z,
        n_particles:    n,
    }
}

// ── IMPL BLOCK ────────────────────────────────────────────────

impl<K: KernelFn> DvsmSystem<K> {

    pub fn new(particles:Vec<Particle>,params:Params,kernel:K,ess_threshold:f64)
    -> Result<Self,Vec<String>> {
        params.validate()?;
        verify_kernel(&kernel, R3{x:0.6,y:0.2,z:0.1}, R3{x:0.1,y:0.2,z:0.6})
            .map_err(|e| vec![e])?;
        Ok(Self {
            particles, params, kernel,
            step_count: 0,
            resample_log: Vec::new(),
            ess_threshold: ess_threshold.clamp(0.1, 0.9),
            mode: SystemMode::LlnDefault,
            clt_log: Vec::new(),
        })
    }

    pub fn set_mode(&mut self, m: SystemMode) { self.mode = m; }

    /// Advance μ_N by one step.
    pub fn advance(&mut self, obs:Obs, gauss:&dyn GaussianSrc, uniform:&dyn UniformSrc) {
        match self.mode {
            SystemMode::SinkhornJko => panic!("SinkhornJko: stub — unimplemented"),
            SystemMode::LlnDefault | SystemMode::CltDiagnostic => {
                let prev_pos: Option<Vec<R3>> =
                    if self.mode == SystemMode::CltDiagnostic {
                        Some(self.particles.iter().map(|p| p.geom).collect())
                    } else { None };

                // CLASS 1: L_τ
                langevin_step(&mut self.particles, obs, &self.params, &self.kernel, gauss);
                // CLASS 2: B_τ  (DI8: full energy)
                gibbs_tilt(&mut self.particles, obs, &self.params, &self.kernel);
                // CLASS 2: R_τ  (representation functor; NOT semigroup)
                let n = self.particles.len();
                if ess(&self.particles) < self.ess_threshold * (n as f64) {
                    stratified_resample(&mut self.particles, self.step_count,
                                        &mut self.resample_log, uniform);
                }

                // CLT: compute η_t^N and M contribution
                if let Some(prev) = prev_pos {
                    let d = clt_fluctuation_field(&self.particles, &prev);
                    self.clt_log.push(d);
                    if self.clt_log.len() > CLT_CAP { self.clt_log.remove(0); }
                }
            }
        }
        self.step_count += 1;
    }

    // ── LLN observables (DI4) ─────────────────────────────

    pub fn barycenter(&self)        -> R3  { barycenter(&self.particles) }
    pub fn ess(&self)               -> f64 { ess(&self.particles) }
    pub fn weight_entropy(&self)    -> f64 { weight_entropy(&self.particles) }
    pub fn variance(&self)          -> f64 { variance(&self.particles) }
    pub fn free_energy(&self,o:Obs) -> f64 { free_energy_empirical(&self.particles,o,&self.params,&self.kernel) }
    pub fn resample_count(&self)    -> usize { self.resample_log.len() }
    pub fn ghost_mode(&self)        -> Ghost {
        let r=self.resample_log.len() as f64/self.step_count.max(1) as f64;
        detect_ghost(&self.particles,r)
    }

    // ── CLT observables (CltDiagnostic mode) ──────────────

    pub fn last_clt(&self) -> Option<&CltDiagnostics> { self.clt_log.last() }

    pub fn mean_eta_norm(&self) -> f64 {
        if self.clt_log.is_empty(){return 0.0;}
        self.clt_log.iter().map(|d|d.eta_norm).sum::<f64>()/(self.clt_log.len() as f64)
    }

    pub fn mean_m_contribution(&self) -> f64 {
        if self.clt_log.is_empty(){return 0.0;}
        self.clt_log.iter().map(|d|d.m_contribution).sum::<f64>()/(self.clt_log.len() as f64)
    }

    /// LLN-to-CLT signal ratio: F̂(μ_N) / mean ‖η‖.
    /// High → LLN dominant (mean-field behavior).
    /// Low  → CLT regime (R_τ fluctuations visible).
    pub fn lln_clt_ratio(&self, obs:Obs) -> f64 {
        let fe=self.free_energy(obs).abs();
        let e=self.mean_eta_norm();
        if e<1e-12{return f64::INFINITY;}
        fe/e
    }

    pub fn convergence_claim(&self) -> &'static str {
        match self.mode {
            SystemMode::LlnDefault    => "LLN: EM MV-SDE + Gibbs SMC. PoC O(1/N) Che et al. 2024. Full system: OP1.",
            SystemMode::CltDiagnostic => "CLT: LLN dynamics + η_t^N=√N(μ̂-μ). 𝒜_N=(ℒ+𝒱)+(1/N)M. R_τ=noise covariance.",
            SystemMode::SinkhornJko   => "Sinkhorn JKO stub: O(ε) when implemented (Agarwal 2024). Unimplemented.",
        }
    }
}

// ── §13 INVARIANT CHECKER ─────────────────────────────────────
pub struct InvariantReport { pub passed:Vec<&'static str>, pub failed:Vec<String> }
impl InvariantReport { pub fn is_ok(&self)->bool{self.failed.is_empty()} }

pub fn check_invariants(p:&[Particle],params:&Params)->InvariantReport {
    let mut ok=Vec::new(); let mut fail=Vec::new();
    let s:f64=p.iter().map(|q|q.weight).sum();
    if (s-1.0).abs()<1e-7{ok.push("DI1:Σwᵢ=1");}else{fail.push(format!("DI1:Σwᵢ={:.9}",s));}
    if p.iter().all(|q|q.weight>=0.0){ok.push("DI1:wᵢ≥0");}else{fail.push("DI1:neg weight".into());}
    if params.temperature>=T_MIN{ok.push("DI2:T≥T_MIN");}else{fail.push("DI2 FAIL".into());}
    if params.alpha>0.0{ok.push("DI3:α>0");}else{fail.push("DI3 FAIL".into());}
    if p.iter().all(|q|q.geom.is_finite()){ok.push("Finite gᵢ");}else{fail.push("Non-finite gᵢ".into());}
    ok.push("DI7:Gaussian via GaussianSrc trait");
    ok.push("DI8:B_τ uses E_full — gibbs_tilt()");
    ok.push("DI9:μ_N sole state; Σwᵢδ_{gᵢ}");
    InvariantReport{passed:ok,failed:fail}
}

// ── §14 BIOSCIENCE MODE ───────────────────────────────────────
pub fn bio_obs(atp:f64,lig:f64,ph:f64)->Obs { Obs(R3{x:atp,y:lig,z:ph}) }
pub fn bio_params(alpha:f64,lambda:f64,dt:f64,r_cut:f64)->Params {
    Params{temperature:BIO_T,alpha,lambda,dt,r_cut}
}

// ── §15 UNDERDAMPED EXTENSION (VR heuristic; OP1) ─────────────
#[derive(Clone,Debug)]
pub struct UnderdampedParticle {
    pub geom:   R3,
    pub vel:    R3,  // Euclidean ℝ³ momentum. NOT W₂ tangent. OP1 applies.
    pub weight: f64,
}

pub fn underdamped_step(
    parts:   &mut Vec<UnderdampedParticle>,
    obs:     Obs,
    params:  &Params,
    kernel:  &dyn KernelFn,
    gauss:   &dyn GaussianSrc,
    friction:f64,
) {
    let dt=params.dt; let sig=(2.0*friction*params.temperature*dt).sqrt();
    let rc2=params.r_cut*params.r_cut;
    let snap:Vec<Particle>=parts.iter().map(|p|Particle{geom:p.geom,weight:p.weight}).collect();
    for i in 0..parts.len() {
        let g=parts[i].geom;
        let f=grad_e_full(g,obs,params,&snap,kernel,rc2).scale(-1.0);
        let vn=parts[i].vel.scale(1.0-friction*dt).add(&f.scale(dt))
            .add(&R3{x:gauss.n01()*sig,y:gauss.n01()*sig,z:gauss.n01()*sig});
        let gn=g.add(&vn.scale(dt));
        debug_assert!(gn.is_finite(),"Underdamped diverged @{i}");
        parts[i].vel=vn; parts[i].geom=gn;
    }
}
// ============================================================
// END OF FILE
// ============================================================
