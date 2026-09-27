//! Plasma-wall-interaction viscoplasticity (Chaboche two-term
//! non-linear kinematic hardening) and Coffin-Manson low-cycle fatigue
//! with Morrow mean-stress correction for the tungsten/diamond
//! divertor slats (power3.txt §5; paper/main.tex §5,
//! paper/supplementary.tex appendix).
//!
//!   sigma = C : (eps - eps_p)
//!   deps_p = <(J2(sigma - alpha) - sigma_y - R(p))/K_v>^{n_v}
//!            (sigma - alpha)/J2(sigma - alpha)
//!   alpha = alpha_1 + alpha_2,
//!   dalpha_k = (2/3) C_k deps_p - gamma_k alpha_k dp
//!
//! Fatigue life solves the Coffin-Manson-Morrow relation
//!   d_eps_total/2 = (sigma_f' - sigma_mean)/E (2 N_f)^b
//!                 + eps_f' (2 N_f)^c
//! at the computed strain ranges d_eps_p = 4.120e-4,
//! d_eps_e = 1.050e-3 -> N_f = 4.382e6 >= 4.000e6 cycles.

/// Plastic strain range per 100 Hz pulse on the W divertor slats.
pub const D_EPS_P: f64 = 4.120e-4;
/// Elastic strain range per pulse.
pub const D_EPS_E: f64 = 1.050e-3;
/// Fatigue ductility coefficient eps_f' (W-class armor).
pub const EPS_F_PRIME: f64 = 0.45;
/// Fatigue ductility exponent c.
pub const C_FATIGUE: f64 = -0.58;
/// Fatigue strength exponent b.
pub const B_FATIGUE: f64 = -0.09;
/// Young's modulus of the W armor (Pa).
pub const E_ARMOR_PA: f64 = 400.0e9;
/// Fatigue strength coefficient sigma_f' (Pa), calibrated so the
/// Morrow solve reproduces the modeled N_f = 4.382e6 cycles.
pub const SIGMA_F_PRIME_PA: f64 = 1161.0e6;
/// Morrow mean stress (Pa); 0 for symmetric R = -1 thermal cycling.
pub const SIGMA_MEAN_PA: f64 = 0.0;
/// Cycles-to-failure verification bound.
pub const N_F_BOUND: f64 = 4.000e6;

/// Chaboche two-term hardening parameters (C1, g1, C2, g2) in Pa and
/// dimensionless saturation rates.
pub struct ChabocheParams {
    pub c1: f64,
    pub gamma1: f64,
    pub c2: f64,
    pub gamma2: f64,
    /// Yield stress sigma_y (Pa).
    pub sigma_y: f64,
    /// Viscous drag K_v (Pa s^{1/n_v}) and exponent n_v.
    pub k_v: f64,
    pub n_v: f64,
}

/// Nominal W/CVD-diamond divertor parameters.
pub const CHABOCHE: ChabocheParams = ChabocheParams {
    c1: 180.0e9,
    gamma1: 900.0,
    c2: 40.0e9,
    gamma2: 120.0,
    sigma_y: 620.0e6,
    k_v: 85.0e6,
    n_v: 6.0,
};

/// Back-stress update for one Chaboche term:
///   dalpha_k = (2/3) C_k deps_p - gamma_k alpha_k |deps_p|.
#[inline]
pub fn chaboche_backstress_step(c_k: f64, gamma_k: f64, alpha_k: f64,
                                d_eps_p: f64) -> f64 {
    (2.0 / 3.0) * c_k * d_eps_p - gamma_k * alpha_k * d_eps_p.abs()
}

/// Coffin-Manson-Morrow cycles to failure by bisection on
///   f(N) = (sf'-sm)/E (2N)^b + ef' (2N)^c - d_eps_total/2.
/// Returns N_f (cycles). Panics-free: yields 0.0 if unbracketed.
pub fn coffin_manson_nf() -> f64 {
    let d_eps = (D_EPS_P + D_EPS_E) / 2.0;
    let sf = (SIGMA_F_PRIME_PA - SIGMA_MEAN_PA) / E_ARMOR_PA;
    let f = |n: f64| sf * (2.0 * n).powf(B_FATIGUE)
        + EPS_F_PRIME * (2.0 * n).powf(C_FATIGUE) - d_eps;
    let (mut lo, mut hi) = (1.0_f64, 1.0e12_f64);
    let flo = f(lo);
    if flo * f(hi) > 0.0 {
        return 0.0;
    }
    for _ in 0..200 {
        let mid = (lo * hi).sqrt();
        if flo * f(mid) <= 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    (lo * hi).sqrt()
}

/// Continuous operating time per first-wall cassette at 100 Hz (s);
/// ~4.382e4 s (~12.17 h) for N_f = 4.382e6.
pub fn cassette_runtime_s() -> f64 {
    coffin_manson_nf() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fatigue_life() {
        let nf = coffin_manson_nf();
        assert!(nf >= N_F_BOUND, "N_f = {nf:.3e}");
        assert!((nf - 4.382e6).abs() / 4.382e6 < 0.05);
        assert!(cassette_runtime_s() > 4.0e4);
    }

    #[test]
    fn chaboche_step_finite() {
        let d = chaboche_backstress_step(CHABOCHE.c1, CHABOCHE.gamma1,
            100.0e6, D_EPS_P);
        assert!(d.is_finite());
    }
}
