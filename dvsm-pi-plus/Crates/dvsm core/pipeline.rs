// src/pipeline.rs — 11-stage canonical execution
// Calls math.rs primitives. No inline arithmetic except classification.
use crate::constants::*;
use crate::math::*;
use crate::manifold::*;
use crate::containment::*;
use crate::ghost::*;
use crate::trace::*;
use crate::core::DvsmCore;

impl DvsmCore {
    pub fn init_kappa(&mut self) {
        let r = self.r as usize;
        let mut i = 0;
        while i < r { let mut j = 0;
            while j < r {
                self.kappa[i*R+j] = sin_approx((i as f32)*KAPPA_A - (j as f32)*KAPPA_B);
                j += 1;
            } i += 1;
        }
    }

    pub fn step(&mut self, input: &[f32]) -> TraceFrame {
        let n = self.n as usize;
        let r = self.r as usize;
        let in_n = if input.len() < n { input.len() } else { n };

        // 1. CONTAINMENT
        let violation = check_containment(&self.z, r);
        if violation { self.contain_fails += 1; } else { self.contain_fails = 0; }
        let killed = self.contain_fails >= KILL_K;
        if killed {
            self.rebirth_mode = select_rebirth(&self.z, &self.s, r);
            let mut k = 0; while k < r { self.z[k] = 0.0; k += 1; }
            self.alive = 0;
        }
        if self.alive == 0 {
            rebirth(&mut self.z, &mut self.s, &mut self.v, &mut self.omega,
                    &mut self.w, self.rebirth_mode, self.frame, r);
            self.alive = 1; self.contain_fails = 0; self.frames_since_rebirth = 0;
        }

        // 2. PROJECTION
        let r_norm = project(&self.w, input, in_n, r,
                             &mut self.c, &mut self.p, &mut self.res);

        // 3. LIE EVOLUTION
        lie_step(&mut self.z, &self.s, &self.kappa, r);

        // 4. EMA (frozen during containment)
        if self.contain_fails == 0 {
            ema_update(&mut self.s, &self.z, r);
        }

        // 5. BASIS ADAPT
        basis_adapt(&mut self.w, &self.res, &self.c, in_n, r, r_norm);

        // 6. MANIFOLD MAINTAIN
        let drift = stiefel_drift(&self.w, r);
        if drift > 1e-6 { orthonormalize(&mut self.w, r); }
        sign_lock(&mut self.w, &self.w_prev, n, r);

        // 7. VELOCITY
        velocity_update(&mut self.v, &mut self.x, &self.res, &self.s, in_n);

        // 8. OMEGA
        omega_update(&mut self.omega, &self.z, r);

        // 9. CLASSIFY
        let z_n = norm_safe(&self.z, r);
        let s_n = norm_safe(&self.s, r);
        let stress = s_n / z_n;
        let mut in_n2 = 0.0f32; let mut i = 0;
        while i < in_n { in_n2 += input[i]*input[i]; i += 1; }
        let novelty = r_norm / norm_safe_val(in_n2);
        let drift_safe = if drift != drift { 0.0 } else if drift < 0.0 { 0.0 } else { drift };
        let entropy = spectral_entropy(&self.z, r, self.frames_since_rebirth);
        let o_n = norm_safe(&self.omega, r);
        let denat_ramp = self.rebirth_mode == RebirthMode::HighEntropy
            && self.frames_since_rebirth < RAMP_FRAMES;
        let ghost = classify(stress, novelty, drift_safe, entropy, o_n / z_n, killed, denat_ramp);

        // 10. STATE COMMIT (w_prev AFTER all evolution, BEFORE next sign_lock)
        state_commit(&mut self.w_prev, &self.w,
                     &mut self.frame, &mut self.frames_since_rebirth);

        // 11. EMIT (delta-encoded)
        let emit = should_emit(novelty, self.prev_novelty, killed, self.frame);
        self.prev_novelty = novelty;

        TraceFrame {
            frame: self.frame, stress, novelty, drift: drift_safe,
            entropy, energy: z_n, omega_norm: o_n,
            ghost: ghost as u8, contained: killed as u8,
            emitted: emit as u8, _pad: 0,
        }
    }

    #[inline] pub fn is_vacuum(&self) -> bool { self.alive == 0 }
}
