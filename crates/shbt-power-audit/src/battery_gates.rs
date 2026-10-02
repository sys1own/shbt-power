//! Verification audit gates for solid-state graser isomer battery bootstrap.
//! Crate: crates/shbt-power-audit
//!
//! Upstream provenance: the 376.99 kg enriched 178m2Hf core, Borrmann
//! cavity, and 3-stage relativistic DEC stack originate in
//! `sys1own/shbt-warp` and `sys1own/shbt-ghost`; the 450 MJ synthetic
//! inertia buffer is repurposed from `sys1own/shbt-cf`.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuditStatus {
    Passed,
    Failed { reason: &'static str, value: f64, threshold: f64 },
}

impl AuditStatus {
    pub fn is_passed(&self) -> bool {
        matches!(self, AuditStatus::Passed)
    }
}

pub struct BatteryTelemetrySnapshot {
    pub core_mass_kg: f64,
    pub stored_energy_tj: f64,
    pub seed_photon_kev: f64,
    pub released_energy_mev: f64,
    pub dec_efficiency_pct: f64,
    pub pcss_quench_ns: f64,
    pub inductive_recovery_pct: f64,
    pub bootstrap_time_s: f64,
    /// Spec contract field (power8.txt): core temperature is carried in
    /// the snapshot even though no BAT gate reads it directly.
    #[allow(dead_code)]
    pub core_temp_k: f64,
    pub mossbauer_recoil_frac: f64,
    pub borrmann_factor: f64,
}

pub struct BatteryAuditMatrix;

impl BatteryAuditMatrix {
    pub const THRESHOLD_ENERGY_DENSITY_TJ_KG: f64 = 1.3263;
    pub const THRESHOLD_TRIGGER_GAIN: f64 = 60.0;
    pub const THRESHOLD_DEC_EFFICIENCY_PCT: f64 = 45.8;
    pub const THRESHOLD_PCSS_QUENCH_NS: f64 = 2.10;
    pub const THRESHOLD_INDUCTIVE_RECOVERY_PCT: f64 = 94.20;
    pub const THRESHOLD_COLD_START_TIME_S: f64 = 1.00;
    pub const THRESHOLD_MOSSBAUER_FRACTION: f64 = 0.74;
    pub const THRESHOLD_BORRMANN_FACTOR: f64 = 0.985;

    pub fn audit_all(data: &BatteryTelemetrySnapshot) -> [AuditStatus; 8] {
        [
            Self::gate_bat_01(data),
            Self::gate_bat_02(data),
            Self::gate_bat_03(data),
            Self::gate_bat_04(data),
            Self::gate_bat_05(data),
            Self::gate_bat_06(data),
            Self::gate_bat_07(data),
            Self::gate_bat_08(data),
        ]
    }

    /// GATE-BAT-01: Specific Energy Density >= 1.3263 TJ/kg
    pub fn gate_bat_01(d: &BatteryTelemetrySnapshot) -> AuditStatus {
        let density = d.stored_energy_tj / d.core_mass_kg;
        if density >= (Self::THRESHOLD_ENERGY_DENSITY_TJ_KG - 1e-4) {
            AuditStatus::Passed
        } else {
            AuditStatus::Failed {
                reason: "Insufficient energy density in isomer crystal",
                value: density,
                threshold: Self::THRESHOLD_ENERGY_DENSITY_TJ_KG,
            }
        }
    }

    /// GATE-BAT-02: Trigger Seed Gain G >= 60.0
    pub fn gate_bat_02(d: &BatteryTelemetrySnapshot) -> AuditStatus {
        let gain = (d.released_energy_mev * 1000.0) / d.seed_photon_kev;
        if gain >= Self::THRESHOLD_TRIGGER_GAIN {
            AuditStatus::Passed
        } else {
            AuditStatus::Failed {
                reason: "Trigger amplification gain below threshold",
                value: gain,
                threshold: Self::THRESHOLD_TRIGGER_GAIN,
            }
        }
    }

    /// GATE-BAT-03: Relativistic DEC Efficiency >= 45.8%
    pub fn gate_bat_03(d: &BatteryTelemetrySnapshot) -> AuditStatus {
        if d.dec_efficiency_pct >= Self::THRESHOLD_DEC_EFFICIENCY_PCT {
            AuditStatus::Passed
        } else {
            AuditStatus::Failed {
                reason: "3-stage relativistic DEC efficiency non-compliant",
                value: d.dec_efficiency_pct,
                threshold: Self::THRESHOLD_DEC_EFFICIENCY_PCT,
            }
        }
    }

    /// GATE-BAT-04: PCSS Crowbar Quench Latency <= 2.10 ns
    pub fn gate_bat_04(d: &BatteryTelemetrySnapshot) -> AuditStatus {
        if d.pcss_quench_ns <= Self::THRESHOLD_PCSS_QUENCH_NS {
            AuditStatus::Passed
        } else {
            AuditStatus::Failed {
                reason: "PCSS crowbar response too sluggish",
                value: d.pcss_quench_ns,
                threshold: Self::THRESHOLD_PCSS_QUENCH_NS,
            }
        }
    }

    /// GATE-BAT-05: Inductive Energy Recovery Ratio >= 94.20%
    pub fn gate_bat_05(d: &BatteryTelemetrySnapshot) -> AuditStatus {
        if d.inductive_recovery_pct >= Self::THRESHOLD_INDUCTIVE_RECOVERY_PCT {
            AuditStatus::Passed
        } else {
            AuditStatus::Failed {
                reason: "Inductive energy recovery below specification",
                value: d.inductive_recovery_pct,
                threshold: Self::THRESHOLD_INDUCTIVE_RECOVERY_PCT,
            }
        }
    }

    /// GATE-BAT-06: Cold Start Bootstrap Latency <= 1.00 s
    pub fn gate_bat_06(d: &BatteryTelemetrySnapshot) -> AuditStatus {
        if d.bootstrap_time_s <= Self::THRESHOLD_COLD_START_TIME_S {
            AuditStatus::Passed
        } else {
            AuditStatus::Failed {
                reason: "Cold start failed instantaneous 1.00 s threshold",
                value: d.bootstrap_time_s,
                threshold: Self::THRESHOLD_COLD_START_TIME_S,
            }
        }
    }

    /// GATE-BAT-07: Mossbauer Recoil-Free Fraction f_M >= 0.74
    pub fn gate_bat_07(d: &BatteryTelemetrySnapshot) -> AuditStatus {
        if d.mossbauer_recoil_frac >= Self::THRESHOLD_MOSSBAUER_FRACTION {
            AuditStatus::Passed
        } else {
            AuditStatus::Failed {
                reason: "Mossbauer lattice de-pinning detected (f_M low)",
                value: d.mossbauer_recoil_frac,
                threshold: Self::THRESHOLD_MOSSBAUER_FRACTION,
            }
        }
    }

    /// GATE-BAT-08: Borrmann Anomalous Transmission Factor epsilon_B >= 0.985
    pub fn gate_bat_08(d: &BatteryTelemetrySnapshot) -> AuditStatus {
        if d.borrmann_factor >= Self::THRESHOLD_BORRMANN_FACTOR {
            AuditStatus::Passed
        } else {
            AuditStatus::Failed {
                reason: "Borrmann suppression degraded; risk of thermal burst",
                value: d.borrmann_factor,
                threshold: Self::THRESHOLD_BORRMANN_FACTOR,
            }
        }
    }
}
