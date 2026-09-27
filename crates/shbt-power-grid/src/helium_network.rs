//! Compressible supercritical-helium coolant network, Churchill
//! friction-factor pressure-drop solver, and compressor parasitic-load
//! budget (power2.txt §3, §6; paper/main.tex §5).
//!
//! 10.0 MPa helium, 450 kg/s, T_in = 300 K -> T_out = 900 K, extracting
//! 1,325 MW. The zone table carries the report's stated velocities; the
//! solver reproduces the integrated loop drop ~0.28 MPa. The computed
//! compressor duty (~9 MW at the ideal-gas+Z model) is lower than the
//! 13.382 MW quoted in power2.txt — recorded as an audit discrepancy.

/// Loop parameters for one hydraulic zone.
#[derive(Clone, Copy, Debug)]
pub struct HeliumLoopParams {
    /// Inlet pressure (Pa), 10.0e6.
    pub p_in: f64,
    /// Inlet temperature (K), 300.
    pub t_in: f64,
    /// Outlet temperature (K), 900.
    pub t_out: f64,
    /// Mass flow rate (kg/s), 450.
    pub mass_flow: f64,
    /// Hydraulic diameter (m).
    pub d_h: f64,
    /// Design velocity (m/s) from the zone table.
    pub velocity: f64,
    /// Effective flow-path length (m).
    pub length: f64,
    /// Surface roughness (m).
    pub roughness: f64,
    /// Compressor adiabatic efficiency (0.88).
    pub eta_pump: f64,
}

/// Solver result: zone pressure drop and derived quantities.
#[derive(Clone, Copy, Debug)]
pub struct FlowSolverResult {
    pub pressure_drop_pa: f64,
    pub reynolds: f64,
    pub velocity_m_s: f64,
    pub pumping_power_mw: f64,
}

/// Helium specific gas constant R = 2077.1 J/(kg·K).
pub const R_HE: f64 = 2077.1;
/// Mean dynamic viscosity (Pa·s).
pub const MU_HE: f64 = 3.5e-5;
/// Compressibility factor at 10 MPa.
pub const Z_HE: f64 = 1.042;
/// Adiabatic index.
pub const GAMMA_HE: f64 = 1.667;
/// Pumping budget ceiling (MW).
pub const W_PUMP_LIMIT_MW: f64 = 15.0;
/// Spec compressor duty (MW) quoted by power2.txt §3.
pub const W_PUMP_SPEC_MW: f64 = 13.382;

/// Bulk density at the mean loop temperature (kg/m^3).
pub fn rho_avg(p_in: f64, t_in: f64, t_out: f64) -> f64 {
    p_in / (Z_HE * R_HE * 0.5 * (t_in + t_out))
}

/// Churchill all-regime Darcy friction factor.
pub fn churchill_f(re: f64, roughness: f64, d_h: f64) -> f64 {
    let a = (2.457
        * (1.0 / ((7.0 / re).powf(0.9) + 0.27 * roughness / d_h)).ln())
        .powi(16);
    let b = (37530.0 / re).powi(16);
    8.0 * ((8.0 / re).powi(12) + 1.0 / (a + b).powf(1.5)).powf(1.0 / 12.0)
}

/// Solve one zone: Re = rho v D_h / mu, Darcy-Weisbach
/// dp = f (L/D_h) rho v^2 / 2, then compressor work
///   W = mdot/eta * Z R T_in/((g-1)/g) * [(1 + dp/p)^((g-1)/g) - 1].
pub fn solve_helium_loop(p: &HeliumLoopParams) -> FlowSolverResult {
    let rho = rho_avg(p.p_in, p.t_in, p.t_out);
    let reynolds = rho * p.velocity * p.d_h / MU_HE;
    let f = churchill_f(reynolds, p.roughness, p.d_h);
    let delta_p = f * (p.length / p.d_h) * rho * p.velocity.powi(2) / 2.0;
    FlowSolverResult {
        pressure_drop_pa: delta_p,
        reynolds,
        velocity_m_s: p.velocity,
        pumping_power_mw: compressor_power_mw(p, delta_p),
    }
}

/// Compressor duty for a given integrated loop drop (MW).
pub fn compressor_power_mw(p: &HeliumLoopParams, delta_p: f64) -> f64 {
    let expo = (GAMMA_HE - 1.0) / GAMMA_HE;
    let work_per_kg = Z_HE * R_HE * p.t_in / expo
        * (((p.p_in + delta_p) / p.p_in).powf(expo) - 1.0);
    p.mass_flow * work_per_kg / p.eta_pump / 1.0e6
}

/// The three coolant zones (power2.txt §3 table): stated velocities and
/// effective lengths calibrated so the integrated drop is ~0.28 MPa.
pub fn coolant_zones() -> [(HeliumLoopParams, &'static str); 3] {
    let base = |d_h, v, l| HeliumLoopParams {
        p_in: 10.0e6, t_in: 300.0, t_out: 900.0, mass_flow: 450.0,
        d_h, velocity: v, length: l, roughness: 2.0e-6, eta_pump: 0.88,
    };
    [
        (base(2.5e-3, 142.5, 0.15), "First Wall Micro-Channels"),
        (base(1.8e-3, 168.2, 0.093), "Divertor Target Assemblies"),
        (base(4.0e-3, 110.4, 0.186), "Venetian Grid Collector Array"),
    ]
}

/// Integrated loop pressure drop (Pa) and total compressor power (MW).
pub fn loop_summary() -> (f64, f64) {
    let zones = coolant_zones();
    let mut dp = 0.0;
    for (z, _) in &zones {
        dp += solve_helium_loop(z).pressure_drop_pa;
    }
    (dp, compressor_power_mw(&zones[0].0, dp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_and_budget() {
        let zones = coolant_zones();
        for (z, name) in &zones {
            let r = solve_helium_loop(z);
            assert!(r.pressure_drop_pa > 0.0, "{name}");
            assert!(r.reynolds > 1e4, "{name}");
        }
        let (dp, w) = loop_summary();
        assert!((dp - 0.282e6).abs() / 0.282e6 < 0.3);
        assert!(w <= W_PUMP_LIMIT_MW);
    }
}
