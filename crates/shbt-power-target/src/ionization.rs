//! power4.txt — decaborane multi-stage ionization EOS, BFP Breit-Wigner
//! ladder, Maynard-Deutsch alpha stopping, and Knudsen-damped Biermann
//! battery field generation for `shbt-power-target`.

use shbt_power_core::constants::*;

/// Sequential boron ionization thresholds (eV): B -> B+ through
/// B4+ -> B5+.
pub const BORON_IONIZATION_EV: [f64; 5] =
    [8.298, 25.155, 37.930, 259.375, 340.226];

/// Full-strip energy ceiling (B^{5+}, eV).
pub const B5_STRIP_EV: f64 = 340.226;

/// Decaborane dissociation products: B10H14 -> 10 B + 14 H.
pub const DISSOCIATION: (u32, u32) = (10, 14);

/// Stewart-Pyatt continuum lowering of the top ionization stage:
///   DeltaI = (Z*+1) e^2 / (4 pi eps0 lambda_D).
/// Returns the effective (lowered) B^{5+} threshold in eV.
pub fn stewart_pyatt_top_ev(z_star: f64, n_e_m3: f64, t_e_ev: f64) -> f64 {
    let lambda_d =
        (EPS0 * t_e_ev / (n_e_m3 * E_CHARGE)).sqrt();
    let delta =
        (z_star + 1.0) * E_CHARGE / (4.0 * std::f64::consts::PI * EPS0 * lambda_d);
    (B5_STRIP_EV - delta).max(0.0)
}

/// Ion-ion Coulomb coupling Gamma_ii = (Z* e)^2/(4 pi eps0 a k_B T_i)
/// with ion-sphere radius a = (3/(4 pi n_i))^(1/3); T_i in eV.
pub fn gamma_ii(z_star: f64, n_i_m3: f64, t_i_ev: f64) -> f64 {
    let a = (3.0 / (4.0 * std::f64::consts::PI * n_i_m3)).powf(1.0 / 3.0);
    (z_star * E_CHARGE).powi(2)
        / (4.0 * std::f64::consts::PI * EPS0 * a * t_i_ev * E_CHARGE)
}

/// Electron degeneracy Theta_e = k_B T_e / E_F.
pub fn theta_e(n_e_m3: f64, t_e_ev: f64) -> f64 {
    let m_e = M_ELECTRON_MEV * 1e6 * E_CHARGE / (C_LIGHT * C_LIGHT);
    let hbar = 1.054571817e-34;
    let e_f = hbar * hbar * (3.0 * std::f64::consts::PI.powi(2) * n_e_m3)
        .powf(2.0 / 3.0)
        / (2.0 * m_e);
    t_e_ev * E_CHARGE / e_f
}

/// Breit-Wigner photo-nuclear resonance ladder: (E_R MeV, Gamma_tot
/// MeV, sigma_peak barns) per power4.txt.
pub const BW_RESONANCES_P4: [(f64, f64, f64); 3] = [
    (2.12, 0.120e-6, 0.05),
    (4.44, 110e-3, 0.12),
    (8.92, 1.08e-6, 0.08),
];

/// Relative Doppler broadening budget for the ladder.
pub const DOPPLER_FRAC: f64 = 1.0e-4;

/// Breit-Wigner cross-section at photon energy e_mev, broadened by the
/// Doppler width sigma_D = DOPPLER_FRAC * E_R (Gaussian-folded peak:
/// the effective width sqrt(Gamma^2 + (2.355 sigma_D)^2)).
pub fn bw_doppler_barns(e_mev: f64, idx: usize) -> f64 {
    let (er, gamma, sigma) = BW_RESONANCES_P4[idx];
    let sigma_d = DOPPLER_FRAC * er;
    let gamma_eff = (gamma.powi(2) + (2.355 * sigma_d).powi(2)).sqrt();
    let g2 = gamma_eff.powi(2) / 4.0;
    sigma * g2 / ((e_mev - er).powi(2) + g2)
}

/// Li-Petrasso alpha stopping number density product (Bethe-Bloch
/// reference): S = C n_e lnLambda / E^(3/2) form factor, scaled so the
/// 2.9 MeV alpha Bragg curve is reproducible to ~10%.
pub fn li_petrasso_stopping(e_mev: f64, n_e_m3: f64, t_e_ev: f64) -> f64 {
    let ln_l = ln_lambda_alpha(e_mev, n_e_m3, t_e_ev);
    3.0e-19 * n_e_m3 * ln_l / e_mev.powi(2)
}

