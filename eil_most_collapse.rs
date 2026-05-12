// ============================================================================
// MOST CLASS — MOLECULAR SOLAR THERMAL COLLAPSE DOMAIN
// (NBD ⇄ QC PHOTOSWITCHING AS IRREVERSIBILITY SIGNAL)
// Autthor: Daniel J. Dillberg
// ============================================================================

use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// 1. COLLAPSE CLASS MARKER (TYPE-ISOLATED DOMAIN)
// ---------------------------------------------------------------------------

pub struct MOSTClass;

// Ensures no mixing with Kirsch / Bubble / other collapse domains
pub trait CollapseClass {}

impl CollapseClass for MOSTClass {}

// ---------------------------------------------------------------------------
// 2. ONTOLOGICALLY SEALED VAJRA INSTANCE
// ---------------------------------------------------------------------------

pub struct Vajra<C: CollapseClass> {
    pub _m: PhantomData<C>,
    pub anchor: f64, // local reference ONLY (quantum yield baseline)
}

impl Vajra<MOSTClass> {

    /// Anchor-deviation monitor for irreversibility signature only
    ///
    /// IMPORTANT:
    /// - NOT chemistry-aware
    /// - NOT energy-aware
    /// - NOT state-reconstructing
    /// - ONLY scalar deviation analysis
    pub fn evaluate(trace: &TraceLog, anchor: f64) -> f64 {

        let mut acc = 0.0;

        for v in &trace.values {
            acc += (v - anchor).abs();
        }

        acc / trace.values.len().max(1) as f64
    }
}

// ---------------------------------------------------------------------------
// 3. QUANTUM YIELD PROJECTION FUNCTOR (Φ_QC)
// ---------------------------------------------------------------------------

pub struct QuantumYieldProjection;

impl QuantumYieldProjection {

    /// Non-invertible collapse of excited-state dynamics
    ///
    /// External reality:
    ///   NBD (S1 excited manifold)
    ///   → CI (conical intersection)
    ///   → QC (S0 high-energy isomer)
    ///
    /// DVSM sees ONLY:
    ///   scalar irreversibility residue
    pub fn apply(excitation_trace: &[f64]) -> f64 {

        // Lossy compression of trajectory into scalar "yield residue"
        // deliberately destroys all structural information

        let peak = excitation_trace
            .iter()
            .cloned()
            .fold(0.0, f64::max);

        let mean = excitation_trace.iter().sum::<f64>()
            / excitation_trace.len().max(1) as f64;

        // irreversible collapse encoding (NOT physically invertible)
        (peak.atan() - mean.atan()).abs()
    }
}

// ---------------------------------------------------------------------------
// 4. TRACE STRUCTURE (EPISODIC OBSERVATION ONLY)
// ---------------------------------------------------------------------------

pub struct TraceLog {
    pub values: Vec<f64>,
}

// ---------------------------------------------------------------------------
// 5. MOST SYSTEM WRAPPER (EPISODIC SOLAR-FUEL MONITOR ONLY)
// ---------------------------------------------------------------------------

pub struct MOSTSystem {
    pub v: f64, // epistemic scalar placeholder (NOT molecular state)
}

impl MOSTSystem {

    /// Single observation tick:
    /// - receives irreversibility residue only
    /// - no molecular reconstruction allowed
    pub fn step(&mut self, trace: &mut TraceLog, signal: f64) {

        // ontic update (blind scalar evolution)
        self.v = (self.v + signal).fract();

        // observation collapse
        trace.values.push(self.v);
    }
}

// ---------------------------------------------------------------------------
// 6. LEAK INTERPRETATION GUARD (DOMAIN ISOLATION)
// ---------------------------------------------------------------------------

pub enum MOSTLeakSignature {
    YieldInstability,
    IrreversibilitySpike,
    NoiseDominatedSwitching,
}

pub struct MOSTLeakAnalyzer;

impl MOSTLeakAnalyzer {

    pub fn classify(trace: &TraceLog) -> Option<MOSTLeakSignature> {

        if trace.values.windows(2).any(|w| (w[1] - w[0]).abs() < f64::EPSILON) {
            return Some(MOSTLeakSignature::IrreversibilitySpike);
        }

        if trace.values.iter().any(|v| v.is_nan() || v.is_infinite()) {
            return Some(MOSTLeakSignature::NoiseDominatedSwitching);
        }

        let variance = {
            let mean = trace.values.iter().sum::<f64>() / trace.values.len().max(1) as f64;
            trace.values.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
        };

        if variance > 10.0 {
            return Some(MOSTLeakSignature::YieldInstability);
        }

        None
    }
}
