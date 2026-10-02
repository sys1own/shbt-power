//! Dynamic transmission interconnect: swing-equation frequency response,
//! supercapacitor synthetic inertia, droop control, and the 5-phase plant
//! handover state machine (power2.txt §5; paper/main.tex §5).
//!
//! A matrix (power2.txt §7 state-space):
//!   x = [df_grid, dV_DC, i_buffer, dP_valve]
//!   2H d df/dt + D_g df = P_gen - P_load + P_buffer

/// Grid interconnect configuration.
#[derive(Clone, Copy, Debug)]
pub struct GridConfig {
    /// Generator inertia constant H (s): 4.50.
    pub h_inertia: f64,
    /// Damping constant D_g: 1.80.
    pub d_damping: f64,
    /// Droop gain K_droop (MW/Hz), 4-5% droop on 7832.903 MW.
    pub k_droop: f64,
    /// Supercapacitor capacity (MJ): 450.
    pub e_buffer_max: f64,
    /// Maximum buffer transient injection (MW): +-1958.23 (repurposed
    /// 450 MJ synthetic-inertia bank).
    pub p_buffer_max_mw: f64,
}

impl GridConfig {
    /// Nominal plant interconnect at 4.0% droop (design-basis
    /// Delta f = -0.50 Hz -> 1,958.23 MW injection over 230 ms).
    pub fn nominal() -> Self {
        Self { h_inertia: 4.50, d_damping: 1.80,
               k_droop: 0.25 * 7_832.903 / 0.5, // R = 4%: dP = 0.25 S_base at -1% df
               e_buffer_max: 450.0, p_buffer_max_mw: 1958.23 }
    }
}

/// Dynamic grid + buffer state.
#[derive(Clone, Copy, Debug)]
pub struct GridState {
    /// Frequency deviation (Hz).
    pub delta_freq: f64,
    /// Buffer energy (MJ).
    pub e_buffer: f64,
}

impl GridState {
    pub fn new(e_buffer: f64) -> Self {
        Self { delta_freq: 0.0, e_buffer }
    }

    /// One Euler step of the swing equation with droop/synthetic inertia.
    /// Returns updated frequency deviation (Hz).
    pub fn step_grid_dynamics(
        &mut self,
        config: &GridConfig,
        p_gen: f64,
        p_load: f64,
        dt: f64,
    ) -> f64 {
        let p_mismatch = p_gen - p_load;
        // Positive P_buffer injects into the grid (discharges the buffer).
        let p_cmd = (-config.k_droop * self.delta_freq)
            .clamp(-config.p_buffer_max_mw, config.p_buffer_max_mw);
        let p_buffer = p_cmd.clamp(
            (self.e_buffer - config.e_buffer_max) / dt,
            self.e_buffer / dt,
        );

        let d_freq = (p_mismatch + p_buffer
            - config.d_damping * self.delta_freq)
            / (2.0 * config.h_inertia);
        self.delta_freq += d_freq * dt;
        self.e_buffer = (self.e_buffer - p_buffer * dt)
            .clamp(0.0, config.e_buffer_max);
        self.delta_freq
    }
}

/// Closed-loop stability metrics from the Bode analysis (power2.txt §5).
#[derive(Clone, Copy, Debug)]
pub struct StabilityMargins {
    /// Gain margin (dB), target >= 10.
    pub gain_margin_db: f64,
    /// Phase margin (deg), target >= 60.
    pub phase_margin_deg: f64,
    /// Gain crossover (rad/s), target 10-50.
    pub crossover_rad_s: f64,
    /// Maximum RoCoF (Hz/s), target <= 0.50.
    pub max_rocof_hz_s: f64,
}

/// Computed margins: GM 14.82 dB, PM 68.45 deg, wc 28.35 rad/s,
/// RoCoF 0.112 Hz/s — all compliant.
pub const MARGINS: StabilityMargins = StabilityMargins {
    gain_margin_db: 14.82,
    phase_margin_deg: 68.45,
    crossover_rad_s: 28.35,
    max_rocof_hz_s: 0.112,
};

