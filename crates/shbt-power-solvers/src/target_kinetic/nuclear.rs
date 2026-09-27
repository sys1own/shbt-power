//! Solbrig transmission kernel and Breit-Wigner multi-channel cross sections.

use crate::target_kinetic::types::*;
use core::f64::consts::PI;

pub struct NuclearCrossSectionModel {
    debye_temp_k: f64,
}

impl NuclearCrossSectionModel {
    pub const fn new(debye_temp_k: f64) -> Self {
        Self { debye_temp_k }
    }

    #[inline]
    pub fn effective_lattice_temp_k(&self, t_plasma_k: f64) -> f64 {
        let theta = self.debye_temp_k;
        let t_eff_zero = (3.0 / 8.0) * theta;
        if t_plasma_k <= 1.0 {
            return t_eff_zero;
        }
        let tau = t_plasma_k / theta;
        t_eff_zero * (1.0 + 8.0 * tau.powi(4) * self.debye_integral_dimless(1.0 / tau))
    }

    fn debye_integral_dimless(&self, upper_limit: f64) -> f64 {
        if upper_limit > 50.0 {
            return core::f64::consts::PI.powi(4) / 15.0;
        }
        let steps = 100;
        let dx = upper_limit / (steps as f64);
        let mut sum = 0.0;
        for i in 1..=steps {
            let x = (i as f64 - 0.5) * dx;
            let integrand = x.powi(3) / (x.exp() - 1.0);
            sum += integrand * dx;
        }
        sum
    }

    pub fn bare_breit_wigner_c12(
        &self,
        e_cm_ev: f64,
        e_res_ev: f64,
        gamma_in_ev: f64,
        gamma_out_ev: f64,
        gamma_tot_ev: f64,
        spin_factor: f64,
    ) -> f64 {
        if e_cm_ev <= 1.0e-3 {
            return 0.0;
        }
        let mu = (MASS_PROTON * MASS_B11) / (MASS_PROTON + MASS_B11);
        let hbar = 1.054_571_817e-34;
        let k_sq = (2.0 * mu * (e_cm_ev * ELEMENTARY_CHARGE)) / (hbar * hbar);
        let term1 = (PI / k_sq) * spin_factor;
        let num = gamma_in_ev * gamma_out_ev;
        let denom = (e_cm_ev - e_res_ev).powi(2) + 0.25 * gamma_tot_ev.powi(2);
        term1 * (num / denom)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn solbrig_broadened_cross_section(
        &self,
        e_ev: f64,
        t_ion_ev: f64,
        e_res_ev: f64,
        gamma_in: f64,
        gamma_out: f64,
        gamma_tot: f64,
        g_j: f64,
    ) -> f64 {
        let t_k = t_ion_ev * 11_604.518;
        let t_eff_k = self.effective_lattice_temp_k(t_k);
        let delta = ((4.0 * MASS_PROTON * e_ev * BOLTZMANN_K * t_eff_k)
            / (MASS_PROTON + MASS_B11))
            .sqrt();

        if delta < 1.0e-6 * e_ev {
            return self.bare_breit_wigner_c12(e_ev, e_res_ev, gamma_in, gamma_out, gamma_tot, g_j);
        }

        let n_quad = 120;
        let e_min = (e_ev - 4.0 * delta).max(1.0);
        let e_max = e_ev + 4.0 * delta;
        let de = (e_max - e_min) / (n_quad as f64);
        let mut integral = 0.0;

        for step in 0..n_quad {
            let ep = e_min + (step as f64 + 0.5) * de;
            let sigma_bare = self.bare_breit_wigner_c12(ep, e_res_ev, gamma_in, gamma_out, gamma_tot, g_j);
            let kernel = ((ep / e_ev).sqrt())
                * ((-((ep.sqrt() - e_ev.sqrt()).powi(2)) / (delta * delta)).exp()
                    - (-((ep.sqrt() + e_ev.sqrt()).powi(2)) / (delta * delta)).exp());
            integral += sigma_bare * kernel * de;
        }

        integral / (delta * PI.sqrt())
    }
}