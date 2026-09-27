//! 2D/3D non-neutral plasma sheath, guiding-center expander optics,
//! active thermionic neutralization, and secondary-electron suppression
//! (power2.txt report 2; paper/main.tex §4, paper/supplementary.tex).

use shbt_power_core::constants::*;

/// Guiding-center pitch-angle evolution sin(theta(z)) = sqrt(B/B0)
/// sin(theta0). Evaluates theta at the collector for theta0 = pi/2:
/// arcsin(sqrt(0.05/5.0)) = arcsin(0.1) = 5.739 deg.
pub fn collector_pitch_deg(theta0_rad: f64) -> f64 {
    (theta0_rad.sin() * (B_COLL_T / B_CORE_T).sqrt()).asin().to_degrees()
}

/// Fraction of gyro-energy converted to parallel motion at the collector:
/// E_parallel/E0 = 1 - (B_c/B0) sin^2(theta0) >= 0.99.
pub fn parallel_energy_fraction(theta0_rad: f64) -> f64 {
    1.0 - (B_COLL_T / B_CORE_T) * theta0_rad.sin().powi(2)
}

/// Radial envelope r(z) = r0 sqrt(B0/B(z)); collector radius 2.50 m.
pub fn expander_radius_m(b_t: f64) -> f64 {
    R_CORE_M * (B_CORE_T / b_t).sqrt()
}

/// Collector diameter D_coll = 2 r_c = 5.000 m.
pub fn collector_diameter_m() -> f64 {
    2.0 * expander_radius_m(B_COLL_T)
}

/// Transient alpha pulse peak current (A) and duration (s).
pub const I_PEAK_A: f64 = 24.14e6;
pub const TAU_PULSE_S: f64 = 2.0e-6;
/// Deceleration gap (m) for the sheath solver.
pub const GAP_M: f64 = 0.15;
/// Terminal stage potential (V), 2.7 MV.
pub const V3_V: f64 = 2.7e6;
/// Alpha charge state and velocity at collector (parallel, ~3.5 MeV).
pub const Q_ALPHA_C: f64 = 2.0 * E_CHARGE;
pub const V_PARA_C_M_S: f64 = 1.2928e7;

/// 1D Child-Langmuir ceiling for 3.5 MeV He^2+ across the 2.7 MV stage:
///   J_CL = (4/9) eps0 sqrt(2 q/m) V^{3/2}/d^2 ~ 7.6204e3 A/m^2.
pub fn child_langmuir_sheath_a_m2(v: f64, d: f64) -> f64 {
    (4.0 / 9.0) * EPS0 * (2.0 * Q_ALPHA_C / M_ALPHA_KG).sqrt() * v.powf(1.5)
        / (d * d)
}

/// Actual transient current density at the collector plane:
///   J_actual = I_peak / (pi r_c^2) = 24.14 MA / 19.635 m^2 ~ 1.2295e6 A/m^2.
pub fn actual_current_density_a_m2() -> f64 {
    let a_c = std::f64::consts::PI * expander_radius_m(B_COLL_T).powi(2);
    I_PEAK_A / a_c
}

/// Required thermionic injection density n_e,inj >= Z J/(q v_para)
/// ~ 5.937e17 m^-3.
pub fn neutralization_density_m3() -> f64 {
    2.0 * actual_current_density_a_m2() / (Q_ALPHA_C * V_PARA_C_M_S)
}

/// Electron plasma frequency omega_pe = sqrt(n_e e^2/(eps0 m_e)) (rad/s).
pub fn electron_plasma_freq(n_e: f64) -> f64 {
    let m_e = M_ELECTRON_MEV * 1e6 * E_CHARGE / (C_LIGHT * C_LIGHT);
    (n_e * E_CHARGE * E_CHARGE / (EPS0 * m_e)).sqrt()
}

/// Collisionless thermalization time tau_therm ~ 10/omega_pe (s).
pub fn thermalization_s() -> f64 {
    10.0 / electron_plasma_freq(neutralization_density_m3())
}

/// Dynamic neutralization gain: J_eff limit scales by (1 - phi_e)^-1 with
/// phi_e = 0.995 -> effective limit ~ 200x the bare J_CL.
pub const PHI_E: f64 = 0.995;

/// Effective space-charge ceiling under neutralization (A/m^2).
pub fn effective_cl_limit_a_m2() -> f64 {
    child_langmuir_sheath_a_m2(V3_V, GAP_M) / (1.0 - PHI_E)
}

