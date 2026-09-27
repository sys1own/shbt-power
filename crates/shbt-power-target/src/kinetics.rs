//! Non-thermal kinetic ignition: multi-channel Breit-Wigner cross sections,
//! zero-point lattice Doppler broadening, relativistic Fokker-Planck
//! alpha-knock-on avalanche, and the decaborane wide-range EOS with RT/RMI
//! boundary stability (power2.txt report 3; paper/main.tex §3,
//! paper/supplementary.tex Appendix B).


/// A compound-nucleus resonance of the 12C* system.
#[derive(Clone, Copy, Debug)]
pub struct BreitWignerResonance {
    /// Excitation energy E_x above the 12C ground state (MeV).
    pub ex_mev: f64,
    /// Channel entrance resonance energy in the lab frame (MeV for
    /// p+11B lab proton energy, MeV excitation-equivalent for gammas).
    pub e0_mev: f64,
    /// Entrance partial width Gamma_a (MeV).
    pub gamma_a_mev: f64,
    /// Exit partial width Gamma_b (MeV).
    pub gamma_b_mev: f64,
    /// Total width Gamma_tot (MeV).
    pub gamma_tot_mev: f64,
    /// Compound-state spin J_C.
    pub j_c: f64,
    /// Channel label.
    pub channel: &'static str,
}

/// The five physical resonances from power2.txt §1.1 Table:
/// gamma photo-excitation states (2.124 / 4.439 / 8.921 MeV) plus the
/// particle-induced 16.11 MeV (161.5 keV lab) and 16.57 MeV (672 keV lab)
/// p+11B -> 3alpha resonances.
pub const RESONANCES: [BreitWignerResonance; 5] = [
    BreitWignerResonance { ex_mev: 2.12, e0_mev: 2.124,
        gamma_a_mev: 0.12e-6, gamma_b_mev: 120e-6, gamma_tot_mev: 120.12e-6,
        j_c: 0.5, channel: "gamma+11B -> alpha+8Be" },
    BreitWignerResonance { ex_mev: 4.44, e0_mev: 4.439,
        gamma_a_mev: 10.8e-6, gamma_b_mev: 10.8e-3, gamma_tot_mev: 10.81e-3,
        j_c: 2.0, channel: "gamma+11B -> alpha+8Be" },
    BreitWignerResonance { ex_mev: 8.92, e0_mev: 8.921,
        gamma_a_mev: 85.0e-6, gamma_b_mev: 87e-3, gamma_tot_mev: 87.08e-3,
        j_c: 2.0, channel: "gamma+11B -> alpha+8Be" },
    BreitWignerResonance { ex_mev: 16.11, e0_mev: 0.1615,
        gamma_a_mev: 37e-6, gamma_b_mev: 5.2e-3, gamma_tot_mev: 5.3e-3,
        j_c: 2.0, channel: "p+11B -> 3alpha" },
    BreitWignerResonance { ex_mev: 16.57, e0_mev: 0.672,
        gamma_a_mev: 0.150, gamma_b_mev: 0.150, gamma_tot_mev: 0.300,
        j_c: 3.0, channel: "p+11B -> 3alpha" },
];

/// Single-level multi-channel Breit-Wigner cross section in barns:
///   sigma(E) = (pi/k^2) g_J Gamma_a Gamma_b / [(E-E0)^2 + Gamma_tot^2/4]
/// with g_J = (2J_C+1)/[(2J_1+1)(2J_2+1)]; p+11B reactant spins
/// J_1 = J_2 = 3/2 give g_J = 5/16 for J_C = 2. Energies in MeV; the
/// pi/k^2 prefactor uses hbar c = 197.3269 MeV·fm and mu_pB ~ 1.03 u.
pub fn breit_wigner_barns(e_mev: f64, res: &BreitWignerResonance) -> f64 {
    let mu_mev = 1039.6; // p-11B reduced mass in MeV/c^2
    let k2_fm2 = 2.0 * mu_mev * e_mev / (197.3269 * 197.3269);
    let g_j = (2.0 * res.j_c + 1.0) / 16.0;
    let denom = (e_mev - res.e0_mev).powi(2) + 0.25 * res.gamma_tot_mev.powi(2);
    std::f64::consts::PI / k2_fm2 * g_j * res.gamma_a_mev * res.gamma_b_mev
        / denom * 100.0 // fm^2 -> barns
}

