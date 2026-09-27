// crates/shbt-power-solvers/src/linac_bbu/reduced_order_wake.rs

use super::bbu_tracker::{BbuTracker, LinacElement, NUM_BUNCHES, T_RF};
use std::vec::Vec;

pub struct ReducedWakeMatrix {
    pub matrix: Vec<f64>, // Row-major (2,500 x 2,500)
}

#[derive(Debug, PartialEq, Eq)]
pub enum QualityGateStatus {
    Passed,
    FailedGate11BeamLoadingResidual,
    FailedGate12BbuEmittanceGrowth,
    FailedGate13PhaseJitterExceeded,
}

impl ReducedWakeMatrix {
    pub fn construct(elem: &LinacElement) -> Self {
        let mut matrix = Vec::with_capacity(NUM_BUNCHES * NUM_BUNCHES);
        matrix.resize(NUM_BUNCHES * NUM_BUNCHES, 0.0);

        for k in 0..NUM_BUNCHES {
            for j in 0..k {
                let tau = ((k - j) as f64) * T_RF;
                let w = BbuTracker::wake_green_function(elem, tau);
                matrix[k * NUM_BUNCHES + j] = w;
            }
        }

        Self { matrix }
    }

    /// Verifies GATE-11, GATE-12, and GATE-13 quality constraints
    pub fn verify_quality_gates(
        &self,
        max_energy_chirp: f64,
        emittance_growth_mm_mrad: f64,
        phase_jitter_fs: f64,
    ) -> QualityGateStatus {
        // GATE-11: Fundamental Beam-Loading Energy Spread Residual <= 1e-4
        if max_energy_chirp > 1.0e-4 {
            return QualityGateStatus::FailedGate11BeamLoadingResidual;
        }

        // GATE-12: Transverse BBU Emittance Growth <= 0.50 mm*mrad
        if emittance_growth_mm_mrad > 0.50 {
            return QualityGateStatus::FailedGate12BbuEmittanceGrowth;
        }

        // GATE-13: Wakefield Arrival Phase Jitter <= 50.0 fs
        if phase_jitter_fs > 50.0 {
            return QualityGateStatus::FailedGate13PhaseJitterExceeded;
        }

        QualityGateStatus::Passed
    }
}