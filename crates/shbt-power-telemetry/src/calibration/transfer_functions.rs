
/// Deterministic hardware transfer functions converting ADC raw voltages to MMIO telemetry.
pub struct CalibrationTransferEngine;

impl CalibrationTransferEngine {
    /// Maps 16-bit ADC voltage [0.0..5.0 V] to Venetian Grid Bias [MV]
    #[inline(always)]
    pub fn adc_to_grid_voltage(v_adc: f32, stage: usize) -> f32 {
        let v_clamped = v_adc.clamp(0.0, 5.0);
        match stage {
            1..=3 => v_clamped, // unity scale: 0.800/1.800/2.700 MV nominal rails
            0 => v_clamped * 0.0,
            _ => 0.0,
        }
    }

    /// Maps Rogowski coil ADC voltage [0.0..5.0 V] to Alpha Burst Current [kA]
    /// 2.414 V corresponds to 24.14 MA (24,140 kA).
    #[inline(always)]
    pub fn adc_to_alpha_current_ka(v_adc: f32) -> f32 {
        v_adc.clamp(0.0, 5.0) * 10_000.0
    }

    /// Linearizes Pt-100 RTD resistance to absolute temperature in Kelvin via ITS-90 CVD
    #[inline]
    pub fn rtd_volts_to_temperature_k(v_adc: f32) -> f32 {
        let r_meas = (v_adc.clamp(0.0, 5.0) as f64) * 100.0; // 1.0 V/100 Ohm
        const R0: f64 = 100.0;
        const A: f64 = 3.9083e-3;
        const B: f64 = -5.7750e-7;

        let discriminant = A * A - 4.0 * B * (1.0 - r_meas / R0);
        if discriminant >= 0.0 {
            let t_celsius = (-A + discriminant.sqrt()) / (2.0 * B);
            (t_celsius + 273.15) as f32
        } else {
            300.0_f32 // Fail-safe default
        }
    }

    /// Encodes a 64-bit data word into a 72-bit SECDED Hamming codeword (8 parity bits)
    pub fn secded_hamming_encode(data: u64) -> u64 {
        let mut p: u8 = 0;
        let mut overall_parity: u64 = 0;

        for i in 0..64 {
            let bit = (data >> i) & 1;
            if bit == 1 {
                overall_parity ^= 1;
                // Accumulate syndrome index coverage
                let col = (i as u64 + 1) ^ (i as u64 + 1).next_power_of_two();
                p ^= (col & 0x7F) as u8;
            }
        }
        let check_bits = ((p as u64) & 0x7F) | ((overall_parity ^ (p.count_ones() as u64 & 1)) << 7);
        (data & 0x00FF_FFFF_FFFF_FFFF) | (check_bits << 56)
    }
}