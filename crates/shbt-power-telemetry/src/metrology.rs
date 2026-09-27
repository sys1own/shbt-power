//! power5.txt — calibration metrology: ASTM/ISO materials checkers
//! and the five-phase plant commissioning state machine with
//! per-phase acceptance bounds.

/// ASTM E1004 conductivity acceptance: >= 101.5% IACS,
/// surface finish R_a <= 0.10 um, residual-resistivity ratio >= 300.
pub fn astm_e1004_ok(iacs_pct: f64, r_a_um: f64, rrr: f64) -> bool {
    iacs_pct >= 101.5 && r_a_um <= 0.10 && rrr >= 300.0
}

/// ASTM E1681 environmental cracking threshold:
/// K_IEAC >= 45 MPa*sqrt(m) at 10 MPa / 900 K supercritical helium.
pub const K_IEAC_BOUND: f64 = 45.0;
pub fn astm_e1681_ok(k_ieac: f64) -> bool {
    k_ieac >= K_IEAC_BOUND
}

/// ASTM F1624 step-load ratio P_th/P_FFS >= 0.75 for hydrogen-
/// assisted fracture immunity of the chamber fasteners.
pub const F1624_RATIO_BOUND: f64 = 0.75;
pub fn astm_f1624_ok(p_th: f64, p_ffs: f64) -> bool {
    p_th / p_ffs >= F1624_RATIO_BOUND
}

/// Commissioning acceptance bounds across the five handover phases.
pub const P_BASE_PA: f64 = 1.0e-5;
pub const LEAK_BOUND: f64 = 1.0e-10;
pub const HOLD_DRIFT_MPA_24H: f64 = 0.005;
pub const HOLD_PRESSURE_MPA: f64 = 12.5;
pub const DP_CORE_BOUND_MPA: f64 = 0.3;
pub const Q_TH_MW: f64 = 1325.0;
pub const Q_TH_TOL_MW: f64 = 2.68;
/// Digital-twin residual bound: |Q_twin - Q_meas| <= 1.80 MW/100 h.
pub const TWIN_RESIDUAL_MW: f64 = 1.80;

/// Five-phase commissioning state machine:
/// Vacuum -> LeakHold -> PressureProof -> ThermalRamp -> SteadyRun.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommissionPhase {
    Vacuum,
    LeakHold,
    PressureProof,
    ThermalRamp,
    SteadyRun,
}

#[derive(Clone, Copy, Debug)]
pub struct CommissioningState {
    pub phase: CommissionPhase,
    pub p_base_pa: f64,
    pub leak_rate: f64,
    pub hold_drift_mpa_24h: f64,
    pub dp_core_mpa: f64,
    pub q_th_mw: f64,
    pub twin_residual_mw: f64,
}

impl CommissioningState {
    pub fn new() -> Self {
        CommissioningState {
            phase: CommissionPhase::Vacuum,
            p_base_pa: P_BASE_PA,
            leak_rate: LEAK_BOUND,
            hold_drift_mpa_24h: 0.0,
            dp_core_mpa: 0.0,
            q_th_mw: 0.0,
            twin_residual_mw: 0.0,
        }
    }

    /// Advance the state machine if the phase-acceptance bound is met.
    pub fn try_advance(&mut self) -> bool {
        let ok = match self.phase {
            CommissionPhase::Vacuum => self.p_base_pa <= P_BASE_PA,
            CommissionPhase::LeakHold => self.leak_rate <= LEAK_BOUND,
            CommissionPhase::PressureProof => {
                self.hold_drift_mpa_24h <= HOLD_DRIFT_MPA_24H
            }
            CommissionPhase::ThermalRamp => {
                self.dp_core_mpa <= DP_CORE_BOUND_MPA
                    && (self.q_th_mw - Q_TH_MW).abs() <= Q_TH_TOL_MW
            }
            CommissionPhase::SteadyRun => self.twin_residual_mw <= TWIN_RESIDUAL_MW,
        };
        if ok {
            self.phase = match self.phase {
                CommissionPhase::Vacuum => CommissionPhase::LeakHold,
                CommissionPhase::LeakHold => CommissionPhase::PressureProof,
                CommissionPhase::PressureProof => CommissionPhase::ThermalRamp,
                CommissionPhase::ThermalRamp => CommissionPhase::SteadyRun,
                CommissionPhase::SteadyRun => CommissionPhase::SteadyRun,
            };
        }
        ok
    }
}

impl Default for CommissioningState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn astm_and_commissioning() {
        assert!(astm_e1004_ok(101.6, 0.08, 350.0));
        assert!(!astm_e1004_ok(100.0, 0.08, 350.0));
        assert!(astm_e1681_ok(50.0));
        assert!(!astm_e1681_ok(40.0));
        assert!(astm_f1624_ok(0.80, 1.0));
        let mut st = CommissioningState::new();
        for _ in 0..5 {
            st.q_th_mw = Q_TH_MW;
            assert!(st.try_advance());
        }
        assert_eq!(st.phase, CommissionPhase::SteadyRun);
    }
}
