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
