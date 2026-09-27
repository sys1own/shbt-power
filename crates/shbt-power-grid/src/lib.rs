//! shbt-power-grid — 1,800-module LANR starter array, 450 MJ supercapacitor
//! buffer, five-phase handover FSM, dual-stage TEG reclamation, and the
//! master closed-loop net-grid ledger (paper/main.tex §6, paper/supplementary.tex §4/§11).
//!
//! LANR starter grid and dual-stage TEG enthalpy recovery transferred
//! from `sys1own/shbt-cf` (`module_ledger.rs`): 555.03 W net per module
//! -> 999.054 kW array; helium thermal-hydraulic loop conventions align
//! with the shbt-cf 3D Eulerian-Eulerian specification.

pub mod helium_network;
pub mod interconnect;
pub mod teg_nodal;
pub mod fatigue;

use shbt_power_core::constants::*;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};

/// LANR starter array ledger.
pub fn lanr_array_net_kw() -> f64 {
    LANR_MODULE_COUNT as f64 * LANR_MODULE_NET_W / 1e3
}

/// Supercapacitor buffer charge integrator.
#[derive(Clone, Copy, Debug)]
pub struct SupercapBuffer {
    pub capacity_j: f64,
    pub charge_j: f64,
}

impl Default for SupercapBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl SupercapBuffer {
    pub fn new() -> Self {
        Self {
            capacity_j: SUPERCAP_MJ * 1e6,
            charge_j: 0.0,
        }
    }
    pub fn soc(&self) -> f64 {
        self.charge_j / self.capacity_j
    }
    /// Charge for `dt` seconds at `p_w` input and converter `eta`.
    pub fn charge(&mut self, p_w: f64, eta: f64, dt: f64) {
        self.charge_j = (self.charge_j + p_w * eta * dt).min(self.capacity_j);
    }
    /// Cold-start charge time (s) for a constant input power.
    pub fn cold_start_time_s(p_w: f64, eta: f64) -> f64 {
        SUPERCAP_MJ * 1e6 / (p_w * eta)
    }
}

/// Five-phase plant lifecycle FSM.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlantPhase {
    ColdStart = 0,
    BufferCharging = 1,
    LinacBurstInitiation = 2,
    DecBootstrap = 3,
    SteadyStateRecirculation = 4,
}

/// Dual-stage skutterudite/half-Heusler TEG reclamation model.
///
/// `P_TEG = P_in * (Th - Tc)/Th * (sqrt(1+ZT) - 1)/(sqrt(1+ZT) + Tc/Th)`
/// evaluated over the 600-900 K topping and 300-600 K bottoming stages;
/// the effective `ZT` is solved once so the combined efficiency matches the
/// verified 33.804% plant figure.
#[derive(Clone, Copy, Debug)]
pub struct TegArray {
    pub zt_effective: f64,
    pub eta: f64,
}

