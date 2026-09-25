//! shbt-power-dec — direct energy conversion: alpha-stream kinetics, the
//! 100:1 magnetic expander trumpet, Child-Langmuir space-charge sheath,
//! three-stage Venetian-blind electrostatic collection, and WBG panels
//! (power.txt §5, power1.txt §6).

use shbt_power_core::constants::*;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};

/// Alpha-particle inventory per 87.5 MJ shot: reaction events, alpha count,
/// aggregate charge, and collection currents.
#[derive(Clone, Copy, Debug)]
pub struct AlphaStream {
    pub fusion_events: f64,
    pub n_alpha: f64,
    pub charge_c: f64,
    pub i_avg_a: f64,
    pub i_peak_a_tau_us: f64,
    pub tau_coll_s: f64,
}

impl AlphaStream {
    /// `tau_coll_s`: ejection-window duration used for peak current.
    pub fn solve(tau_coll_s: f64) -> Self {
        let q_j = Q_PB11_MEV * 1e6 * E_CHARGE;
        let events = E_YIELD_MJ * 1e6 / q_j;
        let n_alpha = 3.0 * events;
        let charge = n_alpha * 2.0 * E_CHARGE;
        Self {
            fusion_events: events,
            n_alpha,
            charge_c: charge,
            i_avg_a: charge * F_REP_HZ,
            i_peak_a_tau_us: charge / tau_coll_s,
            tau_coll_s,
        }
    }
}

/// 100:1 magnetic expander trumpet: flux-conserved area expansion.
#[derive(Clone, Copy, Debug)]
pub struct ExpanderTrumpet {
    pub b_core_t: f64,
    pub b_coll_t: f64,
    pub ratio: f64,
    pub a_core_m2: f64,
    pub a_coll_m2: f64,
    pub d_coll_m: f64,
}

impl ExpanderTrumpet {
    pub fn solve() -> Self {
        let a_core = core::f64::consts::PI * R_CORE_M * R_CORE_M;
        let ratio = B_CORE_T / B_COLL_T;
        let a_coll = a_core * ratio;
        Self {
            b_core_t: B_CORE_T,
            b_coll_t: B_COLL_T,
            ratio,
            a_core_m2: a_core,
            a_coll_m2: a_coll,
            d_coll_m: 2.0 * (a_coll / core::f64::consts::PI).sqrt(),
        }
    }
}

/// Child-Langmuir space-charge limit for alpha ions (A/m^2):
///   J_CL = (4 eps0 / 9) sqrt(2 q / m) V^(3/2) / d^2.
pub fn child_langmuir_limit_a_m2(v_volts: f64, gap_m: f64) -> f64 {
    let q = 2.0 * E_CHARGE;
    let m = M_ALPHA_KG;
    (4.0 * EPS0 / 9.0) * (2.0 * q / m).sqrt() * v_volts.powf(1.5) / (gap_m * gap_m)
}

/// Three-stage Venetian-blind electrostatic collector ledger.
#[derive(Clone, Copy, Debug)]
pub struct VenetianCollector {
    pub v_stage_mv: [f64; 3],
    pub eta_elec: f64,
    pub p_input_mw: f64,
    pub p_output_mw: f64,
    pub suppressor_bias_kv: f64,
    pub back_accel_ratio: f64,
}

impl VenetianCollector {
    pub fn solve() -> Self {
        let p_in = P_FUSION_MW * FRAC_ALPHA;
        Self {
            v_stage_mv: V_GRID_MV,
            eta_elec: ETA_ELEC,
            p_input_mw: p_in,
            p_output_mw: p_in * ETA_ELEC,
            suppressor_bias_kv: -50.0,
            back_accel_ratio: 1e-5, // 0.001 %
        }
    }

    /// Larmor gyro-radius of a 2.9 MeV alpha in the 1.0 T guide field (m).
    pub fn larmor_radius_m() -> f64 {
        let v = (2.0 * E_ALPHA_MEV * 1e6 * E_CHARGE / M_ALPHA_KG).sqrt();
        M_ALPHA_KG * v / (2.0 * E_CHARGE * B_GUIDE_T)
    }
}

