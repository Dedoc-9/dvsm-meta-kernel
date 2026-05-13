// ============================================================================
// DVSM-π — HYBRID PROJECTED GRAPH SYSTEM WITH SYMBOLIC CONSTRAINT LIFT
// Author: Daniel J. dillberg
// ============================================================================
// Single-file research simulator
// Layers:
//   1. Projected nonlinear graph dynamics
//   2. Jet reconstruction (observational only)
//   3. Active-set symbolic lift
//   4. Switching entropy + saturation metrics
//   5. Dwell-time + complexity analysis
//   6. Regime classifier
// ============================================================================

use std::collections::HashMap;

// ============================================================================
// CORE STATE
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub x: f64,
}

// ============================================================================
// JET OBSERVABLE (DERIVED ONLY)
// ============================================================================

#[derive(Clone, Copy, Debug)]
pub struct Jet {
    pub v: f64,
    pub a: f64,
    pub j: f64,
}

// ============================================================================
// GRAPH
// ============================================================================

#[derive(Clone)]
pub struct Graph {
    pub edges: Vec<(usize, usize)>,
}

// ============================================================================
// PARAMETERS
// ============================================================================

#[derive(Clone, Copy)]
pub struct Params {
    pub eta: f64,
    pub gamma: f64,
    pub coupling: f64,
}

// ============================================================================
// BOUNDS (CONSTRAINT MANIFOLD M)
// ============================================================================

#[derive(Clone, Copy)]
pub struct Bounds {
    pub x_min: f64,
    pub x_max: f64,
}

// ============================================================================
// SYMBOLIC ACTIVE SET
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActiveSet {
    Interior,   // ∅
    Upper,      // Σ+
    Lower,      // Σ-
}

// ============================================================================
// KERNEL
// ============================================================================

#[inline(always)]
fn kernel(x: f64, sigma: f64, eta: f64) -> f64 {
    x + eta * (sigma - x)
}

// ============================================================================
// EXCITATION
// ============================================================================

#[inline(always)]
fn excitation(sigma: f64, x: f64) -> f64 {
    sigma - x
}

// ============================================================================
// PROJECTION Π_M
// ============================================================================

#[inline(always)]
fn project(x: f64, b: &Bounds) -> f64 {
    x.clamp(b.x_min, b.x_max)
}

// ============================================================================
// ACTIVE SET LIFT
// ============================================================================

fn active_set(x: f64, b: &Bounds, eps: f64) -> ActiveSet {
    if (x - b.x_max).abs() < eps {
        ActiveSet::Upper
    } else if (x - b.x_min).abs() < eps {
        ActiveSet::Lower
    } else {
        ActiveSet::Interior
    }
}

// ============================================================================
// JET RECONSTRUCTION
// ============================================================================

fn jet(x2: f64, x1: f64, x0: f64) -> Jet {
    let v = x0 - x1;
    let v_prev = x1 - x2;
    let a = v - v_prev;
    let j = a - v_prev;
    Jet { v, a, j }
}

// ============================================================================
// ENTROPY
// ============================================================================

fn entropy(hist: &[ActiveSet]) -> f64 {
    let mut counts = HashMap::new();
    for h in hist {
        *counts.entry(*h).or_insert(0usize) += 1;
    }

    let n = hist.len() as f64;
    let mut h = 0.0;

    for (_, c) in counts {
        let p = c as f64 / n;
        h -= p * p.log2();
    }

    h
}

// ============================================================================
// SATURATION MASS μ(t)
// ============================================================================

fn saturation(hist: &[ActiveSet]) -> f64 {
    let mut active = 0usize;
    for h in hist {
        if *h != ActiveSet::Interior {
            active += 1;
        }
    }
    active as f64 / hist.len() as f64
}

// ============================================================================
// DWELL TIME
// ============================================================================

fn dwell_times(hist: &[ActiveSet]) -> Vec<usize> {
    let mut times = vec![];
    if hist.is_empty() { return times; }

    let mut prev = hist[0];
    let mut count = 1;

    for h in hist.iter().skip(1) {
        if *h == prev {
            count += 1;
        } else {
            times.push(count);
            count = 1;
            prev = *h;
        }
    }
    times.push(count);
    times
}

