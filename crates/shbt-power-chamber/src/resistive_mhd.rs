//! Resistive high-beta MHD fireball stopping, magnetic Rayleigh-Taylor
//! stability, and the HTS inductive-pickup coupled-circuit network
//! (power2.txt report 2; paper/main.tex §4, paper/supplementary.tex).
//!
//! The vented fireball carries E_k = 13.125 MJ per pulse into the MHD
//! channel against B_ext in [3.5, 5.0] T. The energy-balance stopping
//! radius r_c = (3 mu0 E_k / (2 pi B^2))^{1/3} gives 0.8631 m at 3.5 T and
//! 0.6804 m at 5.0 T for this channel (distinct from the 70 MJ full-plasma
//! radius of `stopping_radius_m`, which remains the first-wall criterion).

use shbt_power_core::constants::*;

/// MHD-channel fireball kinetic energy per pulse (J): 13.125 MJ.
pub const E_FIREBALL_J: f64 = 13.125e6;
/// Expansion velocity (m/s).
pub const V_EXP_M_S: f64 = 1.30e6;
/// Expansion timescale (s).
pub const TAU_EXP_S: f64 = 2.0e-6;
/// Spitzer resistivity at T_e = 500 eV, Z = 2 (Ohm m): ~1.50e-7.
pub const ETA_SPITZER: f64 = 1.50e-7;

/// Stopping radius of the 13.125 MJ fireball: r_c = (3 mu0 E_k / (2 pi
/// B^2))^{1/3}.
pub fn fireball_stopping_radius_m(b_t: f64) -> f64 {
    (3.0 * MU0 * E_FIREBALL_J / (2.0 * std::f64::consts::PI * b_t * b_t))
        .powf(1.0 / 3.0)
}

/// Magnetic Reynolds number R_m = mu0 v_exp r_c / eta (~9.36e6 at 3.5 T).
pub fn magnetic_reynolds(b_t: f64) -> f64 {
    MU0 * V_EXP_M_S * fireball_stopping_radius_m(b_t) / ETA_SPITZER
}

/// Magnetic skin depth delta_m = sqrt(2 eta tau_exp / mu0) ~ 0.691 mm.
pub fn magnetic_skin_m() -> f64 {
    (2.0 * ETA_SPITZER * TAU_EXP_S / MU0).sqrt()
}

/// Effective deceleration g_eff = v_exp^2 / (2 r_c) (m/s^2).
pub fn effective_deceleration(b_t: f64) -> f64 {
    V_EXP_M_S * V_EXP_M_S / (2.0 * fireball_stopping_radius_m(b_t))
}

/// MRT linear growth rate gamma_MRT = sqrt(g_eff k - (k.B)^2/(mu0 rho))
/// for mode m (k = m/r_c); magnetic shear suppresses m > 12.
pub fn mrt_growth_rate(b_t: f64, mode_m: f64, rho: f64, b_shear: f64) -> f64 {
    let r_c = fireball_stopping_radius_m(b_t);
    let k = mode_m / r_c;
    let drive = effective_deceleration(b_t) * k;
    let stab = k * k * b_shear * b_shear / (MU0 * rho);
    if drive <= stab { 0.0 } else { (drive - stab).sqrt() }
}

/// Peak nonlinear flute distortion fraction for modes m = 2..6: 0.35 r_c.
pub const MRT_FLUTE_FRACTION: f64 = 0.35;

/// Minimum first-wall radius satisfying R_fw >= r_c + Delta_r_flute +
/// d_cushion (0.50 m): 1.3631 m at 3.5 T, 1.1804 m at 5.0 T.
pub fn min_wall_radius_m(b_t: f64) -> f64 {
    let r_c = fireball_stopping_radius_m(b_t);
    r_c + MRT_FLUTE_FRACTION * r_c + CUSHION_MIN_M
}

/// HTS pickup-coil self-inductance (H): 12.4 uH.
pub const L_HTS_H: f64 = 12.4e-6;
/// Plasma self-inductance coefficient L_0 (H) — order mu0 R_chamber.
pub const L_PLASMA_H: f64 = MU0 * 1.50;
/// Mutual-inductance scale M_0 = 0.6 L_HTS.
pub const M0_H: f64 = 0.6 * L_HTS_H;
/// Crowbar trigger threshold on flux-compression rate (T/s equivalent
/// per m^2): dB/dt > 1.2e12.
pub const DB_DT_TRIGGER: f64 = 1.2e12;
/// SiC MOSFET recovery-shunt efficiency.
pub const ETA_CROWBAR: f64 = 0.9420;

