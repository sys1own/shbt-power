// Linac multi-bunch dipole wakefield / BBU tracking (Q_ext <= 200, SiC HOM).

pub mod bbu_tracker;
pub mod reduced_order_wake;

pub use bbu_tracker::{BbuTracker, LinacElement, PhaseSpaceState};
pub use reduced_order_wake::{QualityGateStatus, ReducedWakeMatrix};


// ---------- power7 reconciled BBU tracker ----------
// crates/shbt-power-solvers/src/linac_bbu/mod.rs
//
// Reconciled Multi-Bunch Beam Breakup (BBU) and Wakefield Tracking Solver.
// Implements P4 HOM Coupler Damping Optimization (Q_ext = 95.0, tau_d = 3.55 ns).

use std::f64::consts::PI;

pub const BATCH_SIZE: usize = 2500;
pub const SLIDING_WINDOW: usize = 32;

#[derive(Debug, Clone, Copy)]
pub struct DipoleWakefieldParams {
    pub f_hom: f64,
    pub q_ext: f64,
    pub r_over_q: f64,
    pub t_bunch: f64,
    pub bunch_charge: f64,
    pub beam_energy_gev: f64,
    pub beta_x: f64,
    pub linac_length: f64,
}

impl Default for DipoleWakefieldParams {
    fn default() -> Self {
        Self {
            f_hom: 8.512e9,
            q_ext: 95.0,
            r_over_q: 4.50e4,
            t_bunch: 175.070098e-12,
            bunch_charge: 0.200e-9,
            beam_energy_gev: 0.500,
            beta_x: 15.0,
            linac_length: 120.0,
        }
    }
}

pub struct BbuWakefieldTracker {
    params: DipoleWakefieldParams,
    pub tau_d: f64,
    pub omega_hom: f64,
}

impl BbuWakefieldTracker {
    pub fn new(params: DipoleWakefieldParams) -> Self {
        let omega_hom = 2.0 * PI * params.f_hom;
        let tau_d = 2.0 * params.q_ext / omega_hom;
        Self {
            params,
            tau_d,
            omega_hom,
        }
    }

    pub fn transverse_wake(&self, tau: f64) -> f64 {
        if tau <= 0.0 {
            return 0.0;
        }
        let damping = (-tau / self.tau_d).exp();
        let oscillation = (self.omega_hom * tau).sin();
        let c = 2.99792458e8;
        (self.params.r_over_q * self.omega_hom / c) * damping * oscillation
    }

    #[allow(clippy::needless_range_loop)]
    pub fn track_macro_burst(&self, initial_offsets_um: &[f64; BATCH_SIZE]) -> (f64, [f64; BATCH_SIZE]) {
        let mut final_centroids_um = [0.0; BATCH_SIZE];
        let mut max_amplification: f64 = 1.0;

        let q_b = self.params.bunch_charge;
        let e_gev = self.params.beam_energy_gev;
        let beta = self.params.beta_x;
        let l_linac = self.params.linac_length;

        for m in 0..BATCH_SIZE {
            let mut wake_sum = 0.0;
            let window_start = if m >= SLIDING_WINDOW { m - SLIDING_WINDOW } else { 0 };

            for k in window_start..m {
                let tau = (m - k) as f64 * self.params.t_bunch;
                let w_perp = self.transverse_wake(tau);
                wake_sum += final_centroids_um[k] * w_perp;
            }

            let kick_factor = (q_b * 1e-6) / (2.0 * e_gev * 1e9) * beta * l_linac;
            let x_out = initial_offsets_um[m] + kick_factor * wake_sum;
            final_centroids_um[m] = x_out;

            if initial_offsets_um[m].abs() > 1e-9 {
                let amp = (x_out / initial_offsets_um[m]).abs();
                if amp > max_amplification {
                    max_amplification = amp;
                }
            }
        }

        (max_amplification, final_centroids_um)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_p4_hom_damping_time_proof() {
        let tracker = BbuWakefieldTracker::new(DipoleWakefieldParams::default());
        assert!(tracker.tau_d <= 5.50e-9, "P4 HOM Gate Failure: tau_d = {}", tracker.tau_d);
        assert!((tracker.tau_d - 3.55257e-9).abs() < 1e-12, "Incorrect tau_d derivation");
    }

    #[test]
    fn test_bbu_suppression_limit() {
        let tracker = BbuWakefieldTracker::new(DipoleWakefieldParams::default());
        let initial_offsets = [10.0; BATCH_SIZE];
        let (a_bbu, _) = tracker.track_macro_burst(&initial_offsets);
        assert!(a_bbu < 1.184, "Cumulative BBU exceeded threshold: A_BBU = {}", a_bbu);
    }
}