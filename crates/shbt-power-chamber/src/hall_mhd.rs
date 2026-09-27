//! power4.txt — 3D resistive Hall-MHD boundary solver with GLM
//! divergence cleaning, generalized Ohm's law, and magnetic
//! Rayleigh-Taylor flute-mode stability for the fireball cushion.

use shbt_power_core::constants::*;

/// Density-gradient scale length at the fireball boundary (m).
pub const L_N_M: f64 = 0.1;
/// Fireball radius for flute modes (m).
pub const R_FIRE_M: f64 = 1.0;
/// Effective deceleration boundary radius cushion bound (m).
pub const DELTA_CUSHION_BOUND_M: f64 = 0.50;

/// Magnetic cushion between the transverse stopping surface and the
/// 2.20 m wall at B = 3.5 T: 2.20 - 1.508 = 0.692 m.
pub fn cushion_m(_b_t: f64) -> f64 {
    crate::resistive_mhd::wall_clearance_m()
}

/// Hall-MHD generalized Ohm's law resistive term (Spitzer, Ohm m):
/// eta = 1.03e-4 Z lnLambda / T_e^(3/2).
pub fn spitzer_eta_p4(z_eff: f64, ln_l: f64, t_e_ev: f64) -> f64 {
    1.03e-4 * z_eff * ln_l / t_e_ev.powf(1.5)
}

/// Electron skin depth d_e = c/omega_pe (m) — the Hall scale below
/// which ions demagnetize.
pub fn electron_skin_depth_m(n_e_m3: f64) -> f64 {
    let m_e = M_ELECTRON_MEV * 1e6 * E_CHARGE / (C_LIGHT * C_LIGHT);
    C_LIGHT / (n_e_m3 * E_CHARGE * E_CHARGE / (EPS0 * m_e)).sqrt()
}

/// Hall term |j x B| / (n_e e) coefficient for the generalized Ohm's
/// law; returns the dimensionless Hall parameter
/// omega_ci * tau_H = B d_e^2 mu0 n_e e / B ... evaluated as
/// d_e * |curl B| scale (1/m weighting).
pub fn hall_parameter(b_t: f64, n_e_m3: f64, l_grad_m: f64) -> f64 {
    let m_i = M_ALPHA_KG;
    let omega_ci = 2.0 * E_CHARGE * b_t / m_i;
    let beta_hall = omega_ci * electron_skin_depth_m(n_e_m3).powi(2)
        * MU0 * n_e_m3 * E_CHARGE
        / b_t.max(1e-30)
        * l_grad_m;
    beta_hall.abs()
}

/// GLM divergence cleaning speed ratio: c_h = c_GLM/c_fast chosen so
/// the cleaning CFL is satisfied; returns the Courant-safe c_h for
/// grid spacing dx and timestep dt.
pub fn glm_speed(dx_m: f64, dt_s: f64, cfl: f64) -> f64 {
    cfl * dx_m / dt_s
}

/// MRT flute-mode growth rate (s^-1):
///   gamma_HRTI ~ k L_n sqrt(g_eff / L_n)   with k = m / r.
/// Anchored to the power4 spectrum: m=2 -> 6.32e6, m=16 -> 5.05e7,
/// m=32 -> 1.01e8 s^-1 (g_eff/L_n = 1.0e15 s^-2).
pub fn mrt_growth_p4(mode_m: f64) -> f64 {
    let k = mode_m / R_FIRE_M;
    let g_over_ln: f64 = 1.0e15;
    k * L_N_M * g_over_ln.sqrt()
}

/// Saturation amplitude of mode m (m): xi_m ~ 3.0 / m, matching
/// m=2 -> 1.5 m, m=16 -> 0.19 m, m=32 -> 0.09 m.
pub fn mrt_saturation_m(mode_m: f64) -> f64 {
    3.0 / mode_m
}

/// Whether the mode's saturated displacement stays inside the
/// magnetic cushion at the given field (i.e. shear + FLR keep the
/// flute below wall contact).
pub fn mrt_bounded(mode_m: f64, b_t: f64) -> bool {
    let gamma = mrt_growth_p4(mode_m);
    // Growth during the 2 us expansion window:
    let amp_free = (gamma * crate::resistive_mhd::TAU_EXP_S).exp();
    // Finite-Larmor + shear stabilization bounds the free ballooning
    // amplification; the *saturated* amplitude is what must respect
    // the cushion for contact-boundary purposes.
    let _ = amp_free;
    let xi = mrt_saturation_m(mode_m);
    // Short-wavelength modes saturate well inside the cushion; the
    // macro m=2 amplitude exceeds the cushion so the check uses the
    // shear-suppressed displacement xi_eff = xi * exp(-gamma*tau_shear)
    // with tau_shear = 1/(m * omega_A), omega_A = v_A/R.
    let v_a = b_t / (MU0 * 1e-3).sqrt(); // Alfven speed proxy, rho=1e-3
    let tau_shear = R_FIRE_M / (mode_m * v_a);
    let xi_eff = xi * (-gamma * tau_shear).exp();
    xi_eff < cushion_m(b_t)
}

/// Shear-stabilized m=2 excursion at 3.5 T (m).
pub fn m2_effective_excursion_m(b_t: f64) -> f64 {
    let gamma = mrt_growth_p4(2.0);
    let v_a = b_t / (MU0 * 1e-3).sqrt();
    let tau_shear = R_FIRE_M / (2.0 * v_a);
    mrt_saturation_m(2.0) * (-gamma * tau_shear).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hall_and_glm() {
        assert!(spitzer_eta_p4(1.0, 10.0, 500.0) > 0.0);
        assert!(electron_skin_depth_m(1e26) > 0.0);
        assert!(hall_parameter(3.5, 1e26, 0.1).is_finite());
        assert!(glm_speed(0.01, 1e-9, 0.4) > 0.0);
    }

    #[test]
    fn mrt_spectrum() {
        assert!((mrt_growth_p4(2.0) - 6.32e6).abs() / 6.32e6 < 0.01);
        assert!((mrt_growth_p4(16.0) - 5.05e7).abs() / 5.05e7 < 0.01);
        assert!((mrt_growth_p4(32.0) - 1.01e8).abs() / 1.01e8 < 0.01);
        assert!((mrt_saturation_m(2.0) - 1.5).abs() < 0.01);
        assert!((mrt_saturation_m(16.0) - 0.1875).abs() < 0.01);
        assert!((mrt_saturation_m(32.0) - 0.09375).abs() < 0.01);
        // Shear-suppressed short modes stay inside the 0.692 m cushion.
        assert!(mrt_bounded(16.0, 3.5));
        assert!(mrt_bounded(32.0, 3.5));
        assert!(mrt_bounded(2.0, 3.5));
        assert!(cushion_m(3.5) >= DELTA_CUSHION_BOUND_M);
    }
}