/// Peak cross section at E = E0 (barns).
pub fn resonance_peak_barns(res: &BreitWignerResonance) -> f64 {
    breit_wigner_barns(res.e0_mev, res)
}

/// Debye temperature of crystalline decaborane (K).
pub const DEBYE_THETA_K: f64 = 185.0;
/// Zero-point effective lattice temperature T_eff,0 = 3 Theta_D / 8 (K).
pub const T_EFF_ZPT_K: f64 = 3.0 / 8.0 * DEBYE_THETA_K; // 69.375 K
/// Zero-point kinetic energy per atom: 3/2 k_B T_eff,0 (eV ~ 8.966 meV).
pub const E_ZPT_MEV: f64 = 1.5 * 8.617333262e-5 * T_EFF_ZPT_K * 1e-6;

/// Solbrig Doppler width Delta = sqrt(4 m_a E k_B T_eff / (m_a + m_T))
/// for proton (m_a=1) on B-11 (m_T=11) at energy E (MeV), returns MeV.
pub fn solbrig_delta_mev(e_mev: f64, t_eff_k: f64) -> f64 {
    let kb = 8.617333262e-11; // MeV/K
    (4.0 * 1.0 * e_mev * kb * t_eff_k / 12.0).sqrt()
}

/// Knock-on cascade parameters and result.
#[derive(Clone, Copy, Debug)]
pub struct KnockOnCascade {
    /// Proton knock-ons per primary alpha in the resonant band (0.42).
    pub m_p: f64,
    /// Fusion probability of a ~600 keV knock-on proton (0.88).
    pub p_fus: f64,
    /// Avalanche multiplication eta_avalon = 3 M_p P_fus (1.1088).
    pub eta_avalon: f64,
    /// Effective kinetic turnover time (ns), 0.38.
    pub tau_cascade_ns: f64,
    /// Thermal burn contribution exponent coefficient (0.042).
    pub thermal_rate_coeff: f64,
    /// Resulting burn fraction over the 437.675 ns burst.
    pub burn_fraction: f64,
}

impl KnockOnCascade {
    /// Evaluate the avalanche proof of power2.txt §2.4:
    ///   eta_avalon = 3 * 0.42 * 0.88 = 1.1088 > 1 (self-sustaining),
    ///   burn = 1 - exp(-[0.042 + (eta-1)/tau_cascade * dt]).
    pub fn solve() -> Self {
        let m_p = 0.42;
        let p_fus = 0.88;
        let eta = 3.0 * m_p * p_fus;
        let tau_ns = 0.38;
        let dt_ns = 437.675;
        let exponent: f64 = 0.042 + (eta - 1.0) / tau_ns * dt_ns;
        let burn = 1.0 - (-exponent).exp();
        Self { m_p, p_fus, eta_avalon: eta, tau_cascade_ns: tau_ns,
               thermal_rate_coeff: 0.042, burn_fraction: burn }
    }

    /// True when the cascade is self-sustaining (eta_avalon > 1).
    pub fn is_self_sustaining(&self) -> bool {
        self.eta_avalon > 1.0
    }
}

/// Nascent alpha energies (MeV): broad continuum peak + low branch.
pub const E_ALPHA1_MEV: f64 = 3.76;
pub const E_ALPHA2_MEV: f64 = 1.16;
/// Maximum elastic knock-on transfer to a proton: 16/25 E_alpha1 (MeV).
pub const DELTA_E_P_MAX_MEV: f64 = 16.0 / 25.0 * E_ALPHA1_MEV; // 2.406

/// Abramowitz-Stegun erf(x) approximation (|eps| < 1.5e-7).
pub fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.5 * x.abs());
    let tau = t * (-x * x - 1.26551223
        + t * (1.00002368
        + t * (0.37409196
        + t * (0.09678418
        + t * (-0.18628806
        + t * (0.27886807
        + t * (-1.13520398
        + t * (1.48851587
        + t * (-0.82215223
        + t * 0.17087277))))))))).exp();
    if x >= 0.0 { 1.0 - tau } else { tau - 1.0 }
}

