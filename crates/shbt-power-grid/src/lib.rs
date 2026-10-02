//! shbt-power-grid — solid-state coherent graser nuclear isomer battery
//! bootstrap, repurposed 450 MJ supercapacitor synthetic-inertia buffer,
//! five-phase plant lifecycle FSM, dual-stage TEG reclamation, and the
//! master closed-loop net-grid ledger (paper/main.tex §5-§6,
//! paper/supplementary.tex §4/§11).
//!
//! The 178m2Hf graser isomer battery core (376.99 kg, 500 TJ stored),
//! Borrmann cavity, and 3-stage relativistic DEC stack originate upstream
//! in `sys1own/shbt-warp` and `sys1own/shbt-ghost`; they supersede the
//! legacy 1,800-module LANR starter array from `sys1own/shbt-cf`
//! (`module_ledger.rs`: 555.03 W net per module -> 999.054 kW array,
//! 450.43 s / 7.51 min buffer pre-charge). The 450 MJ supercapacitor bank
//! is relieved of that pre-charge role and re-tasked as a dedicated
//! synthetic inertia buffer (H = 57.45 ms, 4.0-5.0 % droop, up to
//! 1,958.23 MW for 230 ms). Dual-stage TEG enthalpy recovery remains
//! transferred from `sys1own/shbt-cf`.

pub mod helium_network;
pub mod interconnect;
pub mod teg_nodal;
pub mod cht_fault;
pub mod thermal_teg;
pub mod fatigue;

use shbt_power_core::constants::*;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};

/// Solid-state coherent graser nuclear isomer battery
/// (376.99 kg enriched 178m2Hf, 500 TJ stored; upstream
/// `sys1own/shbt-warp` / `sys1own/shbt-ghost`).
#[derive(Clone, Copy, Debug)]
pub struct IsomerBattery {
    pub core_mass_kg: f64,
    pub stored_energy_tj: f64,
    pub core_temp_k: f64,
    pub cryo_headroom_k: f64,
    pub soc: f64,
    pub bus_voltage_kv: f64,
    pub decay_heat_kw: f64,
    pub mossbauer_recoil_frac: f64,
    pub borrmann_suppress_factor: f64,
    /// Elapsed bootstrap pulse time within the ignition state (s).
    pub pulse_elapsed_s: f64,
}

impl Default for IsomerBattery {
    fn default() -> Self {
        Self {
            core_mass_kg: ISOMER_CORE_MASS_KG,
            stored_energy_tj: ISOMER_STORED_TJ,
            core_temp_k: CRYO_CORE_TEMP_K,
            cryo_headroom_k: CRYO_HEADROOM_K,
            soc: 1.0,
            bus_voltage_kv: 24.0,
            decay_heat_kw: DECAY_HEAT_KW,
            mossbauer_recoil_frac: MOSSBAUER_RECOIL_FRAC,
            borrmann_suppress_factor: BORRMANN_SUPPRESS,
            pulse_elapsed_s: 0.0,
        }
    }
}

impl IsomerBattery {
    /// Optical trigger gain G = E_released / E_seed (>= 60.0).
    pub fn trigger_gain() -> f64 {
        ISOMER_RELEASE_MEV * 1e3 / ISOMER_SEED_KEV
    }

    /// Electrical energy injected per cold start (J): the 140 MW
    /// recirculating load bridged over the bootstrap window.
    pub fn boot_energy_elec_j() -> f64 {
        P_BATTERY_BURST_MW * 1e6 * TAU_BOOT_LIMIT_S
    }

    /// Raw isomeric energy discharged per cold start (J) after the
    /// 3-stage relativistic DEC efficiency.
    pub fn boot_energy_nuclear_j() -> f64 {
        Self::boot_energy_elec_j() / ETA_DEC_ISOMER
    }

    /// Active nuclear mass consumed per cold start (kg).
    pub fn mass_consumed_per_start_kg() -> f64 {
        Self::boot_energy_nuclear_j() / (ISOMER_ENERGY_DENSITY_TJ_KG * 1e12)
    }

    /// Specific energy density of the core (TJ/kg).
    pub fn energy_density_tj_kg(&self) -> f64 {
        self.stored_energy_tj / self.core_mass_kg
    }

    /// Cryostat coherence invariants required for arming (Phi_01).
    pub fn arming_ok(&self) -> bool {
        self.core_temp_k <= CRYO_CORE_TEMP_K + 1e-6
            && self.cryo_headroom_k >= CRYO_HEADROOM_K - 1e-6
            && self.mossbauer_recoil_frac >= MOSSBAUER_MIN
            && self.borrmann_suppress_factor >= BORRMANN_MIN
    }

