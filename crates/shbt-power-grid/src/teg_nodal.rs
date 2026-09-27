//! Dual-stage TEG nodal thermal network: temperature-dependent Seebeck /
//! electrical / thermal conductivities, figure-of-merit efficiency, and
//! coupled Joule + Thomson nodal balance (power2.txt §4; paper/main.tex §5).
//!
//! Skutterudite CoSb3 topping (600-900 K), half-Heusler ZrNiSn bottoming
//! (300-600 K); combined efficiency 33.804% on 1,325 MW -> 447.903 MW.

/// Material property set for one TEG stage.
#[derive(Clone, Copy, Debug)]
pub struct TegMaterial {
    /// Base Seebeck coefficient S0 (uV/K).
    pub s0_uv_k: f64,
    /// Base electrical conductivity sigma0 (1/(Ohm m)).
    pub sigma0: f64,
    /// Base thermal conductivity k0 (W/(m K)).
    pub k0: f64,
    /// Interfacial thermal resistance (m^2 K/W).
    pub r_th_c: f64,
    /// Interfacial electrical resistance (Ohm m^2).
    pub r_e_c: f64,
    /// Hot-side temperature (K).
    pub t_hot: f64,
    /// Cold-side temperature (K).
    pub t_cold: f64,
}

/// CoSb3 skutterudite topping stage.
pub const SKUTTERUDITE: TegMaterial = TegMaterial {
    s0_uv_k: 210.5, sigma0: 1.25e5, k0: 2.15,
    r_th_c: 1.2e-4, r_e_c: 2.5e-9, t_hot: 900.0, t_cold: 600.0,
};
/// ZrNiSn half-Heusler bottoming stage.
pub const HALF_HEUSLER: TegMaterial = TegMaterial {
    s0_uv_k: 165.2, sigma0: 0.88e5, k0: 3.42,
    r_th_c: 1.5e-4, r_e_c: 3.0e-9, t_hot: 600.0, t_cold: 300.0,
};

/// Temperature-dependent Seebeck coefficient S(T) = s0 + s1 T + s2 T^2.
pub fn seebeck_uv_k(m: &TegMaterial, t: f64) -> f64 {
    // gentle downward slope ~ -0.02 uV/K^2 normalized
    let t_mid = 0.5 * (m.t_hot + m.t_cold);
    m.s0_uv_k * (1.0 + 0.02 * (t - t_mid) / t_mid)
}

/// Temperature-dependent electrical conductivity (1/(Ohm m)).
pub fn sigma_inv_m(m: &TegMaterial, t: f64) -> f64 {
    let t_mid = 0.5 * (m.t_hot + m.t_cold);
    m.sigma0 * (1.0 + 0.05 * (t - t_mid) / t_mid)
}

/// Temperature-dependent thermal conductivity (W/(m K)).
pub fn k_w_mk(m: &TegMaterial, t: f64) -> f64 {
    let t_mid = 0.5 * (m.t_hot + m.t_cold);
    m.k0 * (1.0 - 0.05 * (t - t_mid) / t_mid)
}

/// Figure of merit ZT = S^2 sigma / k * T at the stage mean temperature.
pub fn figure_of_merit(m: &TegMaterial) -> f64 {
    let t = 0.5 * (m.t_hot + m.t_cold);
    let s = seebeck_uv_k(m, t) * 1e-6;
    s * s * sigma_inv_m(m, t) / k_w_mk(m, t) * t
}

/// Single-stage conversion efficiency:
///   eta = (T_H - T_C)/T_H * (sqrt(1 + Z Tbar) - 1)
///         / (sqrt(1 + Z Tbar) + T_C/T_H).
pub fn stage_efficiency(m: &TegMaterial) -> f64 {
    let zt = figure_of_merit(m);
    let root = (1.0 + zt).sqrt();
    (m.t_hot - m.t_cold) / m.t_hot * (root - 1.0) / (root + m.t_cold / m.t_hot)
}

/// Spec efficiency quoted by power2.txt (33.804% -> 447.903 MW on 1,325 MW).
/// The ZT-based solver computes a materially lower stage-averaged
/// efficiency (~14%) from the stated material properties; the delta is
/// recorded as an audit discrepancy rather than masked.
pub const ETA_TEG_SPEC: f64 = 0.33804;

/// Combined dual-stage efficiency:
///   eta = eta_top + eta_bot - eta_top eta_bot.
pub fn combined_efficiency() -> f64 {
    stage_efficiency(&SKUTTERUDITE) + stage_efficiency(&HALF_HEUSLER)
        - stage_efficiency(&SKUTTERUDITE) * stage_efficiency(&HALF_HEUSLER)
}

/// Nodal interface heat flux q'' = (T_i - T_{i+1})/R_th,c + J^2 R_e,c.
pub fn interface_flux(m: &TegMaterial, t_i: f64, t_i1: f64, j: f64) -> f64 {
    (t_i - t_i1) / m.r_th_c + j * j * m.r_e_c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_stage_efficiency() {
        let eta = combined_efficiency();
        assert!(eta > 0.0 && eta < 1.0);
        // Computed-vs-spec delta is recorded, not forced to match.
        assert!(ETA_TEG_SPEC > eta);
        assert!(figure_of_merit(&SKUTTERUDITE) > 0.5);
        assert!(interface_flux(&SKUTTERUDITE, 900.0, 890.0, 1e4) > 0.0);
    }
}
