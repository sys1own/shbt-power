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

// ---------- power7 PCHE core synthesis ----------

// Conjugate Heat Transfer (CHT) Surrogate Solver for Supercritical Helium PCHE Core.
// Formulates Churchill friction factors, Petrov-Popov variable property scalings,
// and discretized channel pressure drop integration across 64 primary sectors.

use std::f64::consts::PI;

/// Geometric parameters defining the diffusion-bonded micro-channel PCHE core.
#[derive(Debug, Clone, Copy)]
pub struct PcheGeometry {
    pub channel_diameter_m: f64,
    pub active_length_m: f64,
    pub sector_count: usize,
    pub total_distribution_channels: usize,
    pub total_core_channels: usize,
    pub surface_roughness_m: f64,
}

impl Default for PcheGeometry {
    fn default() -> Self {
        Self {
            channel_diameter_m: 0.00150,          // 1.50 mm
            active_length_m: 3.20,                // 3.20 m active passage length
            sector_count: 64,                     // 64 primary macro sectors
            total_distribution_channels: 1_057_728, // Cold distribution passage count
            total_core_channels: 440_000,         // High-temp active passage count
            surface_roughness_m: 1.5e-6,          // 1.5 um chemical etch roughness
        }
    }
}

impl PcheGeometry {
    /// Computes single-channel semi-circular cross-sectional area (m^2).
    pub fn cross_sectional_area(&self) -> f64 {
        let r = self.channel_diameter_m / 2.0;
        PI * r * r / 2.0
    }

    /// Computes single-channel wetted perimeter (m).
    pub fn wetted_perimeter(&self) -> f64 {
        let r = self.channel_diameter_m / 2.0;
        PI * r + self.channel_diameter_m
    }

    /// Computes channel equivalent hydraulic diameter D_h (m).
    pub fn hydraulic_diameter(&self) -> f64 {
        4.0 * self.cross_sectional_area() / self.wetted_perimeter()
    }
}

/// Evaluates Darcy friction factor using Churchill's universal formulation (1977).
pub fn churchill_friction_factor(reynolds_num: f64, relative_roughness: f64) -> f64 {
    if reynolds_num <= 0.0 {
        return 0.0;
    }
    let term1 = (8.0 / reynolds_num).powi(12);
    let a_arg = 1.0 / ((7.0 / reynolds_num).powf(0.9) + 0.27 * relative_roughness);
    let a = (2.457 * a_arg.ln()).powi(16);
    let b = (37530.0 / reynolds_num).powi(16);
    let term2 = (a + b).powf(-1.5);
    
    8.0 * (term1 + term2).powf(1.0 / 12.0)
}

/// Applies Petrov-Popov correction for non-isothermal boundary layer properties.
pub fn petrov_popov_friction_factor(
    f_darcy: f64,
    rho_wall: f64,
    rho_bulk: f64,
    mu_wall: f64,
    mu_bulk: f64,
) -> f64 {
    let rho_ratio = (rho_wall / rho_bulk).powf(0.3);
    let mu_ratio = (mu_wall / mu_bulk).powf(0.15);
    f_darcy * rho_ratio * mu_ratio
}

/// Integrates local frictional and acceleration pressure drop along channel length.
pub fn calculate_channel_pressure_drop(
    geom: &PcheGeometry,
    mass_flow_total: f64,
    t_in_k: f64,
    t_out_k: f64,
    p_in_pa: f64,
) -> f64 {
    let n_ch = geom.total_distribution_channels as f64;
    let m_dot_ch = mass_flow_total / n_ch;
    let a_ch = geom.cross_sectional_area();
    let d_h = geom.hydraulic_diameter();
    let rel_rough = geom.surface_roughness_m / d_h;

    let num_steps = 100;
    let dz = geom.active_length_m / (num_steps as f64);
    let mut current_p = p_in_pa;

    for i in 0..num_steps {
        let z_frac = (i as f64 + 0.5) / (num_steps as f64);
        let t_local = t_in_k + z_frac * (t_out_k - t_in_k);
        
        // Density and dynamic viscosity profiles for supercritical Helium at 10.0 MPa
        let rho_local = 16.05 * (300.0 / t_local);
        let mu_local = 1.98e-5 * (t_local / 300.0).powf(0.70);
        let u_local = m_dot_ch / (rho_local * a_ch);

        let re_local = (rho_local * u_local * d_h) / mu_local;
        let f_d = churchill_friction_factor(re_local, rel_rough);
        
        // Wall boundary layer offset (+50 K thermal delta)
        let rho_wall = 16.05 * (300.0 / (t_local + 50.0));
        let mu_wall = 1.98e-5 * ((t_local + 50.0) / 300.0).powf(0.70);
        let f_pp = petrov_popov_friction_factor(f_d, rho_wall, rho_local, mu_wall, mu_local);

        let dp_dz = f_pp * (rho_local * u_local * u_local) / (2.0 * d_h);
        current_p -= dp_dz * dz;
    }

    p_in_pa - current_p
}