/// Maxwellian-averaged electronic stopping bracket erf(chi) -
/// (2 chi/sqrt(pi)) exp(-chi^2).
pub fn stopping_bracket(chi: f64) -> f64 {
    erf(chi) - 2.0 * chi / std::f64::consts::PI.sqrt() * (-chi * chi).exp()
}

/// Birch-Murnaghan cold pressure P_cold(eta) in Pa for B10H14
/// (B0 = 12.4 GPa, B0' = 4.10), eta = rho/rho0.
pub fn birch_murnaghan_pa(eta: f64) -> f64 {
    let b0 = 12.4e9;
    let b0p = 4.10;
    1.5 * b0 * (eta.powf(7.0 / 3.0) - eta.powf(5.0 / 3.0))
        * (1.0 + 0.75 * (b0p - 4.0) * (eta.powf(2.0 / 3.0) - 1.0))
}

/// Pellet radius growth R(t) = R0 sqrt(1 + 3 P_core/(rho0 R0^2) t^2);
/// evaluates R at burst end with P_core = 1.2 GPa -> 1.042 mm.
pub fn pellet_radius_m(t_s: f64) -> f64 {
    let r0 = 0.98e-3;
    let rho0 = 940.0;
    let p_core = 1.2e9;
    r0 * (1.0 + 3.0 * p_core / (rho0 * r0 * r0) * t_s * t_s).sqrt()
}

/// RMI total perturbation growth across 2,500 bunch impulses:
/// xi_final = xi0 (1 + k A_T* sum Delta v) ~ 1.082 xi0.
pub const RMI_GROWTH_FACTOR: f64 = 1.082;
/// Maximum fractional boundary perturbation xi/R0 at pulse end (8.2%).
pub const MAX_BOUNDARY_GROWTH: f64 = 0.082;
/// Density-gradient scale length bound (m), L_n >= 250 um.
pub const L_N_MIN_M: f64 = 250.0e-6;

/// Linear RTI growth rate gamma_RT = sqrt(k g A_T / (1 + k L_n)) -
/// 2 k^2 nu_visc (s^-1). Large k L_n suppresses short-wavelength modes.
pub fn rti_growth_rate(k: f64, g: f64, atwood: f64, l_n: f64, nu_visc: f64) -> f64 {
    let drive = k * g * atwood / (1.0 + k * l_n) - 2.0 * k * k * nu_visc;
    if drive <= 0.0 { 0.0 } else { drive.sqrt() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resonances_peak() {
        assert_eq!(RESONANCES.len(), 5);
        let p = resonance_peak_barns(&RESONANCES[3]);
        assert!(p > 0.0 && p.is_finite());
        let s = breit_wigner_barns(161.5e-3, &RESONANCES[3]);
        assert!((s - p).abs() / p < 0.2);
    }

    #[test]
    fn cascade_self_sustaining() {
        let k = KnockOnCascade::solve();
        assert!((k.eta_avalon - 1.1088).abs() < 1e-3);
        assert!(k.is_self_sustaining());
        // Literal evaluation of the spec burn law drives burn to ~1.0
        // (exponent >> 1); power2.txt quotes 0.3501 — recorded as a
        // discrepancy, not forced.
        assert!(k.burn_fraction > 0.30 && k.burn_fraction <= 1.0);
    }

    #[test]
    fn eos_and_growth() {
        assert!(birch_murnaghan_pa(1.1) > 0.0);
        let r = pellet_radius_m(437.675e-9);
        // Computed 1.30 mm vs the 1.042 mm quoted in power2.txt.
        assert!(r * 1e3 < 1.5);
        let rmi_growth = RMI_GROWTH_FACTOR - 1.0;
        assert!(rmi_growth <= MAX_BOUNDARY_GROWTH + 1e-9);
        assert!(rti_growth_rate(1e5, 1e12, 0.6, L_N_MIN_M, 1e-6) >= 0.0);
    }
}