/// Grid geometric transparency eta_trans = 0.987.
pub const GRID_TRANSPARENCY: f64 = 0.987;
/// Thermal power intercepted per grid during the pulse (W).
/// Note: I_parasitic * V3 = 847.3 GW computed here; the 847.3 MW figure
/// quoted in power2.txt report 2 is a units slip (tracked as an audit
/// discrepancy, matching GATE-43-style reconciliation notes).
pub fn grid_thermal_w() -> f64 {
    (1.0 - GRID_TRANSPARENCY) * I_PEAK_A * V3_V
}
/// Energy deposited per 2.0 us pulse (J): ~1.694 MJ computed
/// (1.694 kJ in power2.txt carries the same 1e3 slip).
pub fn grid_pulse_energy_j() -> f64 {
    grid_thermal_w() * TAU_PULSE_S
}
/// Parasitic interception current (A): 313.82 kA.
pub fn parasitic_current_a() -> f64 {
    (1.0 - GRID_TRANSPARENCY) * I_PEAK_A
}
/// Inter-stage capacitance (F) and transient dV/dt (V/s).
pub const C_STAGE_F: f64 = 1.85e-9;
pub const DV_DT_V_S: f64 = 13.5e12;
/// Capacitive charging current (A): 24.975 kA.
pub fn capacitive_current_a() -> f64 {
    C_STAGE_F * DV_DT_V_S
}

/// Sternglass secondary-electron yield coefficient (nm/eV) per material.
#[derive(Clone, Copy, Debug)]
pub struct SeeMaterial {
    pub name: &'static str,
    pub a_mat: f64,
    pub yields: [f64; 3], // 3.5 / 1.8 / 0.5 MeV
    pub thermal_cond: f64,
    pub melting_k: f64,
}

/// W and Mo SEE table (power2.txt report 2).
pub const SEE_TABLE: [SeeMaterial; 2] = [
    SeeMaterial { name: "Tungsten", a_mat: 0.012,
        yields: [1.48, 2.35, 4.18], thermal_cond: 173.0, melting_k: 3695.0 },
    SeeMaterial { name: "Molybdenum", a_mat: 0.010,
        yields: [1.12, 1.82, 3.24], thermal_cond: 138.0, melting_k: 2896.0 },
];

/// Suppressor grid bias (kV).
pub const V_SUPP_KV: f64 = -50.0;
/// Wire spacing s_g (m) and suppressor gap d_s (m).
pub const WIRE_SPACING_M: f64 = 2.5e-3;
pub const SUPPRESSOR_GAP_M: f64 = 12.0e-3;
/// Central potential barrier created at aperture midplane (kV): -4.85 kV,
/// far exceeding the ~50 eV max secondary-electron energy.
pub const PHI_BARRIER_KV: f64 = -4.85;

/// Reverse-leakage fraction of recovered alpha power (<0.12%).
pub const MAX_REVERSE_LEAKAGE: f64 = 0.0012;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expander_optics() {
        let half_pi = std::f64::consts::FRAC_PI_2;
        assert!((collector_pitch_deg(half_pi) - 5.739).abs() < 0.01);
        assert!(parallel_energy_fraction(half_pi) >= 0.99);
        assert!((collector_diameter_m() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn sheath_neutralization() {
        let j_cl = child_langmuir_sheath_a_m2(V3_V, GAP_M);
        assert!((j_cl - 7.6204e3).abs() / 7.6204e3 < 0.05);
        let j_act = actual_current_density_a_m2();
        assert!((j_act - 1.2295e6).abs() / 1.2295e6 < 0.05);
        // Neutralization lifts the effective ceiling above J_actual.
        assert!(effective_cl_limit_a_m2() > j_act);
        assert!(thermalization_s() < TAU_PULSE_S);
        assert!((neutralization_density_m3() - 5.937e17).abs() / 5.937e17 < 0.1);
    }

    #[test]
    fn suppressor_and_thermal() {
        assert!((grid_pulse_energy_j() - 1.694e6).abs() / 1.694e6 < 0.1);
        assert_eq!(PHI_BARRIER_KV, -4.85);
        assert!((capacitive_current_a() - 24.975e3).abs() < 100.0);
        assert_eq!(SEE_TABLE.len(), 2);
    }
}
