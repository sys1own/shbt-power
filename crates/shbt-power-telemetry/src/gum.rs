//! power4.txt — GUM measurement covariance budget for the 1,325 MW
//! enthalpy-balance calorimetry and the Ni80Cr20 4-wire Kelvin
//! calibration chain.

/// Mean thermal output of the calorimetric enthalpy balance (MW).
pub const Q_MEAN_MW: f64 = 1_309.995;
/// Combined standard uncertainty bound (MW): u_c = 3.923 MW.
pub const U_C_BOUND_MW: f64 = 3.923;
/// Ni80Cr20 Joule calibration heater element temperature span (K).
pub const DT_CAL_K: f64 = 400.0;
/// Ni80Cr20 composition fraction: 80% Ni / 20% Cr.
pub const NICHROME_NI_FRAC: f64 = 0.80;

/// Independent GUM input components (c_i u_i)^2 contributions to the
/// enthalpy balance budget, summing to 15.392 MW^2 -> u_c = 3.923 MW:
/// mass-flow (Venturi), Delta-T (Pt100 4-wire), c_p table lookup,
/// and parasitic Q_loss.
pub const GUM_COMPONENTS_MW2: [f64; 4] = [6.20, 5.10, 3.10, 0.992];

/// Combined standard uncertainty u_c = sqrt(sum (c_i u_i)^2 +
/// 2 r_{m,DT} c_m u_m c_DT u_DT). The m--Delta-T correlation is
/// negligible at the table precision (r ~ 0), so the quadrature sum
/// reproduces the power4 budget: 15.392 MW^2 -> 3.923 MW.
pub fn gum_u_c_mw() -> f64 {
    let var: f64 = GUM_COMPONENTS_MW2.iter().sum();
    var.sqrt()
}

/// Whether the live enthalpy readout is inside the GUM band.
pub fn enthalpy_in_gum(q_mw: f64) -> bool {
    (q_mw - Q_MEAN_MW).abs() <= U_C_BOUND_MW
}

/// 4-wire Kelvin sensed resistance: R = V_sense / I_drive — the
/// voltage sense pair is separate from the drive pair, eliminating
/// lead resistance.
pub fn kelvin_r_ohm(v_sense: f64, i_drive: f64) -> f64 {
    v_sense / i_drive
}

/// Joule calibration power P_cal = I^2 R_NiCr (W).
pub fn joule_cal_w(i_a: f64, r_ohm: f64) -> f64 {
    i_a * i_a * r_ohm
}

/// SECDED Hamming(72,64) syndrome width: 8 check bits cover
/// 64 data bits + 8 parity = 72 total.
pub const SECDED_DATA_BITS: u32 = 64;
pub const SECDED_TOTAL_BITS: u32 = 72;
/// CRC-32C (Castagnoli) polynomial used on MMIO frames.
pub const CRC32C_POLY: u32 = 0x1EDC_6F41;

/// Diagnostic transfer function: converts ADC counts to engineering
/// units via H(f) = G0 / (1 + (f/f_c)^2) with the embedded calibration
/// line from the Ni80Cr20 heaters.
pub fn transfer_gain(g0: f64, f_hz: f64, f_c_hz: f64) -> f64 {
    g0 / (1.0 + (f_hz / f_c_hz).powi(2))
}


// ---------- power5: extended GUM budget + WLS calorimetry ----------

/// Tightened combined uncertainty budget at the 1,325.0 MW thermal
/// duty: sigma_b = 1.3416 MW (variance 1.8000 MW^2), U95 = k=2.
pub const GUM5_SIGMA_MW: f64 = 1.3416;
pub const GUM5_VAR_MW2: f64 = 1.8000;
pub const GUM5_U95_MW: f64 = 2.683;
pub const Q_CORE_P5_MW: f64 = 1325.0;

/// R_cal = 50.000 +/- 0.005 Ohm (k=2) Ni80Cr20 calibration resistor:
/// alpha = 1.05e-4 /K, beta = -4.50e-8 /K^2; lead ~0.15 Ohm;
/// sense DAQ Z_in > 10 GOhm (burden < 0.0015%).
pub const R_CAL_OHM: f64 = 50.000;
pub const R_CAL_TOL_OHM: f64 = 0.005;
pub const NICRO_ALPHA_K: f64 = 1.05e-4;
pub const NICRO_BETA_K2: f64 = -4.50e-8;
pub const R_LEAD_OHM: f64 = 0.15;
pub const Z_IN_GOHM: f64 = 10.0;

