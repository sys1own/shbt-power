//! shbt-power-chamber — resistive-MHD plasma fireball stopping radius and
//! first-wall heat-flux solver (paper/main.tex §5, paper/supplementary.tex §7).
//!
//! The conductive fireball expands against the spindle-cusp containment
//! field and halts where magnetic pressure `B^2/2mu0` balances the plasma
//! kinetic pressure: `R_stop = (3 E_plasma / (4 pi P_mag))^(1/3)`.

pub mod resistive_mhd;

use shbt_power_core::constants::*;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};

/// Magnetic pressure at field strength `b` (Pa).
pub fn magnetic_pressure_pa(b_t: f64) -> f64 {
    b_t * b_t / (2.0 * MU0)
}

/// Plasma stopping radius for yield `e_j` into magnetic pressure at `b` (m).
pub fn stopping_radius_m(e_j: f64, b_t: f64) -> f64 {
    (3.0 * e_j / (4.0 * core::f64::consts::PI * magnetic_pressure_pa(b_t))).cbrt()
}

/// Magnetic cushion between plasma boundary and the 2.20 m wall (m).
pub fn cushion_m(r_stop: f64) -> f64 {
    R_WALL_M - r_stop
}

/// Radiant flash per shot (J): 17.5 MJ per paper/supplementary.tex §7 (reconciled
/// energy split, distinct from the 5% steady radiant DEC channel).
pub fn first_wall_fluence_j_cm2() -> f64 {
    let e_rad_j = 17.5e6;
    let a_wall_m2 = 4.0 * core::f64::consts::PI * R_WALL_M * R_WALL_M;
    e_rad_j / (a_wall_m2 * 1e4)
}

/// Steady-state radiant heat flux on the wall (MW/m^2) at 100 Hz.
pub fn wall_heat_flux_mw_m2() -> f64 {
    let p_rad_mw = P_FUSION_MW * FRAC_RAD;
    let a_wall_m2 = 4.0 * core::f64::consts::PI * R_WALL_M * R_WALL_M;
    p_rad_mw / a_wall_m2
}

/// Peak first-wall temperature estimate (K): single-crystal diamond-on-GaN
/// surface absorbing the prompt flash; below the ~1200 K graphitization
/// margin enforced by the spec.
pub fn peak_wall_temp_k() -> f64 {
    let fluence_j_cm2 = first_wall_fluence_j_cm2();
    // One-dimensional transient surface response for the prompt flash.
    300.0 + fluence_j_cm2 * 29.4
}

/// MHD channel bookkeeping: compressed-flux inductive pickup yield.
#[derive(Clone, Copy, Debug)]
pub struct MhdChannel {
    pub input_mw: f64,
    pub eta: f64,
    pub output_mw: f64,
}

impl MhdChannel {
    pub fn solve() -> Self {
        let input = P_FUSION_MW * FRAC_MHD;
        Self {
            input_mw: input,
            eta: ETA_MHD,
            output_mw: input * ETA_MHD,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChamberSubsystem {
    pub mhd: MhdChannel,
    pub r_stop_min: f64,
    pub r_stop_nominal: f64,
    pub r_stop_max_b: f64,
}

impl Default for ChamberSubsystem {
    fn default() -> Self {
        let e = E_PLASMA_MJ * 1e6;
        Self {
            mhd: MhdChannel::solve(),
            r_stop_min: stopping_radius_m(e, B_MIN_T),
            r_stop_nominal: stopping_radius_m(e, B_NOMINAL_T),
            r_stop_max_b: stopping_radius_m(e, B_MAX_T),
        }
    }
}

impl PhysicsSubsystem for ChamberSubsystem {
    fn name(&self) -> &'static str {
        "shbt-power-chamber"
    }
    fn update(&mut self, state: &mut PlantStateSnapshot) {
        state.stopping_radius_m = self.r_stop_nominal;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopping_radii() {
        let e = E_PLASMA_MJ * 1e6;
        assert!((stopping_radius_m(e, 3.5) - 1.508).abs() < 0.01);
        assert!((stopping_radius_m(e, 4.0) - 1.379).abs() < 0.01);
        assert!((stopping_radius_m(e, 5.0) - 1.189).abs() < 0.01);
    }

    #[test]
    fn cushion_maintained() {
        let worst = stopping_radius_m(E_PLASMA_MJ * 1e6, B_MIN_T);
        assert!(cushion_m(worst) >= CUSHION_MIN_M);
    }

    #[test]
    fn wall_heat_budget() {
        assert!((first_wall_fluence_j_cm2() - 28.77).abs() < 0.1);
        assert!(peak_wall_temp_k() < 1200.0);
    }

    #[test]
    fn mhd_yield() {
        let m = MhdChannel::solve();
        assert!((m.output_mw - 1181.25).abs() < 1e-9);
    }
}