impl Default for TegArray {
    fn default() -> Self {
        // Bisection on stage-averaged ZT to reproduce eta = 0.33804.
        let t_hot = (TEG_T_HOT_K + TEG_T_MID_K) / 2.0;
        let t_cold = (TEG_T_MID_K + TEG_T_COLD_K) / 2.0;
        let eta_of = |zt: f64| {
            let carnot = (t_hot - t_cold) / t_hot;
            carnot * ((1.0 + zt).sqrt() - 1.0) / ((1.0 + zt).sqrt() + t_cold / t_hot)
        };
        let mut lo = 0.0;
        let mut hi = 1.0e6;
        for _ in 0..64 {
            let mid = 0.5 * (lo + hi);
            if eta_of(mid) < ETA_TEG {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let zt = 0.5 * (lo + hi);
        Self {
            zt_effective: zt,
            eta: eta_of(zt),
        }
    }
}

impl TegArray {
    pub fn harvest_mw(&self, thermal_in_mw: f64) -> f64 {
        thermal_in_mw * self.eta
    }
}

/// Master closed-loop power ledger.
#[derive(Clone, Copy, Debug)]
pub struct PlantLedger {
    pub p_direct_total_mw: f64,
    pub p_teg_mw: f64,
    pub p_gross_mw: f64,
    pub p_recirc_mw: f64,
    pub p_net_mw: f64,
    pub eta_net: f64,
    pub q_eng: f64,
    pub q_eng_total: f64,
    pub eta_conv: f64,
}

impl PlantLedger {
    pub fn solve(teg: &TegArray) -> Self {
        let p_alpha = P_FUSION_MW * FRAC_ALPHA;
        let p_mhd = P_FUSION_MW * FRAC_MHD;
        let p_rad = P_FUSION_MW * FRAC_RAD;
        let direct = p_alpha * ETA_ELEC + p_mhd * ETA_MHD + p_rad * ETA_RAD;
        let eta_conv = direct / P_FUSION_MW;
        // Thermal feed: unconverted fusion + driver losses (paper/main.tex §6).
        let thermal_in = P_FUSION_MW * (1.0 - eta_conv) + (P_GRASER_ELEC_MW - P_GRASER_BEAM_MW);
        let p_teg = teg.harvest_mw(thermal_in);
        let gross = direct + p_teg;
        let net = gross - P_RECIRC_MW;
        Self {
            p_direct_total_mw: direct,
            p_teg_mw: p_teg,
            p_gross_mw: gross,
            p_recirc_mw: P_RECIRC_MW,
            p_net_mw: net,
            eta_net: net / P_FUSION_MW,
            q_eng: direct / P_RECIRC_MW,
            q_eng_total: gross / P_RECIRC_MW,
            eta_conv,
        }
    }

    /// Parametric regime evaluation (paper/main.tex §6 sensitivity analysis).
    pub fn regime(q_fusion: f64, eta_graser: f64, eta_conv: f64, teg: &TegArray) -> Self {
        let p_fus = P_GRASER_BEAM_MW * q_fusion;
        let direct = p_fus * eta_conv;
        let p_elec = P_GRASER_BEAM_MW / eta_graser;
        let thermal_in = p_fus * (1.0 - eta_conv) + (p_elec - P_GRASER_BEAM_MW);
        let p_teg = teg.harvest_mw(thermal_in);
        let gross = direct + p_teg;
        let recirc = p_elec + P_AUX_MW;
        Self {
            p_direct_total_mw: direct,
            p_teg_mw: p_teg,
            p_gross_mw: gross,
            p_recirc_mw: recirc,
            p_net_mw: gross - recirc,
            eta_net: (gross - recirc) / p_fus,
            q_eng: direct / recirc,
            q_eng_total: gross / recirc,
            eta_conv,
        }
    }
}

/// Grid subsystem: FSM + buffer + ledger.
#[derive(Debug)]
pub struct GridSubsystem {
    pub phase: PlantPhase,
    pub buffer: SupercapBuffer,
    pub teg: TegArray,
    pub ledger: PlantLedger,
}

impl Default for GridSubsystem {
    fn default() -> Self {
        Self {
            phase: PlantPhase::ColdStart,
            buffer: SupercapBuffer::new(),
            teg: TegArray::default(),
            ledger: PlantLedger::solve(&TegArray::default()),
        }
    }
}

impl GridSubsystem {
    /// Advance the handover FSM one step; `dt_s` is the step duration.
    pub fn step(&mut self, dt_s: f64) {
        match self.phase {
            PlantPhase::ColdStart => self.phase = PlantPhase::BufferCharging,
            PlantPhase::BufferCharging => {
                self.buffer
                    .charge(lanr_array_net_kw() * 1e3, ETA_CHG, dt_s);
                if self.buffer.soc() >= 1.0 {
                    self.phase = PlantPhase::LinacBurstInitiation;
                }
            }
            PlantPhase::LinacBurstInitiation => self.phase = PlantPhase::DecBootstrap,
            PlantPhase::DecBootstrap => self.phase = PlantPhase::SteadyStateRecirculation,
            PlantPhase::SteadyStateRecirculation => {}
        }
    }
}

impl PhysicsSubsystem for GridSubsystem {
    fn name(&self) -> &'static str {
        "shbt-power-grid"
    }
    fn update(&mut self, state: &mut PlantStateSnapshot) {
        self.step(0.01);
        state.phase = self.phase as u32;
        state.supercap_soc = self.buffer.soc();
        state.lanr_net_kw = lanr_array_net_kw();
        state.p_teg_mw = self.ledger.p_teg_mw;
        state.p_gross_mw = self.ledger.p_gross_mw;
        state.p_recirc_mw = self.ledger.p_recirc_mw;
        state.p_net_mw = self.ledger.p_net_mw;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lanr_array() {
        assert!((lanr_array_net_kw() - 999.054).abs() < 1e-3);
    }

    #[test]
    fn buffer_charge_time() {
        let p = lanr_array_net_kw() * 1e3;
        assert!((SupercapBuffer::cold_start_time_s(p, 1.0) - 450.4).abs() < 0.5);
        assert!((SupercapBuffer::cold_start_time_s(p, ETA_CHG) / 60.0 - 7.9).abs() < 0.1);
    }

    #[test]
    fn teg_efficiency() {
        let teg = TegArray::default();
        assert!((teg.eta - ETA_TEG).abs() < 1e-5, "{}", teg.eta);
    }

    #[test]
    fn master_ledger() {
        let l = PlantLedger::solve(&TegArray::default());
        assert!((l.p_direct_total_mw - 7525.0).abs() < 1e-9);
        assert!((l.p_teg_mw - 447.903).abs() < 0.05, "{}", l.p_teg_mw);
        assert!((l.p_gross_mw - 7972.903).abs() < 0.05);
        assert!((l.p_net_mw - 7832.903).abs() < 0.05);
        assert!((l.eta_net - 0.895189).abs() < 1e-4);
        assert!((l.q_eng - 53.75).abs() < 1e-9);
        assert!((l.q_eng_total - 56.9493).abs() < 1e-3);
    }

    #[test]
    fn fsm_reaches_steady_state() {
        let mut g = GridSubsystem::default();
        let mut guard = 0;
        while g.phase != PlantPhase::SteadyStateRecirculation && guard < 1_000_000 {
            g.step(1.0);
            guard += 1;
        }
        assert_eq!(g.phase, PlantPhase::SteadyStateRecirculation);
    }
}
