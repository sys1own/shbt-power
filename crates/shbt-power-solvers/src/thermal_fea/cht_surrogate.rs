// crates/shbt-power-solvers/src/thermal_fea/cht_surrogate.rs

/// Configuration parameters for supercritical helium micro-channel heat sink.
#[derive(Debug, Clone, Copy)]
pub struct MicroChannelConfig {
    pub length_m: f64,
    pub hydraulic_diameter_m: f64,
    pub relative_roughness: f64,
    pub channel_count: usize,
}

pub struct ChtSurrogateSolver {
    pub config: MicroChannelConfig,
}

impl ChtSurrogateSolver {
    pub fn new(config: MicroChannelConfig) -> Self {
        Self { config }
    }

    /// Computes Darcy friction factor across laminar, transition, and turbulent regimes
    /// using Churchill's correlation.
    pub fn churchill_friction_factor(&self, reynolds: f64) -> f64 {
        if reynolds < 1e-3 {
            return 64.0;
        }

        let re = reynolds;
        let rr = self.config.relative_roughness;

        let term_a = (2.457 * ((1.0 / ((7.0 / re).powf(0.9) + 0.27 * rr)).ln())).powi(16);
        let term_b = (37530.0 / re).powi(16);

        let part1 = (8.0 / re).powi(12);
        let part2 = 1.0 / (term_a + term_b).powf(1.5);

        8.0 * (part1 + part2).powf(1.0 / 12.0)
    }

    /// Computes supercritical helium pressure drop (Pa) and pumping power (W).
    pub fn evaluate_hydraulics(
        &self,
        mass_flow_total_kg_s: f64,
        fluid_density_kg_m3: f64,
        dynamic_viscosity_pa_s: f64,
        circulator_efficiency: f64,
    ) -> (f64, f64) {
        let m_dot_per_channel = mass_flow_total_kg_s / (self.config.channel_count as f64);
        let area = std::f64::consts::PI * (self.config.hydraulic_diameter_m / 2.0).powi(2);
        let velocity = m_dot_per_channel / (fluid_density_kg_m3 * area);

        let reynolds = (fluid_density_kg_m3 * velocity * self.config.hydraulic_diameter_m) 
            / dynamic_viscosity_pa_s;

        let f_darcy = self.churchill_friction_factor(reynolds);
        let delta_p = f_darcy * (self.config.length_m / self.config.hydraulic_diameter_m) 
            * (fluid_density_kg_m3 * velocity.powi(2) / 2.0);

        let pumping_power_w = (mass_flow_total_kg_s * delta_p) 
            / (fluid_density_kg_m3 * circulator_efficiency);

        (delta_p, pumping_power_w)
    }
}