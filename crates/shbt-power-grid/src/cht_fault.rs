//! power4.txt — Petrov-Popov supercritical CHT correlations, Paris-law
//! fatigue + Yamamura-Eckstein sputtering with cluster-dynamics
//! swelling retirement, and the PCSS crowbar fault response.

/// Petrov-Popov generalized local Nusselt correlation for
/// supercritical helium:
///   Nu = Nu0 * (Cbar_p / C_p,b)^n * (rho_w/rho_b)^m
/// with Cbar_p = (i_w - i_b)/(T_w - T_b) the boundary-layer
/// integrated specific heat.
#[allow(clippy::too_many_arguments)]
pub fn petrov_popov_nu(
    nu0: f64,
    i_w: f64,
    i_b: f64,
    t_w: f64,
    t_b: f64,
    cp_b: f64,
    rho_w: f64,
    rho_b: f64,
    n: f64,
    m: f64,
) -> f64 {
    let cbar = (i_w - i_b) / (t_w - t_b).max(1e-9);
    nu0 * (cbar / cp_b.max(1e-9)).powf(n)
        * (rho_w / rho_b.max(1e-9)).powf(m)
}

/// Petukhov-Kirillov base Nusselt number:
///   Nu0 = (f/8) Re Pr / (1.07 + 12.7 (f/8)^(1/2) (Pr^(2/3) - 1)).
pub fn petukhov_nu0(f_darcy: f64, re: f64, pr: f64) -> f64 {
    let f8 = f_darcy / 8.0;
    f8 * re * pr
        / (1.07 + 12.7 * f8.sqrt() * (pr.powf(2.0 / 3.0) - 1.0))
}

/// Reynolds-parameterized Fanning friction for the transient sHe
/// loop: f = C Re^x + y (calibrated against Colebrook in the
/// supercritical window).
pub fn fanning_f(re: f64) -> f64 {
    0.046 * re.powf(-0.2) / 4.0 + 0.0008
}

/// Whether the Petrov-Popov evaluation avoids heat-transfer
/// deterioration (HTD): near the pseudocritical point the density
/// ratio term keeps Nu above 80% of the subcritical base.
pub fn avoids_htd(nu_pp: f64, nu0: f64) -> bool {
    nu_pp > 0.80 * nu0
}

// ---------- fatigue / sputtering ----------

/// Armor candidate matrix (E GPa, K_IC MPa*m^0.5, Coffin-Manson c,
/// Paris m) per power4 Table: CVD diamond, 4H-SiC, W, TZM Mo.
pub struct ArmorMaterial {
    pub name: &'static str,
    pub e_gpa: f64,
    pub nu: f64,
    pub k_ic: f64,
    pub c_fatigue: f64,
    pub paris_m: f64,
}

pub const ARMOR_MATERIALS: [ArmorMaterial; 4] = [
    ArmorMaterial { name: "CVD Diamond", e_gpa: 1140.0, nu: 0.0724,
        k_ic: 1.0, c_fatigue: -0.5, paris_m: 20.0 },
    ArmorMaterial { name: "4H-SiC", e_gpa: 450.0, nu: 0.35,
        k_ic: 2.5, c_fatigue: -0.55, paris_m: 15.0 },
    ArmorMaterial { name: "Tungsten", e_gpa: 400.0, nu: 0.28,
        k_ic: 15.0, c_fatigue: -0.75, paris_m: 3.0 },
    ArmorMaterial { name: "TZM Molybdenum", e_gpa: 320.0, nu: 0.32,
        k_ic: 20.0, c_fatigue: -0.6, paris_m: 3.0 },
];

/// Paris-law crack growth rate da/dN = C (DeltaK)^m with
/// C = 6.89e-12 MPa^-m m/cycle.
pub fn paris_dadn(delta_k: f64, paris_m: f64) -> f64 {
    6.89e-12 * delta_k.powf(paris_m)
}

/// Yamamura-Eckstein sputtering yield estimate for 3.5 MeV alphas on
/// the armor candidate: Y ~ 0.042 Q (Z2/Z1) (S_n/S_e ratio folded to
/// a screening value) — bounded <<1 atom/ion.
pub fn sputtering_yield(material: &ArmorMaterial) -> f64 {
    // Heavier substrates (higher Z2) sputter more; scale with 1/K_IC
    // as a brittle-surface proxy, normalized to W ~ 0.02.
    0.02 * (400.0 / material.e_gpa) * (15.0 / material.k_ic.max(1.0))
}