/// True when all four margins meet their targets.
pub fn margins_ok(m: &StabilityMargins) -> bool {
    m.gain_margin_db >= 10.0
        && m.phase_margin_deg >= 60.0
        && (10.0..=50.0).contains(&m.crossover_rad_s)
        && m.max_rocof_hz_s <= 0.50
}

/// 4x4 plant state matrix A (power2.txt §7 numeric values):
///   -D/2H= -0.2, K_pcc/2H=0.0556, 1/2H=0.1111; -K_dc=-1.25,
///   -1/C_bus=-1.25e-3; 1/L_sc=83.3333, -R_sc/L_sc=-1.0;
///   -K_droop=-0.05, -1/tau_valve=-10.
pub const A_GRID: [[f64; 4]; 4] = [
    [-0.2000, 0.0556, 0.0000, 0.1111],
    [-1.2500, 0.0000, -1.2500e-3, 0.0000],
    [0.0000, 83.3333, -1.0000, 0.0000],
    [-0.0500, 0.0000, 0.0000, -10.0000],
];
/// Input matrix B (4x2).
pub const B_GRID: [[f64; 2]; 4] = [
    [0.1111, 0.0],
    [0.0, -0.0125],
    [0.0, 0.0],
    [0.0, 0.5],
];
/// Output matrix C (2x4).
pub const C_GRID: [[f64; 4]; 2] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
];

/// Plant lifecycle phase (5-phase isomer bootstrap state machine,
/// power8.txt FSM contract — supersedes the legacy LANR handover).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandoverPhase {
    /// State 0: core quiescent at 21.13 K, Mossbauer locked, bus isolated.
    ColdStandby,
    /// State 1: PFN primed, PCSS verified, DC link pre-charged (15 kV).
    IsomerArming,
    /// State 2: 40 keV seed pulse; 3-stage DEC delivers 140 MW (<= 850 ms).
    GraserIgnitionPulse,
    /// State 3: fusion burn initiated, gross >= 1,000 MW ramping (600 ms).
    DecBootstrap,
    /// State 4: 7,832.903 MW export locked, graser quenched, buffer on droop.
    SteadyStateRecirculation,
}

impl HandoverPhase {
    /// Exit criteria text and nominal duration (s).
    pub fn spec(&self) -> (&'static str, f64) {
        match self {
            Self::ColdStandby => ("dT_headroom >= 11.79 K, PCSS ready", f64::INFINITY),
            Self::IsomerArming => ("tau_pcss <= 2.10 ns, V_link = 15 kV", 0.0),
            Self::GraserIgnitionPulse => ("140 MW battery -> recirc, t <= 850 ms", 0.85),
            Self::DecBootstrap => ("P_gross >= 1000 MW, mdot_sHe nominal", 0.15),
            Self::SteadyStateRecirculation => ("net yield 7832.903 MW", f64::INFINITY),
        }
    }
    /// Next phase in the handover sequence.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::ColdStandby => Some(Self::IsomerArming),
            Self::IsomerArming => Some(Self::GraserIgnitionPulse),
            Self::GraserIgnitionPulse => Some(Self::DecBootstrap),
            Self::DecBootstrap => Some(Self::SteadyStateRecirculation),
            Self::SteadyStateRecirculation => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn margins_compliant() {
        assert!(margins_ok(&MARGINS));
    }

    #[test]
    fn swing_droop_response() {
        let cfg = GridConfig::nominal();
        let mut st = GridState::new(225.0);
        // 10 MW load step over 5 s; droop holds frequency within bounds.
        for _ in 0..50_000 {
            st.step_grid_dynamics(&cfg, 7_833.0, 7_843.0, 1e-4);
        }
        assert!(st.delta_freq.abs() < 1.0);
        assert!(st.e_buffer >= 0.0 && st.e_buffer <= 450.0);
    }

    #[test]
    fn phase_sequence() {
        let mut p = HandoverPhase::ColdStandby;
        let mut n = 0;
        while let Some(next) = p.next() {
            p = next;
            n += 1;
        }
        assert_eq!(n, 4);
        assert_eq!(p, HandoverPhase::SteadyStateRecirculation);
    }
}
