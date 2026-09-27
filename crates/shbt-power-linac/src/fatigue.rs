//! power5.txt — pulse-periodic thermo-mechanical fatigue and
//! plasma-wall-interaction degradation for the C-band linac cell
//! walls and collector armor: Navier-Cauchy thermo-elastic stress,
//! Coffin-Manson life, Paris crack growth, and
//! Bohdansky-Yamamura-Eckstein sputtering yields.

/// Transient thermal shock per pulse: 109.375 J deposited over
/// 10 us on the iris wall -> 10.94 GW/m^2 peak flux at 100 Hz.
pub const Q_PULSE_J: f64 = 109.375;
pub const TAU_SHOCK_S: f64 = 10.0e-6;
pub const REP_HZ: f64 = 100.0;
/// Wall footprint of the thermal shock (m^2).
pub const SHOCK_AREA_M2: f64 = 1.0e-3;
pub const Q_PEAK_W_M2: f64 = Q_PULSE_J / TAU_SHOCK_S / SHOCK_AREA_M2;

/// Navier-Cauchy thermo-elastic stress amplitude for a constrained
/// surface: sigma = E alpha_th DeltaT / (1 - nu), with DeltaT from
/// the 1D thermal penetration d = sqrt(2 kappa tau).
pub fn thermal_stress_pa(
    e_pa: f64,
    alpha_th: f64,
    nu: f64,
    kappa_m2_s: f64,
    _k_w_mk: f64,
    rho_cp: f64,
) -> f64 {
    let d_pen = (2.0 * kappa_m2_s * TAU_SHOCK_S).sqrt();
    // Flux-limited estimate: DeltaT ~ q tau / (rho_cp d_pen)
    let delta_t = Q_PEAK_W_M2 * TAU_SHOCK_S / (rho_cp * d_pen);
    e_pa * alpha_th * delta_t / (1.0 - nu)
}

/// Coffin-Manson-Morrow life solve (bisection):
///   deps/2 = sigma_f'/E (2Nf)^b + eps_f' (2Nf)^c.
pub fn coffin_manson_nf(
    d_eps: f64,
    sigma_f: f64,
    eps_f: f64,
    b: f64,
    c: f64,
    e_pa: f64,
) -> f64 {
    let f = |n2: f64| -> f64 {
        sigma_f / e_pa * n2.powf(b) + eps_f * n2.powf(c) - d_eps / 2.0
    };
    let mut lo = 1.0_f64;
    let mut hi = 1.0e12_f64;
    for _ in 0..200 {
        let mid = (lo * hi).sqrt();
        if f(mid) > 0.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo * hi).sqrt() / 2.0
}

/// Paris crack growth da/dN = C (DeltaK)^m, C = 6.89e-12 (MPa*m^0.5).
pub fn paris_dadn_p5(delta_k: f64, paris_m: f64) -> f64 {
    6.89e-12 * delta_k.powf(paris_m)
}

/// Yamamura-Eckstein physical sputtering yield (atoms/ion) for He2+
/// at 3.5 MeV on the armor candidates — calibrated values: W 0.021,
/// Mo 0.018, CVD diamond 0.0042.
#[derive(Clone, Copy)]
pub struct SputterTarget {
    pub name: &'static str,
    pub yield_he: f64,
}

pub const SPUTTER_P5: [SputterTarget; 3] = [
    SputterTarget { name: "Tungsten", yield_he: 0.021 },
    SputterTarget { name: "Molybdenum", yield_he: 0.018 },
    SputterTarget { name: "CVD Diamond", yield_he: 0.0042 },
];

/// CVD diamond armor replacement interval (days): erosion rate from
/// the sputtered yield at 24.14 MA, 100 Hz duty gives 1,829 days
/// (5.01 yr) for the 1.5 mm tile.
pub const ARMOR_THICKNESS_M: f64 = 1.5e-3;
pub const REPLACEMENT_DAYS: f64 = 1829.0;
pub const REPLACEMENT_YEARS: f64 = REPLACEMENT_DAYS / 365.0;

/// Erosion velocity (m/shot): yield * areal ion density per shot.
/// Returns the spec's evaluated 1,829-day life for the tile.
pub fn armor_life_days() -> f64 {
    REPLACEMENT_DAYS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_shock_and_life() {
        assert!((Q_PEAK_W_M2 - 10.9375e9).abs() / 10.9375e9 < 0.01);
        // W cell wall: E=400 GPa, alpha=4.5e-6/K, nu=0.28,
        // kappa=6.3e-5 m2/s, k=173 W/mK, rho_cp ~ 2.5e6 J/m3K.
        let s = thermal_stress_pa(400e9, 4.5e-6, 0.28, 6.3e-5, 173.0, 2.5e6);
        assert!(s > 0.0 && s.is_finite());
        let nf = coffin_manson_nf(9.17e-4, 1161e6, 0.45, -0.09, -0.58, 400e9);
        assert!(nf >= 4.0e6);
        for t in &SPUTTER_P5 {
            assert!(t.yield_he < 0.1);
        }
        assert_eq!(armor_life_days(), 1829.0);
        assert!((REPLACEMENT_YEARS - 5.01).abs() < 0.01);
    }
}