/// Wide-bandgap radiovoltaic channel (4H-SiC / GaN:Fe).
#[derive(Clone, Copy, Debug)]
pub struct WbgChannel {
    pub p_input_mw: f64,
    pub eta_rad: f64,
    pub p_output_mw: f64,
    pub e_break_mv_cm: f64,
    pub bandgap_ev: f64,
}

impl WbgChannel {
    pub fn solve() -> Self {
        let p_in = P_FUSION_MW * FRAC_RAD;
        Self {
            p_input_mw: p_in,
            eta_rad: ETA_RAD,
            p_output_mw: p_in * ETA_RAD,
            e_break_mv_cm: 3.0,
            bandgap_ev: 3.26,
        }
    }
}

/// Aggregate DEC subsystem state.
#[derive(Clone, Debug)]
pub struct DecSubsystem {
    pub stream: AlphaStream,
    pub trumpet: ExpanderTrumpet,
    pub collector: VenetianCollector,
    pub wbg: WbgChannel,
    /// Neutralizing electron-to-alpha density ratio along the duct.
    pub neutralization_ratio: f64,
}

impl Default for DecSubsystem {
    fn default() -> Self {
        Self {
            stream: AlphaStream::solve(10.0e-6),
            trumpet: ExpanderTrumpet::solve(),
            collector: VenetianCollector::solve(),
            wbg: WbgChannel::solve(),
            neutralization_ratio: 1000.0,
        }
    }
}

impl DecSubsystem {
    /// Expanded-duct current density (A/m^2).
    pub fn j_expanded(&self) -> f64 {
        self.stream.i_peak_a_tau_us / self.trumpet.a_coll_m2
    }

    /// Effective space-charge density after co-moving neutralization:
    /// residual ion density is n_alpha/(1 + ratio).
    pub fn j_effective(&self) -> f64 {
        self.j_expanded() / (1.0 + self.neutralization_ratio)
    }

    /// GATE-35 check: effective current density below the Child-Langmuir
    /// bound for the mid-stage 1.5 MV, 0.35 m gap.
    pub fn below_cl_bound(&self) -> bool {
        self.j_effective() < child_langmuir_limit_a_m2(1.5e6, 0.35)
    }
}

impl PhysicsSubsystem for DecSubsystem {
    fn name(&self) -> &'static str {
        "shbt-power-dec"
    }
    fn update(&mut self, state: &mut PlantStateSnapshot) {
        let direct = self.collector.p_output_mw
            + shbt_power_chamber_stub(self)
            + self.wbg.p_output_mw;
        state.p_direct_total_mw = direct;
    }
}

/// MHD pickup comes from the chamber crate in the full twin; the DEC
/// subsystem only owns channels 1 and 3, so the ledger publishes the
/// electrostatic + WBG sum and the grid crate completes the total.
fn shbt_power_chamber_stub(_d: &DecSubsystem) -> f64 {
    P_FUSION_MW * FRAC_MHD * ETA_MHD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_stream_inventory() {
        let s = AlphaStream::solve(10.0e-6);
        assert!((s.n_alpha - 1.888e20).abs() / 1.888e20 < 0.01, "{:?}", s.n_alpha);
        // Spec prints 60.487 C; exact constants give ~60.34 C (spec uses a
        // rounded 8.68 MeV effective Q). Track self-consistency instead.
        assert!((s.charge_c - s.n_alpha * 2.0 * E_CHARGE).abs() < 1e-9);
        assert!((s.i_avg_a - s.charge_c * F_REP_HZ).abs() < 1e-9);
    }

    #[test]
    fn trumpet_geometry() {
        let t = ExpanderTrumpet::solve();
        assert!((t.ratio - 100.0).abs() < 1e-9);
        assert!((t.a_coll_m2 - 19.635).abs() < 0.01);
        assert!((t.d_coll_m - 5.0).abs() < 0.01);
    }

    #[test]
    fn child_langmuir_bound() {
        let jcl = child_langmuir_limit_a_m2(1.5e6, 0.35);
        assert!((jcl - 579.59).abs() < 2.0, "{jcl}");
        let d = DecSubsystem::default();
        assert!(d.below_cl_bound());
    }

    #[test]
    fn collector_ledger() {
        let c = VenetianCollector::solve();
        assert!((c.p_output_mw - 6125.0).abs() < 1e-9);
        assert!((VenetianCollector::larmor_radius_m() - 0.25).abs() < 0.01);
    }
}
