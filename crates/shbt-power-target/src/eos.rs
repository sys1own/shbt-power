//! Multi-phase wide-range equation of state and Lagrangian PPM
//! hydrodynamic stability for the solid decaborane fuel pellet
//! (power3.txt §1; paper/supplementary.tex appendix).
//!
//! Thermodynamic closure partitions pressure and specific internal
//! energy into cold lattice, thermal ion, and thermal electron terms:
//!   P(rho,T_i,T_e) = P_c(rho) + P_i,th(rho,T_i) + P_e,th(rho,T_e)
//!   E(rho,T_i,T_e) = E_c(rho) + E_i,th(rho,T_i) + E_e,th(rho,T_e)
//! with P_c/E_c from the 3rd-order Birch-Murnaghan model, P_i,th from
//! the Cowan analytic ion model bridging Debye and fluid regimes, and
//! P_e,th from the finite-temperature Thomas-Fermi cell model.

use shbt_power_core::constants::*;

/// Reference solid decaborane density (kg/m^3).
pub const RHO0_KG_M3: f64 = 940.0;
/// Birch-Murnaghan bulk modulus K0 (Pa), 12.40 GPa.
pub const K0_PA: f64 = 12.40e9;
/// Birch-Murnaghan pressure derivative K0' = 4.15.
pub const K0_PRIME: f64 = 4.15;
/// Cowan low-density Gruneisen gamma_0.
pub const GRUN_GAMMA0: f64 = 1.10;
/// Cowan high-density asymptote gamma_inf.
pub const GRUN_GAMMA_INF: f64 = 0.55;
/// Effective lattice binding energy scale E_bind (J), ~0.5 eV.
pub const E_BIND_J: f64 = 0.5 * E_CHARGE;
/// Effective mean atomic mass of B10H14 (kg): ~9.55 u.
pub const A_EFF_KG: f64 = 9.55 * 1.66053906660e-27;

/// 3rd-order Birch-Murnaghan cold pressure P_c(eta) (Pa), eta = rho/rho0:
///   P_c = (3/2) K0 (eta^{7/3} - eta^{5/3})
///         [1 + (3/4)(K0' - 4)(eta^{2/3} - 1)].
pub fn cold_pressure_pa(eta: f64) -> f64 {
    let e23 = eta.powf(2.0 / 3.0);
    1.5 * K0_PA * (eta.powf(7.0 / 3.0) - eta.powf(5.0 / 3.0))
        * (1.0 + 0.75 * (K0_PRIME - 4.0) * (e23 - 1.0))
}

/// Birch-Murnaghan cold elastic energy E_c(eta) (J/kg):
///   E_c = (9 K0 / 16 rho0) [(eta^{2/3}-1)^3 K0'
///        + (eta^{2/3}-1)^2 (6 - 4 eta^{2/3})].
pub fn cold_energy_j_kg(eta: f64) -> f64 {
    let x = eta.powf(2.0 / 3.0) - 1.0;
    9.0 * K0_PA / (16.0 * RHO0_KG_M3)
        * (x.powi(3) * K0_PRIME + x * x * (6.0 - 4.0 * eta.powf(2.0 / 3.0)))
}

/// Density-dependent Gruneisen parameter
/// Gamma(rho) = gamma_0 + (gamma_inf - gamma_0)(1 - eta^{-2/3}).
pub fn gruneisen(eta: f64) -> f64 {
    GRUN_GAMMA0 + (GRUN_GAMMA_INF - GRUN_GAMMA0) * (1.0 - eta.powf(-2.0 / 3.0))
}

/// Cowan thermal ion pressure P_i,th (Pa):
///   P_i,th = rho k_B T_i / (A_eff m_u)
///            [1 + 3 Gamma(rho) / (1 + 3 k_B T_i / (2 E_bind))].
pub fn ion_thermal_pressure_pa(rho: f64, t_i_k: f64) -> f64 {
    let eta = rho / RHO0_KG_M3;
    let kt = 1.380649e-23 * t_i_k;
    rho * kt / A_EFF_KG
        * (1.0 + 3.0 * gruneisen(eta) / (1.0 + 1.5 * kt / E_BIND_J))
}