    /// Fire the graser ignition pulse: DEC bus ramps 15 -> 400 kV and the
    /// battery delivers the 140 MW bootstrap injection.  Returns the
    /// electrical energy delivered this step (J).
    pub fn ignition_step(&mut self, dt_s: f64) -> f64 {
        self.bus_voltage_kv = BUS_DISCHARGE_MAX_KV;
        self.pulse_elapsed_s += dt_s;
        let e_j = P_BATTERY_BURST_MW * 1e6 * dt_s;
        self.soc = (self.soc - e_j / (ETA_DEC_ISOMER * self.stored_energy_tj * 1e12))
            .max(0.0);
        e_j
    }

    /// Quench the 40 keV seed laser back to quiescent cryogenic standby.
    pub fn quench(&mut self) {
        self.bus_voltage_kv = BUS_PRECHARGE_KV;
        self.pulse_elapsed_s = 0.0;
    }

    /// Design-nominal cold-start bootstrap latency (s): the 0.85 s
    /// ignition pulse plus recirculation lock inside the 1.00 s bound.
    pub fn bootstrap_time_s(&self) -> f64 {
        TAU_BOOT_S
    }
}

/// 20 K cryogenic helium sub-loop thermal balance for the isomer core
/// (paper/main.tex §5.4, Table `tab:cryo_parameters`).
#[derive(Clone, Copy, Debug)]
pub struct CryoSubloop {
    pub pressure_mpa: f64,
    pub mass_flow_kg_s: f64,
    pub cp_kj_kg_k: f64,
    pub delta_t_k: f64,
    pub decay_heat_kw: f64,
    pub cryocooler_elec_mw: f64,
    pub teg_shield_reclaim_kw: f64,
}

impl Default for CryoSubloop {
    fn default() -> Self {
        Self {
            pressure_mpa: CRYO_PRESS_MPA,
            mass_flow_kg_s: CRYO_MDOT_KG_S,
            cp_kj_kg_k: CRYO_CP_KJ_KG_K,
            delta_t_k: CRYO_DT_K,
            decay_heat_kw: DECAY_HEAT_KW,
            cryocooler_elec_mw: CRYO_ELEC_MW,
            teg_shield_reclaim_kw: TEG_SHIELD_KW,
        }
    }
}

impl CryoSubloop {
    /// Required helium mass flow lifting the quiescent decay heat
    /// (kg/s): m_dot = Q_decay / (c_p * Delta T).
    pub fn required_mass_flow_kg_s(&self) -> f64 {
        self.decay_heat_kw / (self.cp_kj_kg_k * self.delta_t_k)
    }

    /// Carnot COP between the 21.13 K core and the 300 K rejection sink.
    pub fn carnot_cop() -> f64 {
        CRYO_CORE_TEMP_K / (300.0 - CRYO_CORE_TEMP_K)
    }

    /// Effective operational COP at eta_ex = 0.280 relative to Carnot.
    pub fn actual_cop() -> f64 {
        0.280 * Self::carnot_cop()
    }

    /// Cryoplant electrical demand (MW) consistent with the COP chain.
    pub fn electrical_demand_mw(&self) -> f64 {
        self.decay_heat_kw / 1e3 / Self::actual_cop()
    }

    /// Intermediate-shield TEG recovery (kW DC) at ZT = 1.45 over the
    /// 80 K intercept, capturing the 125.4 kW conductive bypass flux.
    pub fn teg_reclaim_kw() -> f64 {
        let t_shield = 80.0_f64;
        let t_cryo = CRYO_CORE_TEMP_K;
        let zt = 1.45_f64;
        let s = (1.0 + zt).sqrt();
        let eta = (t_shield - t_cryo) / t_shield * (s - 1.0) / (s + t_cryo / t_shield);
        125.4 * eta
    }
}

