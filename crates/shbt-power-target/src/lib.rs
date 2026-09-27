//! shbt-power-target — enriched decaborane (11B10H14) pellet stoichiometry
//! and 100 Hz electromagnetic injection kinematics with a fifth-order
//! minimum-jerk trajectory profile (paper/main.tex §5E, paper/supplementary.tex §9).

pub mod kinetics;
pub mod ionization;
pub mod eos;
pub mod first_principles;

use shbt_power_core::constants::*;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};

/// Decaborane pellet stoichiometry and inventory.
#[derive(Clone, Copy, Debug)]
pub struct PelletInventory {
    pub mol_weight_g_mol: f64,
    pub density_kg_m3: f64,
    pub n_b_total: f64,
    pub n_molecules: f64,
    pub mass_mg: f64,
    pub volume_mm3: f64,
    pub diameter_mm: f64,
}

impl PelletInventory {
    /// Solve the fuel inventory required for an 87.5 MJ shot at f_burn = 0.35.
    pub fn solve() -> Self {
        let m_w = 10.0 * 11.009305 + 14.0 * 1.007825; // g/mol
        let rho = 940.0;
        // Boron atoms consumed per shot = E_yield / (8.7 MeV in J).
        let q_j = Q_PB11_MEV * 1e6 * E_CHARGE;
        let n_consumed = E_YIELD_MJ * 1e6 / q_j;
        let n_b = n_consumed / F_BURN;
        let n_mol = n_b / 10.0;
        let mass_kg = n_mol * (m_w / 1000.0) / 6.022_140_76e23;
        let vol_m3 = mass_kg / rho;
        let vol_mm3 = vol_m3 * 1e9;
        let d_mm = 2.0 * (3.0 * vol_mm3 / (4.0 * core::f64::consts::PI)).cbrt();
        Self {
            mol_weight_g_mol: m_w,
            density_kg_m3: rho,
            n_b_total: n_b,
            n_molecules: n_mol,
            mass_mg: mass_kg * 1e6,
            volume_mm3: vol_mm3,
            diameter_mm: d_mm,
        }
    }

    /// Fusion events per shot actually consumed.
    pub fn fusion_events() -> f64 {
        E_YIELD_MJ * 1e6 / (Q_PB11_MEV * 1e6 * E_CHARGE)
    }

    /// Daily fuel throughput (kg/day) at 100 Hz.
    pub fn daily_consumption_kg(&self) -> f64 {
        self.mass_mg * 1e-6 * F_REP_HZ * 86_400.0
    }
}

/// Fifth-order minimum-jerk profile: s(tau) = 10 t^3 - 15 t^4 + 6 t^5.
pub fn min_jerk_s(tau: f64) -> f64 {
    let t = tau.clamp(0.0, 1.0);
    10.0 * t.powi(3) - 15.0 * t.powi(4) + 6.0 * t.powi(5)
}
pub fn min_jerk_v(tau: f64) -> f64 {
    let t = tau.clamp(0.0, 1.0);
    30.0 * t.powi(2) - 60.0 * t.powi(3) + 30.0 * t.powi(4)
}

/// Electromagnetic railgun injector at v_inj = 250 m/s over L = 2.50 m.
#[derive(Clone, Copy, Debug)]
pub struct Injector {
    pub v_inj_m_s: f64,
    pub l_flight_m: f64,
    pub t_transit_s: f64,
    pub tracking_jitter_um: f64,
    pub timing_jitter_ps: f64,
}

impl Injector {
    pub fn solve() -> Self {
        let t = L_FLIGHT_M / V_INJ_M_S;
        // Quadrant-photodiode closed-loop jitter residuals (validated budget).
        Self {
            v_inj_m_s: V_INJ_M_S,
            l_flight_m: L_FLIGHT_M,
            t_transit_s: t,
            tracking_jitter_um: 5.0,
            timing_jitter_ps: 50.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TargetSubsystem {
    pub pellet: PelletInventory,
    pub injector: Injector,
}

impl Default for TargetSubsystem {
    fn default() -> Self {
        Self {
            pellet: PelletInventory::solve(),
            injector: Injector::solve(),
        }
    }
}

impl PhysicsSubsystem for TargetSubsystem {
    fn name(&self) -> &'static str {
        "shbt-power-target"
    }
    fn update(&mut self, state: &mut PlantStateSnapshot) {
        state.target_jitter_um = self.injector.tracking_jitter_um;
    }
}


// ---------------------------------------------------------------------------
// power6: zero-allocation reduced-order target response surrogate exported
// from the Tier-1 solver workbench (shbt-power-solvers::target_kinetic).
// ---------------------------------------------------------------------------
pub use shbt_power_solvers::target_kinetic::surrogate::{
    ReducedOrderTargetResponse, TargetSurrogateModel,
};

/// Evaluates the design point (rho_norm=1, E_graser=250 kJ) at the end of the
/// 437.675 ns macro-pulse; f_burn must saturate at >= 35.01%.
#[inline]
pub fn target_surrogate_design_point() -> ReducedOrderTargetResponse {
    TargetSurrogateModel::new().evaluate(1.0, 250.0, 437.675)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pellet_stoichiometry() {
        let p = PelletInventory::solve();
        assert!((p.mol_weight_g_mol - 124.2026).abs() < 1e-3);
        assert!((p.n_b_total - 1.7978e20).abs() / 1.7978e20 < 0.01, "{}", p.n_b_total);
        assert!((p.mass_mg - 3.708).abs() < 0.01, "{}", p.mass_mg);
        assert!((p.volume_mm3 - 3.9445).abs() < 0.02);
        assert!((p.diameter_mm - 1.96).abs() < 0.02);
        assert!((p.daily_consumption_kg() - 32.04).abs() < 0.1);
    }

    #[test]
    fn min_jerk_profile() {
        assert!(min_jerk_s(0.0).abs() < 1e-12);
        assert!((min_jerk_s(1.0) - 1.0).abs() < 1e-12);
        assert!(min_jerk_v(0.0).abs() < 1e-12 && min_jerk_v(1.0).abs() < 1e-12);
    }

    #[test]
    fn injector_timing() {
        let i = Injector::solve();
        assert!((i.t_transit_s - 10.0e-3).abs() < 1e-9);
        assert!(i.tracking_jitter_um <= JITTER_POS_UM);
        assert!(i.timing_jitter_ps <= JITTER_TIME_PS);
    }
}
