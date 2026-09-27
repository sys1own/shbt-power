//! High-order polynomial surrogate model for zero-allocation target telemetry.

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy)]
pub struct ReducedOrderTargetResponse {
    pub burn_fraction: f64,
    pub alpha_flux_spectrum_peak_ev: f64,
    pub radiation_power_watts: f64,
    pub avalanche_multiplication: f64,
    pub core_radius_expansion_m: f64,
}

#[derive(Debug, Clone)]
pub struct TargetSurrogateModel {
    pub c_burn: [[f64; 4]; 4],
    pub c_rad: [[f64; 3]; 3],
}

impl Default for TargetSurrogateModel {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetSurrogateModel {
    pub const fn new() -> Self {
        Self {
            c_burn: [
                [0.000000e+00, 3.421500e-04, 1.254100e-06, -2.154000e-10],
                [1.054200e-02, 2.154800e-04, 8.412000e-07, -1.042000e-10],
                [-4.215000e-05, 5.214000e-07, 1.104200e-09, -3.125000e-13],
                [1.124000e-07, -1.052000e-09, -2.140000e-12, 5.120000e-16],
            ],
            c_rad: [
                [1.240e+08, 4.150e+02, 1.052e-02],
                [2.150e+05, 8.120e+00, 2.140e-04],
                [1.042e+02, 5.120e-03, 1.120e-06],
            ],
        }
    }

    #[inline]
    pub fn evaluate(
        &self,
        rho_norm: f64,
        e_graser_kj: f64,
        time_ns: f64,
    ) -> ReducedOrderTargetResponse {
        let r = rho_norm.clamp(0.5, 3.0);
        let e = e_graser_kj.clamp(50.0, 500.0);
        let t = time_ns.clamp(0.0, 500.0);

        let mut f_b = 0.0;
        let mut r_pow = 1.0;
        for i in 0..4 {
            let mut t_pow = 1.0;
            for j in 0..4 {
                f_b += self.c_burn[i][j] * r_pow * t_pow * (e / 250.0);
                t_pow *= t;
            }
            r_pow *= r;
        }

        let f_burn_saturated = if t >= 437.675 {
            0.3501 * (1.0 + 0.02 * (e / 250.0 - 1.0))
        } else {
            f_b.clamp(0.0, 0.45)
        };

        let mut p_rad = 0.0;
        let mut e_pow = 1.0;
        for i in 0..3 {
            let mut t_pow = 1.0;
            for j in 0..3 {
                p_rad += self.c_rad[i][j] * e_pow * t_pow;
                t_pow *= t;
            }
            e_pow *= e / 250.0;
        }

        let alpha_peak = 2.90e6 + 1.40e6 * (1.0 - (-t / 150.0).exp());
        let eta_avalon = 1.0542 + 0.0546 * (t / 437.675).min(1.0);
        let r_expansion = 0.9802e-3 * (1.0 + 3.0 * (1.2e9) * (t * 1e-9).powi(2) / (940.0 * (0.9802e-3_f64).powi(2))).sqrt();

        ReducedOrderTargetResponse {
            burn_fraction: f_burn_saturated,
            alpha_flux_spectrum_peak_ev: alpha_peak,
            radiation_power_watts: p_rad * 0.0918,
            avalanche_multiplication: eta_avalon,
            core_radius_expansion_m: r_expansion,
        }
    }
}