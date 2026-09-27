//! Relativistic Fokker-Planck alpha transport and Li-Petrasso stopping power.

use crate::target_kinetic::types::*;
use core::f64::consts::PI;

pub struct LiPetrassoTransport;

impl LiPetrassoTransport {
    pub fn stopping_power_de_dx(
        alpha_energy_ev: f64,
        n_e: f64,
        t_e_ev: f64,
        n_ions: &[f64],
        z_ions: &[f64],
        m_ions: &[f64],
        t_i_ev: f64,
    ) -> (f64, f64) {
        let e_joules = alpha_energy_ev * ELEMENTARY_CHARGE;
        let gamma_alpha = 1.0 + (e_joules / (MASS_ALPHA * SPEED_OF_LIGHT * SPEED_OF_LIGHT));
        let v_sq = SPEED_OF_LIGHT * SPEED_OF_LIGHT * (1.0 - 1.0 / (gamma_alpha * gamma_alpha));
        let v_alpha = v_sq.sqrt().max(1.0);

        let z_alpha = 2.0;
        let t_e_joules = t_e_ev * ELEMENTARY_CHARGE;
        let v_th_e = (2.0 * t_e_joules / MASS_ELECTRON).sqrt();
        let omega_pe = (n_e * ELEMENTARY_CHARGE.powi(2) / (EPSILON_0 * MASS_ELECTRON)).sqrt();
        let lambda_d = (EPSILON_0 * t_e_joules / (n_e * ELEMENTARY_CHARGE.powi(2))).sqrt();

        // Electron Channel
        let x_alpha_e = v_sq / (v_th_e * v_th_e);
        let psi_e = Self::psi_maxwell(x_alpha_e);
        let b_perp_e = (z_alpha * ELEMENTARY_CHARGE.powi(2))
            / (4.0 * PI * EPSILON_0 * MASS_ELECTRON * v_sq);
        let b_min_e = (b_perp_e.powi(2) + (1.054e-34 / (2.0 * MASS_ELECTRON * v_alpha)).powi(2)).sqrt();
        let ln_lambda_e = (lambda_d / b_min_e).max(2.0).ln();

        let de_dx_e_classical = (z_alpha.powi(2) * ELEMENTARY_CHARGE.powi(2) / (4.0 * PI * EPSILON_0 * v_sq))
            * (omega_pe.powi(2) * ln_lambda_e * psi_e);

        // Maynard-Deutsch dielectric correction
        let k_min = 1.0 / lambda_d;
        let f_md = 1.0 + (0.185 * (k_min * v_alpha / omega_pe).cos()) / (1.0 + v_sq / (v_th_e * v_th_e));
        let de_dx_e = de_dx_e_classical * f_md;

        // Ion Channels
        let mut de_dx_i_total = 0.0;
        for k in 0..n_ions.len() {
            let n_k = n_ions[k];
            let z_k = z_ions[k];
            let m_k = m_ions[k];
            let v_th_k = (2.0 * t_i_ev * ELEMENTARY_CHARGE / m_k).sqrt();
            let x_alpha_k = v_sq / (v_th_k * v_th_k);
            let psi_k = Self::psi_maxwell(x_alpha_k);
            let omega_pk = (n_k * (z_k * ELEMENTARY_CHARGE).powi(2) / (EPSILON_0 * m_k)).sqrt();
            let mu_k = (MASS_ALPHA * m_k) / (MASS_ALPHA + m_k);
            let b_perp_k = (z_alpha * z_k * ELEMENTARY_CHARGE.powi(2))
                / (4.0 * PI * EPSILON_0 * mu_k * v_sq);
            let b_min_k = (b_perp_k.powi(2) + (1.054e-34 / (2.0 * mu_k * v_alpha)).powi(2)).sqrt();
            let ln_lambda_k = (lambda_d / b_min_k).max(2.0).ln();

            let de_dx_k = (z_alpha.powi(2) * ELEMENTARY_CHARGE.powi(2) / (4.0 * PI * EPSILON_0 * v_sq))
                * (omega_pk.powi(2) * ln_lambda_k * psi_k);
            de_dx_i_total += de_dx_k;
        }

        (de_dx_e, de_dx_i_total)
    }

    #[inline]
    fn psi_maxwell(x: f64) -> f64 {
        let x_sqrt = x.sqrt();
        Self::erf(x_sqrt) - (2.0 * x_sqrt / PI.sqrt()) * (-x).exp()
    }

    fn erf(x: f64) -> f64 {
        let t = 1.0 / (1.0 + 0.3275911 * x.abs());
        let poly = t * (0.254829592 + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
        let res = 1.0 - poly * (-x * x).exp();
        if x >= 0.0 { res } else { -res }
    }
}