/// Plasma self-inductance L_p(t) = L0 (1 - r^3/R_chamber^3).
pub fn plasma_inductance_h(r_m: f64) -> f64 {
    L_PLASMA_H * (1.0 - (r_m / 1.50).powi(3)).max(0.0)
}

/// Mutual inductance M(t) = M0 (r/r_c)^2.
pub fn mutual_inductance_h(r_m: f64, r_c_m: f64) -> f64 {
    M0_H * (r_m / r_c_m).powi(2)
}

/// One explicit Euler step of the coupled pickup-circuit ODEs:
///   d/dt[L_p I_p + M I_HTS] + R_p I_p = 0
///   d/dt[L_HTS I_HTS + M I_p] + R_HTS I_HTS + V_crowbar = 0
/// Returns (dI_p, dI_HTS).
pub fn pickup_step(
    r_m: f64,
    r_c_m: f64,
    dr_dt: f64,
    i_p: f64,
    i_hts: f64,
    r_p: f64,
    v_crowbar: f64,
) -> (f64, f64) {
    let l_p = plasma_inductance_h(r_m);
    let m = mutual_inductance_h(r_m, r_c_m);
    let dl_p_dt = -3.0 * L_PLASMA_H * r_m * r_m * dr_dt / 1.50_f64.powi(3);
    let dm_dt = 2.0 * M0_H * r_m * dr_dt / (r_c_m * r_c_m);
    let det = l_p * L_HTS_H - m * m;
    if det.abs() < 1e-18 {
        return (0.0, 0.0);
    }
    // Solve [L_p M; M L_HTS] [dI_p; dI_HTS] = rhs
    let rhs1 = -dl_p_dt * i_p - dm_dt * i_hts - r_p * i_p;
    let rhs2 = -dm_dt * i_p - v_crowbar; // R_HTS -> 0 superconducting
    let di_p = (rhs1 * L_HTS_H - m * rhs2) / det;
    let di_hts = (l_p * rhs2 - m * rhs1) / det;
    (di_p, di_hts)
}

/// Bean critical-state sheet current J_c(B,T) = J_c0 (1 - T/Tc)^2
/// B0/(B + B0). J_c0 = 1.0e9 A/m^2, B0 = 0.1 T, MgB2 T_c = 31.79 K.
pub fn bean_jc(b_t: f64, t_k: f64) -> f64 {
    const JC0: f64 = 1.0e9;
    const B0: f64 = 0.1;
    const TC: f64 = 31.79;
    if t_k >= TC {
        return 0.0;
    }
    JC0 * (1.0 - t_k / TC).powi(2) * B0 / (b_t + B0)
}

/// Superconducting thermal margin: T_op = 20.0 K, T_c = 31.79 K ->
/// Delta T = 11.79 K headroom.
pub const SC_HEADROOM_K: f64 = 11.79;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fireball_stopping() {
        assert!((fireball_stopping_radius_m(3.5) - 0.8631).abs() < 0.01);
        assert!((fireball_stopping_radius_m(5.0) - 0.6804).abs() < 0.01);
        assert!(magnetic_reynolds(3.5) > 1e6);
        assert!((magnetic_skin_m() - 0.691e-3).abs() < 0.05e-3);
    }

    #[test]
    fn mrt_and_wall() {
        assert!(mrt_growth_rate(3.5, 2.0, 1.0, 0.0) > 0.0);
        // Shear stabilizes high-m modes.
        let uns = mrt_growth_rate(3.5, 20.0, 1.0, 0.0);
        let shd = mrt_growth_rate(3.5, 20.0, 1.0, 0.5);
        assert!(shd <= uns);
        assert!(min_wall_radius_m(3.5) < 2.20);
    }

    #[test]
    fn pickup_circuit() {
        let (di_p, di_h) = pickup_step(0.4, 0.86, V_EXP_M_S, 1e6, 0.0, 0.01, 0.0);
        assert!(di_p.is_finite() && di_h.is_finite());
        assert!(bean_jc(1.0, 20.0) > 0.0);
        assert_eq!(bean_jc(1.0, 40.0), 0.0);
    }
}