/// Weighted-least-squares slope/intercept of the calibration line
///   Y_i = b0 + b1 * P_cal,i + e_i
/// over the 5-tier traceable ramp (10-50 MW, 45 min plateaus).
pub fn wls_fit(p_cal: &[f64], y_meas: &[f64], w: &[f64]) -> (f64, f64) {
    let n = p_cal.len();
    let mut sw = 0.0;
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut i = 0;
    while i < n {
        let wi = w[i];
        sw += wi;
        sx += wi * p_cal[i];
        sy += wi * y_meas[i];
        sxx += wi * p_cal[i] * p_cal[i];
        sxy += wi * p_cal[i] * y_meas[i];
        i += 1;
    }
    let delta = sw * sxx - sx * sx;
    let b0 = (sxx * sy - sx * sxy) / delta;
    let b1 = (sw * sxy - sx * sy) / delta;
    (b0, b1)
}

/// Acceptance bounds on the calibration slope and intercept
/// (power5): |b1 - 1| <= 0.0020 and u(b0) <= 0.15 MW.
pub const B1_TOL: f64 = 0.0020;
pub const U_B0_BOUND_MW: f64 = 0.15;

pub fn wls_gain_ok(b1: f64) -> bool {
    (b1 - 1.0).abs() <= B1_TOL
}

/// Ni80Cr20 heater resistance vs temperature:
/// R(T) = R0 (1 + a (T-T0) + b (T-T0)^2), T0 = 293.15 K.
pub fn nicro_resistance_ohm(t_k: f64, r0: f64) -> f64 {
    let dt = t_k - 293.15;
    r0 * (1.0 + NICRO_ALPHA_K * dt + NICRO_BETA_K2 * dt * dt)
}

// ---------- operando diagnostics ----------

/// Channel count of the 64-station Faraday array; noise floor Gamma
/// <= -32 dB at the 2.5 GHz beam-repetition harmonic.
pub const FARADAY_CHANNELS: usize = 64;
pub const FARADAY_NOISE_DB: f64 = -32.0;
/// Suppressor grid bias/current ceiling: -1.50 kV, < 10 uA.
pub const GRID_BIAS_KV: f64 = -1.50;
pub const GRID_LEAK_UA: f64 = 10.0;
/// Thomson scattering probe: E_x = 1.20 MV/m, B_x = 0.85 T,
/// spatial resolution +/- 25 um.
pub const THOMSON_EX_MV_M: f64 = 1.20;
pub const THOMSON_BX_T: f64 = 0.85;
pub const THOMSON_RES_UM: f64 = 25.0;
/// scCVD diamond detector leakage < 10 pA at 500 V / 500 K.
pub const SCCVD_LEAK_PA: f64 = 10.0;
/// Streak camera sweep 50 mm/ps, trigger jitter <= 150 fs.
pub const STREAK_SWEEP_MM_PS: f64 = 50.0;
pub const STREAK_JITTER_FS: f64 = 150.0;
/// GaP electro-optic sampling resolution <= 1.0 ps.
pub const EO_RES_PS: f64 = 1.0;
/// BPM electronics: 10 um nominal, +/- 5 um per-shot resolution.
pub const BPM_RES_UM: f64 = 10.0;
/// Coherent diffraction radiation spatial resolution >= 15 um.
pub const CDR_RES_UM: f64 = 15.0;
/// Telemetry frame rate 10 kHz.
pub const TELEM_RATE_HZ: f64 = 10.0e3;

#[cfg(test)]
mod tests2 {
    use super::*;

    #[test]
    fn gum5_and_wls() {
        assert!((GUM5_SIGMA_MW.powi(2) - GUM5_VAR_MW2).abs() / GUM5_VAR_MW2 < 0.01);
        assert!((2.0 * GUM5_SIGMA_MW - GUM5_U95_MW).abs() < 0.001);
        let p = [10.0, 20.0, 30.0, 40.0, 50.0];
        let y = [10.001, 20.002, 29.998, 40.001, 49.999];
        let w = [1.0; 5];
        let (b0, b1) = wls_fit(&p, &y, &w);
        assert!(b0.abs() < 0.01);
        assert!(wls_gain_ok(b1));
        let r = nicro_resistance_ohm(900.0, R_CAL_OHM);
        assert!(r > R_CAL_OHM);
    }

    #[test]
    fn diagnostic_bounds() {
        assert_eq!(FARADAY_CHANNELS, 64);
        assert_eq!(FARADAY_NOISE_DB, -32.0);
        assert_eq!(STREAK_JITTER_FS, 150.0);
        assert_eq!(EO_RES_PS, 1.0);
        assert_eq!(TELEM_RATE_HZ, 10.0e3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gum_budget() {
        let uc = gum_u_c_mw();
        assert!((uc - U_C_BOUND_MW).abs() < 0.01);
        assert!(enthalpy_in_gum(1_312.0));
        assert!(!enthalpy_in_gum(1_320.0));
        assert_eq!(SECDED_TOTAL_BITS - SECDED_DATA_BITS, 8);
        assert!((kelvin_r_ohm(1.2, 0.5) - 2.4).abs() < 1e-12);
        assert!((joule_cal_w(2.0, 50.0) - 200.0).abs() < 1e-9);
    }
}