// ============================================================================
// SIMPLE LZ-LIKE COMPLEXITY (FINGERPRINT APPROX)
// ============================================================================

fn lz_complexity(hist: &[ActiveSet]) -> f64 {
    let mut dict = HashMap::new();
    let mut complexity = 1usize;

    let mut w = vec![];

    for h in hist {
        w.push(*h);
        if !dict.contains_key(&w) {
            dict.insert(w.clone(), true);
            complexity += 1;
            w.clear();
        }
    }

    complexity as f64 / hist.len().max(1) as f64
}

// ============================================================================
// GRAPH COUPLING
// ============================================================================

fn coupling(graph: &Graph, states: &[State], i: usize, p: &Params) -> f64 {
    let mut sum = 0.0;
    for &(a, b) in &graph.edges {
        if a == i {
            sum += p.coupling * (states[b].x - states[a].x);
        }
    }
    sum
}

// ============================================================================
// EVOLUTION
// ============================================================================

fn evolve(x: f64, sigma: f64, cx: f64, p: &Params) -> f64 {
    let k = kernel(x, sigma + cx, p.eta);
    let u = p.gamma * excitation(sigma, x);
    k + u
}

// ============================================================================
// REGIME CLASSIFIER
// ============================================================================

fn classify(h: f64, mu: f64, lz: f64) -> &'static str {
    if mu < 0.1 {
        "interior attractor"
    } else if mu > 0.9 && h < 0.5 {
        "locked-to-boundary"
    } else if h > 1.2 && lz > 0.6 {
        "high-entropy switching"
    } else if lz < 0.3 {
        "periodic boundary orbit"
    } else {
        "mixed boundary contact"
    }
}

// ============================================================================
// SYSTEM
// ============================================================================

pub struct DVSMGraph {
    pub states: Vec<State>,
    pub history: Vec<Vec<State>>,
    pub graph: Graph,
    pub params: Params,
    pub bounds: Bounds,
    pub active_history: Vec<ActiveSet>,
}

impl DVSMGraph {

    pub fn step(&mut self, sigma: f64) -> HashMap<String, f64> {

        let snapshot = self.states.clone();
        let mut next = snapshot.clone();

        let prev2 = self.history
            .last()
            .cloned()
            .unwrap_or(snapshot.clone());

        let prev1 = snapshot.clone();

        self.active_history.clear();

        for i in 0..snapshot.len() {

            let cx = coupling(&self.graph, &snapshot, i, &self.params);
            let x_raw = evolve(snapshot[i].x, sigma, cx, &self.params);
            let x_proj = project(x_raw, &self.bounds);

            next[i].x = x_proj;

            let a = active_set(x_proj, &self.bounds, 1e-6);
            self.active_history.push(a);
        }

        let h = entropy(&self.active_history);
        let mu = saturation(&self.active_history);
        let lz = lz_complexity(&self.active_history);

        let regime = classify(h, mu, lz);

        let mut out = HashMap::new();
        out.insert("entropy".to_string(), h);
        out.insert("saturation".to_string(), mu);
        out.insert("lz".to_string(), lz);
        out.insert("regime_code".to_string(), match regime {
            "interior attractor" => 0.0,
            "locked-to-boundary" => 1.0,
            "mixed boundary contact" => 2.0,
            "high-entropy switching" => 3.0,
            "periodic boundary orbit" => 4.0,
            _ => 5.0,
        });

        self.history.push(snapshot);
        self.states = next;

        out
    }
}

// ============================================================================
// ADVERSARY
// ============================================================================

pub struct Adversary {
    pub strength: f64,
}

impl Adversary {
    pub fn perturb(&self, sigma: f64, t: usize) -> f64 {
        sigma + (t as f64).sin() * self.strength
    }
}

// ============================================================================
// STRESS TEST
// ============================================================================

pub fn stress_test(sys: &mut DVSMGraph, adv: Adversary, steps: usize, base: f64) {
    for t in 0..steps {
        let sigma = adv.perturb(base, t);
        let _ = sys.step(sigma);
    }
}
