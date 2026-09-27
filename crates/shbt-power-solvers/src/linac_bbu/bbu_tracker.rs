// crates/shbt-power-solvers/src/linac_bbu/bbu_tracker.rs

use std::vec::Vec;

use core::f64::consts::PI;

pub const NUM_BUNCHES: usize = 2500;
pub const T_RF: f64 = 175.070e-12; // 175.070 ps bunch spacing
pub const C_LIGHT: f64 = 299_792_458.0;
pub const E_CHARGE: f64 = 1.602_176_634e-19;
pub const M_E_EV: f64 = 0.510_998_950e6;

#[derive(Clone, Copy, Debug, Default)]
pub struct PhaseSpaceState {
    pub x: f64,
    pub px: f64,
    pub energy_ev: f64,
}

pub struct LinacElement {
    pub length: f64,
    pub beta_in: f64,
    pub beta_out: f64,
    pub alpha_in: f64,
    pub alpha_out: f64,
    pub phase_advance: f64,
    pub hom_r_over_q: f64,
    pub hom_freq: f64,
    pub hom_q_ext: f64,
}

pub struct BbuTracker {
    elements: Vec<LinacElement>,
}

impl BbuTracker {
    pub fn new(elements: Vec<LinacElement>) -> Self {
        Self { elements }
    }

    /// Evaluates dipole wakefield Green function W_perp(tau)
    #[inline(always)]
    pub fn wake_green_function(elem: &LinacElement, tau: f64) -> f64 {
        if tau <= 0.0 {
            return 0.0;
        }
        let omega = 2.0 * PI * elem.hom_freq;
        let damping = (-omega * tau / (2.0 * elem.hom_q_ext)).exp();
        (elem.hom_r_over_q * omega * omega / C_LIGHT) * damping * (omega * tau).sin()
    }

    /// Tracks 2,500 bunches through linac lattice and returns final phase space states
    #[allow(clippy::needless_range_loop)]
    pub fn track_macro_burst(
        &self,
        initial_states: &[PhaseSpaceState; NUM_BUNCHES],
        bunch_charge: f64,
    ) -> [PhaseSpaceState; NUM_BUNCHES] {
        let mut states = *initial_states;
        let mut wake_history = [0.0f64; NUM_BUNCHES];

        for elem in &self.elements {
            let cos_phi = elem.phase_advance.cos();
            let sin_phi = elem.phase_advance.sin();
            let sqrt_beta = (elem.beta_out / elem.beta_in).sqrt();

            let m11 = sqrt_beta * (cos_phi + elem.alpha_in * sin_phi);
            let m12 = (elem.beta_in * elem.beta_out).sqrt() * sin_phi;
            let m21 = -(1.0 + elem.alpha_in * elem.alpha_out) / (elem.beta_in * elem.beta_out).sqrt() * sin_phi
                + (elem.alpha_in - elem.alpha_out) / (elem.beta_in * elem.beta_out).sqrt() * cos_phi;
            let m22 = (1.0 / sqrt_beta) * (cos_phi - elem.alpha_out * sin_phi);

            // Accumulate transverse wake kicks across bunches
            for k in 0..NUM_BUNCHES {
                let mut kick_sum = 0.0;
                for j in 0..k {
                    let tau = ((k - j) as f64) * T_RF;
                    let w_perp = Self::wake_green_function(elem, tau);
                    kick_sum += w_perp * states[j].x;
                }
                wake_history[k] = kick_sum;

                // Apply unperturbed transfer matrix
                let x_new = m11 * states[k].x + m12 * states[k].px;
                let px_temp = m21 * states[k].x + m22 * states[k].px;

                // Apply transverse wake kick
                let gamma_k = states[k].energy_ev / M_E_EV;
                let kick = (E_CHARGE * bunch_charge / (gamma_k * M_E_EV * 1.0e-6)) * elem.length * wake_history[k];
                let px_new = px_temp + kick;

                states[k].x = x_new;
                states[k].px = px_new;
            }
        }

        states
    }
}