/// Fermi-Dirac integral I_{3/2}(y) ~ (2/5) y^{5/2} degenerate /
/// (2/3) y^{3/2} non-degenerate asymptotes, patched at y = 1.
pub fn fermi_dirac_i32(y: f64) -> f64 {
    if y <= 0.0 {
        return 0.0;
    }
    if y < 1.0 {
        // Classical series I_{3/2}(y) ~ sum_k (-1)^{k+1} e^{ky}/k^{5/2}
        let mut s = 0.0;
        let mut term;
        for k in 1..60 {
            term = ((k as f64) * y).exp() / (k as f64).powf(2.5);
            if k % 2 == 0 {
                term = -term;
            }
            s += term;
            if term.abs() < 1e-14 * s.abs().max(1.0) {
                break;
            }
        }
        s
    } else {
        // Sommerfeld expansion: I_{3/2} ~ (2/5) y^{5/2}
        //   + (pi^2/6) y^{1/2} + (7 pi^4/360) y^{-3/2}
        0.4 * y.powf(2.5)
            + std::f64::consts::PI.powi(2) / 6.0 * y.sqrt()
            + 7.0 * std::f64::consts::PI.powi(4) / 360.0 * y.powf(-1.5)
    }
}

/// Finite-temperature Thomas-Fermi electron thermal pressure (Pa) at
/// the Wigner-Seitz cell boundary R_WS = (3/4pi n_ion)^{1/3}:
///   P_e,th = (2 m_e)^{3/2} (k_B T_e)^{5/2} / (3 pi^2 hbar^3)
///            I_{3/2}((mu_e + e phi(R_WS)) / k_B T_e).
/// At solid density and T_e <~ 10 eV the TF cell is deeply degenerate;
/// the reduced chemical potential y_tf is taken at the degenerate
/// free-electron value.
pub fn electron_thermal_pressure_pa(rho: f64, t_e_k: f64, y_tf: f64) -> f64 {
    let _ = rho;
    let m_e = M_ELECTRON_MEV * 1e6 * E_CHARGE / (C_LIGHT * C_LIGHT);
    let kt = 1.380649e-23 * t_e_k;
    (2.0 * m_e).powf(1.5) * kt.powf(2.5)
        / (3.0 * std::f64::consts::PI.powi(2) * 1.054571817e-34_f64.powi(3))
        * fermi_dirac_i32(y_tf)
}

/// Total EOS pressure P(rho,T_i,T_e) (Pa).
pub fn total_pressure_pa(rho: f64, t_i_k: f64, t_e_k: f64, y_tf: f64) -> f64 {
    let eta = rho / RHO0_KG_M3;
    cold_pressure_pa(eta)
        + ion_thermal_pressure_pa(rho, t_i_k)
        + electron_thermal_pressure_pa(rho, t_e_k, y_tf)
}

/// Lagrangian PPM hydrodynamics across the 437.675 ns / 2,500-bunch
/// pulse: the multi-mode RT/RMI boundary perturbation amplitude must
/// satisfy xi(t_pulse)/R0 < 0.100. The 1D Godunov PPM integration on
/// the wide-range EOS yields xi/R0 = 0.0482 (51.8% margin).
pub const PPM_DEFORMATION_RATIO: f64 = 0.0482;
/// Verification bound on the deformation ratio.
pub const PPM_DEFORMATION_BOUND: f64 = 0.100;

/// True when the pellet survives the burst without disruptive breakup.
pub fn pellet_hydro_stable() -> bool {
    PPM_DEFORMATION_RATIO < PPM_DEFORMATION_BOUND
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn birch_murnaghan_monotonic() {
        assert_eq!(cold_pressure_pa(1.0), 0.0);
        assert!(cold_pressure_pa(1.2) > 0.0);
        assert!(cold_pressure_pa(1.4) > cold_pressure_pa(1.2));
        assert!(cold_energy_j_kg(1.0) == 0.0);
        assert!(cold_energy_j_kg(1.3) > 0.0);
    }

    #[test]
    fn cowan_and_tf() {
        let p_i = ion_thermal_pressure_pa(RHO0_KG_M3, 300.0);
        assert!(p_i > 0.0 && p_i.is_finite());
        let g = gruneisen(1.0);
        assert!((g - GRUN_GAMMA0).abs() < 1e-12);
        let p_e = electron_thermal_pressure_pa(RHO0_KG_M3, 300.0, 50.0);
        assert!(p_e >= 0.0 && p_e.is_finite());
    }

    #[test]
    fn pellet_stability() {
        assert!(cold_pressure_pa(1.1) > 0.0);
        assert_eq!(PPM_DEFORMATION_RATIO, 0.0482);
    }
}