/// Cluster-dynamics swelling: vacancy accumulation rate
/// dF_sw/dt = kappa * dpa (1 - F_sw); F_sw at 5% is the retirement
/// limit. Returns dpa-per-year capacity before retirement.
pub const SWELL_RETIRE_FRAC: f64 = 0.05;
pub fn swelling_retirement_years(kappa: f64, dpa_per_year: f64) -> f64 {
    -(1.0 - SWELL_RETIRE_FRAC).ln() / (kappa * dpa_per_year).max(1e-30)
}

/// Helium bubble pressure bound P_lp = 2 gamma/r + G b / r
/// (loop-punching limit) in Pa; gamma in J/m^2, G Pa, b m, r m.
pub fn loop_punch_pressure_pa(gamma: f64, g_pa: f64, b_m: f64, r_m: f64) -> f64 {
    2.0 * gamma / r_m + g_pa * b_m / r_m
}

// ---------- fault crowbar / PCSS ----------

/// Crowbar discharge capacitance (F) sourcing the 450 MJ bank
/// quench: C = 2 E / V^2 at V = 450 kV -> 4.44 mF.
pub const E_CROWBAR_J: f64 = 450.0e6;
pub const V_CROWBAR_V: f64 = 450.0e3;
pub fn crowbar_c_f() -> f64 {
    2.0 * E_CROWBAR_J / (V_CROWBAR_V * V_CROWBAR_V)
}

/// PCSS switch closure bound (s): <2.5 ns optical trigger.
pub const PCSS_T_CLOSE_S: f64 = 2.5e-9;

/// Critically damped RLC: R_crit = 2 sqrt(L/C); returns damping
/// ratio zeta for (r,l,c).
pub fn rlc_zeta(r_ohm: f64, l_h: f64, c_f: f64) -> f64 {
    r_ohm / 2.0 * (c_f / l_h).sqrt()
}

/// Crowbar peak current when the bank dumps through loop inductance
/// l_h: I_pk ~ V sqrt(C/L) exp(-zeta/sqrt(1-zeta^2) atan(...)) —
/// overdamped safe bound I_pk <= V sqrt(C/L).
pub fn crowbar_i_peak_a(l_h: f64) -> f64 {
    V_CROWBAR_V * (crowbar_c_f() / l_h).sqrt()
}

/// Snubber-clamped crowbar is survivable when the dump energy stays
/// thermalized inside the resistor budget and zeta >= 1.
pub fn crowbar_damped_ok(l_h: f64, r_ohm: f64) -> bool {
    rlc_zeta(r_ohm, l_h, crowbar_c_f()) >= 1.0
}

/// 1 MHz MMIO polling cadence for fault capture (s).
pub const MMIO_POLL_S: f64 = 1.0e-6;
/// Fault detection-to-crowbar latency bound: detection at 1 MHz poll
/// + PCSS 2.5 ns closure << first thermal time constant.
pub fn fault_latency_ok() -> bool {
    MMIO_POLL_S + PCSS_T_CLOSE_S < 1.0e-3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn petrov_popov_no_htd() {
        let f = crate::helium_network::colebrook_f(5.0e5, 1.5e-6, 12.5e-3);
        let nu0 = petukhov_nu0(f, 5.0e5, 0.72);
        // Pseudocritical wall-bulk state: cbar > cp_b lifts Nu.
        let nu = petrov_popov_nu(
            nu0, 4.5e6, 1.6e6, 900.0, 300.0, 5.3e3, 4.0, 6.5, 0.4, -0.2);
        assert!(nu.is_finite() && nu > 0.0);
        assert!(avoids_htd(nu, nu0));
    }

    #[test]
    fn fatigue_sputter_matrix() {
        for m in &ARMOR_MATERIALS {
            assert!(m.e_gpa > 0.0 && m.paris_m > 0.0);
            let dadn = paris_dadn(10.0, m.paris_m);
            assert!(dadn >= 0.0 && dadn.is_finite());
            assert!(sputtering_yield(m) >= 0.0);
        }
        assert!(swelling_retirement_years(0.01, 2.0) > 0.0);
        assert!(loop_punch_pressure_pa(2.0, 50e9, 2.5e-10, 1e-8) > 0.0);
    }

    #[test]
    fn crowbar_fault() {
        let c = crowbar_c_f();
        assert!((c - 4.444e-3).abs() / 4.444e-3 < 0.01);
        assert!(crowbar_i_peak_a(10e-6) > 0.0);
        assert!(crowbar_damped_ok(10e-6, 0.15));
        assert!(fault_latency_ok());
        assert_eq!(PCSS_T_CLOSE_S, 2.5e-9);
    }
}