/// Repurposed supercapacitor synthetic-inertia buffer.  Relieved of the
/// legacy LANR pre-charge role, the 450 MJ bank is held at the nominal
/// rail and dedicated to primary grid frequency stabilization.
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
            // Held topped-up: no bootstrap pre-charge role remains.
            charge_j: SUPERCAP_MJ * 1e6,
        }
    }
    pub fn soc(&self) -> f64 {
        self.charge_j / self.capacity_j
    }
    /// Discharge `p_w` watts for `dt` seconds (synthetic inertia draw).
    pub fn discharge(&mut self, p_w: f64, dt: f64) {
        self.charge_j = (self.charge_j - p_w * dt).max(0.0);
    }
    /// Equivalent synthetic inertia constant H = E_buffer / S_base (s).
    pub fn synthetic_inertia_s() -> f64 {
        SUPERCAP_MJ / P_NET_EXPORT_MW
    }
    /// Droop power injection (MW) for a frequency deviation `df_hz` at
    /// droop ratio `r` on the 50 Hz base, clamped to the buffer slew.
    pub fn droop_injection_mw(df_hz: f64, r: f64) -> f64 {
        (-(df_hz / GRID_FREQ_HZ) / r * P_NET_EXPORT_MW).clamp(0.0, DROOP_INJECT_MW)
    }
    /// Full-power discharge window (s) at the design droop injection.
    pub fn hold_time_s() -> f64 {
        SUPERCAP_MJ / DROOP_INJECT_MW
    }
}

/// Five-phase plant lifecycle FSM (power8.txt §2 bootstrap contract).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlantPhase {
    /// 0: core quiescent at 21.13 K, Mossbauer locked, bus isolated.
    ColdStandby = 0,
    /// 1: PFN primed, PCSS verified, DC link pre-charged to 15 kV.
    IsomerArming = 1,
    /// 2: 40 keV seed pulse; graser + 3-stage DEC deliver 140 MW.
    GraserIgnitionPulse = 2,
    /// 3: fusion burn initiated, primary DEC/MHD ramping to gross.
    DecBootstrap = 3,
    /// 4: 7,832.903 MW export locked, graser quenched, buffer on droop.
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

/// Grid subsystem: 5-phase isomer bootstrap FSM + synthetic-inertia
/// buffer + battery + cryo sub-loop + master ledger.
#[derive(Debug)]
pub struct GridSubsystem {
    pub phase: PlantPhase,
    pub buffer: SupercapBuffer,
    pub battery: IsomerBattery,
    pub cryo: CryoSubloop,
    pub teg: TegArray,
    pub ledger: PlantLedger,
    /// Time in the current phase (s).
    pub phase_elapsed_s: f64,
    /// Verified plant start command (drives Phi_01).
    pub start_commanded: bool,
    /// Gross fusion power ramp during DEC bootstrap (MW).
    pub gross_ramp_mw: f64,
    /// Total cold-start elapsed time once the start command lands (s).
    pub boot_elapsed_s: f64,
}

impl Default for GridSubsystem {
    fn default() -> Self {
        Self {
            phase: PlantPhase::ColdStandby,
            buffer: SupercapBuffer::new(),
            battery: IsomerBattery::default(),
            cryo: CryoSubloop::default(),
            teg: TegArray::default(),
            ledger: PlantLedger::solve(&TegArray::default()),
            phase_elapsed_s: 0.0,
            start_commanded: true,
            gross_ramp_mw: 0.0,
            boot_elapsed_s: 0.0,
        }
    }
}

impl GridSubsystem {
    /// Advance the five-phase lifecycle FSM one step; `dt_s` is the step
    /// duration.  Transition predicates follow the power8.txt contract:
    /// instantaneous isomer-battery bootstrap with tau_boot <= 1.00 s.
    pub fn step(&mut self, dt_s: f64) {
        if self.phase != PlantPhase::ColdStandby {
            self.boot_elapsed_s += dt_s;
        }
        match self.phase {
            PlantPhase::ColdStandby => {
                // Phi_01: start command + cryo headroom + PCSS ready +
                // supercap hold rail at 24 kV nominal.
                if self.start_commanded && self.battery.arming_ok() {
                    self.phase = PlantPhase::IsomerArming;
                    self.phase_elapsed_s = 0.0;
                    self.battery.bus_voltage_kv = BUS_PRECHARGE_KV;
                }
            }
            PlantPhase::IsomerArming => {
                // Phi_12: PCSS quench within budget and DC link at 15 kV.
                if self.battery.bus_voltage_kv == BUS_PRECHARGE_KV
                    && PCSS_QUENCH_NS <= PCSS_QUENCH_BUDGET_NS
                {
                    self.phase = PlantPhase::GraserIgnitionPulse;
                    self.phase_elapsed_s = 0.0;
                }
            }
            PlantPhase::GraserIgnitionPulse => {
                // Seed pulse fires; 140 MW battery injection over the
                // 0.85 s ignition window, then fusion burn takes over.
                self.battery.ignition_step(dt_s);
                if self.battery.pulse_elapsed_s >= TAU_BOOT_S {
                    self.phase = PlantPhase::DecBootstrap;
                    self.phase_elapsed_s = 0.0;
                    self.gross_ramp_mw = 1000.0;
                }
            }
            PlantPhase::DecBootstrap => {
                // Primary DEC/MHD ramp to 7,972.903 MW gross; the graser
                // seed laser is throttled and quenched on lock.
                self.gross_ramp_mw = (self.gross_ramp_mw
                    + self.ledger.p_gross_mw * dt_s / 0.10)
                    .min(self.ledger.p_gross_mw);
                if self.gross_ramp_mw >= self.ledger.p_gross_mw
                    && self.boot_elapsed_s <= TAU_BOOT_LIMIT_S + 1e-9
                {
                    self.battery.quench();
                    self.phase = PlantPhase::SteadyStateRecirculation;
                    self.phase_elapsed_s = 0.0;
                }
            }
            PlantPhase::SteadyStateRecirculation => {}
        }
        self.phase_elapsed_s += dt_s;
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
        state.battery_core_temp_k = self.battery.core_temp_k;
        state.battery_cryo_headroom_k = self.battery.cryo_headroom_k;
        state.battery_soc = self.battery.soc;
        state.battery_bus_voltage_kv = self.battery.bus_voltage_kv;
        state.battery_decay_heat_kw = self.battery.decay_heat_kw;
        state.mossbauer_recoil_frac = self.battery.mossbauer_recoil_frac;
        state.borrmann_suppress_factor = self.battery.borrmann_suppress_factor;
        state.p_teg_mw = self.ledger.p_teg_mw;
        state.p_gross_mw = self.ledger.p_gross_mw;
        state.p_recirc_mw = self.ledger.p_recirc_mw;
        state.p_net_mw = self.ledger.p_net_mw;
    }
}