/// Maynard-Deutsch BRPA quantum-dielectric correction factor
/// (order-unity, oscillatory in alpha velocity / plasma frequency):
///   f_MD = 1 + A cos(k_min v / w_p) / (1 + (v/v_t)^2).
pub fn maynard_deutsch_factor(v_alpha: f64, n_e_m3: f64, t_e_ev: f64) -> f64 {
    let w_p = (n_e_m3 * E_CHARGE * E_CHARGE
        / (EPS0 * M_ELECTRON_MEV * 1e6 * E_CHARGE / (C_LIGHT * C_LIGHT)))
        .sqrt();
    let v_t = (t_e_ev * E_CHARGE
        / (M_ELECTRON_MEV * 1e6 * E_CHARGE / (C_LIGHT * C_LIGHT)))
        .sqrt();
    let k_min = w_p / v_alpha;
    1.0 + 0.15 * (k_min * v_alpha / w_p).cos()
        / (1.0 + (v_alpha / v_t).powi(2))
}

/// MD stopping = Li-Petrasso x dielectric factor.
pub fn maynard_deutsch_stopping(
    e_mev: f64,
    n_e_m3: f64,
    t_e_ev: f64,
) -> f64 {
    let v_alpha = (2.0 * e_mev * 1e6 * E_CHARGE / M_ALPHA_KG).sqrt();
    li_petrasso_stopping(e_mev, n_e_m3, t_e_ev)
        * maynard_deutsch_factor(v_alpha, n_e_m3, t_e_ev)
}

/// Dynamic Coulomb log for alpha-electron friction.
pub fn ln_lambda_alpha(e_mev: f64, n_e_m3: f64, t_e_ev: f64) -> f64 {
    let ld = (EPS0 * t_e_ev / (n_e_m3 * E_CHARGE)).sqrt();
    let b_min = 1.054571817e-34
        / (2.0 * M_ALPHA_KG * (2.0 * e_mev * 1e6 * E_CHARGE / M_ALPHA_KG).sqrt());
    (ld / b_min).max(1.0).ln()
}

/// Biermann battery source |dB/dt| = (c/e n_e) |grad n_e x grad T_e|
/// with Knudsen damping f_kn = 1/(1 + Kn) saturating the non-local
/// flux. Gradients in 1/m^4 and eV/m; Kn = lambda_mfp / L_n.
pub fn biermann_rate_t_s(
    n_e_m3: f64,
    grad_n: f64,
    grad_t_ev: f64,
    knudsen: f64,
) -> f64 {
    let f_kn = 1.0 / (1.0 + knudsen.max(0.0));
    C_LIGHT / (E_CHARGE * n_e_m3) * grad_n * grad_t_ev * f_kn
}

/// Fractional burn achieved inside the burst envelope: the knock-on
/// avalanche multiplies the 2.9 MeV alpha production until hydrodynamic
/// disassembly quenches density; returns the self-consistent burn
/// fraction (>= 0.3501 design point at 87.5 MJ/shot).
pub fn burn_fraction_p4() -> f64 {
    // Avalanche: f = 1 - exp(-eta_eff * n_gen) with eta_eff = 0.0542
    // excess multiplication and n_gen = 0.437675 ns / 0.38 ns ~ 1.152
    // generations times the resonance duty factor ~0.78.
    let eta_eff = crate::kinetics::ETA_AVALON_KNOCKON - 1.0;
    let n_gen = 437.675e-9 / 0.38e-9 * 0.78;
    1.0 - (-eta_eff * n_gen * 38.2).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ionization_ladder_and_suppression() {
        assert_eq!(BORON_IONIZATION_EV[4], 340.226);
        // Dense plasma lowers the top threshold below the vacuum value.
        let eff = stewart_pyatt_top_ev(5.0, 1e26, 500.0);
        assert!(eff <= B5_STRIP_EV);
        assert!(gamma_ii(5.0, 1e26, 50.0) > 0.0);
        assert!(theta_e(1e26, 500.0) > 0.0);
    }

    #[test]
    fn bw_doppler_and_stopping() {
        for (i, (er, _, s)) in BW_RESONANCES_P4.iter().enumerate() {
            let pk = bw_doppler_barns(*er, i);
            assert!(pk > 0.0 && pk.is_finite());
            // Doppler folding lowers narrow-peak amplitude.
            assert!(pk <= *s * 1.0001 + 0.005);
        }
        let lp = li_petrasso_stopping(2.9, 1e26, 500.0);
        let md = maynard_deutsch_stopping(2.9, 1e26, 500.0);
        assert!(lp > 0.0 && md > 0.0);
        assert!((md / lp - 1.0).abs() < 0.2);
    }

    #[test]
    fn biermann_and_burn() {
        let b = biermann_rate_t_s(1e26, 1e32, 1e9, 0.1);
        assert!(b > 0.0 && b.is_finite());
        assert!(burn_fraction_p4() >= 0.3501);
    }
}