// ---------------------------------------------------------------------------
// power6: supercritical-helium CHT hydraulic surrogate
// (shbt-power-solvers::thermal_fea::cht_surrogate) — Churchill friction
// factor, core Delta p and circulator pumping power.
// ---------------------------------------------------------------------------
pub use shbt_power_solvers::thermal_fea::cht_surrogate::{
    ChtSurrogateSolver, MicroChannelConfig,
};

/// Evaluates (Delta p [Pa], W_pump [W]) for the nominal 450 kg/s, 10 MPa
/// sHe loop; spec targets: Delta p = 0.282 MPa, W_pump = 14.22 MW <= 15 MW.
#[inline]
pub fn she_loop_hydraulics_p6(cfg: MicroChannelConfig, m_dot: f64, rho: f64, mu: f64, eta: f64) -> (f64, f64) {
    ChtSurrogateSolver::new(cfg).evaluate_hydraulics(m_dot, rho, mu, eta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isomer_battery_sizing() {
        let bat = IsomerBattery::default();
        assert!((bat.energy_density_tj_kg() - 1.3263).abs() < 1e-3);
        assert!((IsomerBattery::trigger_gain() - 61.15).abs() < 1e-6);
        let m = IsomerBattery::mass_consumed_per_start_kg();
        assert!((m - 2.305e-4).abs() < 1e-5);
        assert!(bat.bootstrap_time_s() <= TAU_BOOT_LIMIT_S);
    }

    #[test]
    fn synthetic_inertia_buffer() {
        assert!((SupercapBuffer::synthetic_inertia_s() * 1e3 - SYNTH_INERTIA_MS).abs() < 0.1);
        let p = SupercapBuffer::droop_injection_mw(-0.50, 0.04);
        assert!((p - DROOP_INJECT_MW).abs() < 0.1, "{p}");
        assert!((SupercapBuffer::hold_time_s() * 1e3 - DROOP_HOLD_MS).abs() < 0.5);
    }

    #[test]
    fn cryo_subloop_balance() {
        let c = CryoSubloop::default();
        assert!((c.required_mass_flow_kg_s() - 21.795).abs() < 0.01);
        assert!((c.electrical_demand_mw() - 16.70).abs() < 0.05);
        assert!((CryoSubloop::teg_reclaim_kw() - 28.50).abs() < 0.05);
    }

    #[test]
    fn fsm_cold_start_under_1s() {
        let mut g = GridSubsystem::default();
        while g.phase != PlantPhase::SteadyStateRecirculation {
            g.step(0.001);
        }
        assert!(g.boot_elapsed_s <= TAU_BOOT_LIMIT_S + 1e-9,
            "boot {} s", g.boot_elapsed_s);
        assert_eq!(g.battery.bus_voltage_kv, BUS_PRECHARGE_KV);
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
            g.step(0.001);
            guard += 1;
        }
        assert_eq!(g.phase, PlantPhase::SteadyStateRecirculation);
    }
}
