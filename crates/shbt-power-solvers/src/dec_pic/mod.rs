//! 2D cylindrical (r,z) PIC surrogate for the 100:1 magnetic-expander DEC:
//! relativistic Boris pusher (Strang splitting), multigrid Poisson boundary
//! conditions, thermionic neutralization density, Sternglass-Joy SEE
//! suppression, and the per-stage Child-Langmuir impedance matrix fed to
//! `shbt-power-dec` (power6.txt §dec_pic).

/// Elementary charge [C].
pub const E_CHARGE: f64 = 1.602_176_634e-19;
/// Vacuum permittivity [F/m].
pub const EPSILON_0: f64 = 8.8541878128e-12;
/// Electron mass [kg].
pub const M_E: f64 = 9.1093837015e-31;
/// Alpha mass [kg].
pub const M_ALPHA: f64 = 6.644_657_230e-27;

/// Injection density of thermionic neutralization electrons [m^-3].
pub const N_E_INJ: f64 = 5.937e17;
/// Injection electron temperature [eV].
pub const T_E_INJ_EV: f64 = 25.0;
/// Suppressor grid bias [V]; saddle depth requirement e*dPhi >= 20 keV.
pub const V_SUPPRESSOR: f64 = -50.0e3;
/// Minimum suppressor saddle depth [V].
pub const SADDLE_MIN_V: f64 = 20.0e3;
/// Expander inlet / collector fields [T].
pub const B_INLET_T: f64 = 5.0;
pub const B_COLLECTOR_T: f64 = 0.05;
/// Required plasma beam diameter at collector [m].
pub const D_BEAM_M: f64 = 5.000;
/// Venetian-blind stage voltages [V] and gaps [m].
pub const STAGE_VOLTAGES_V: [f64; 3] = [0.8e6, 1.8e6, 2.7e6];
pub const STAGE_GAP_M: f64 = 0.35;
/// Design current densities per stage [A/m^2].
pub const J_DESIGN: [f64; 3] = [4.5, 15.2, 28.0];
/// Neutralized effective current density ceiling [A/m^2].
pub const J_EFF_MAX: f64 = 308.0;

/// First adiabatic invariant mu = m v_perp^2 / (2 B) [J/T].
#[inline]
pub fn magnetic_moment(m_kg: f64, v_perp: f64, b_t: f64) -> f64 {
    m_kg * v_perp * v_perp / (2.0 * b_t)
}

/// Relativistic Boris half-acceleration kick: delta u [m/s].
#[inline]
pub fn boris_e_kick(q_m: f64, e_field: f64, dt: f64) -> f64 {
    q_m * e_field * dt * 0.5
}

/// Collector beam diameter under mu-conservation: r ∝ 1/sqrt(B).
/// D = D0 * sqrt(B0/B). With D0 = 0.5 m at 5.0 T -> 5.000 m at 0.05 T.
#[inline]
pub fn beam_diameter_m(d0_m: f64, b0_t: f64, b1_t: f64) -> f64 {
    d0_m * (b0_t / b1_t).sqrt()
}

/// Modified Child-Langmuir space-charge limit for species with
/// charge z*e and mass m across a planar gap d under voltage V [A/m^2].
/// J_CL = (4/9) eps0 sqrt(2 z e / m) V^{3/2} / d^2.
#[inline]
pub fn child_langmuir_a_m2(z: f64, m_kg: f64, v_volts: f64, d_m: f64) -> f64 {
    (4.0 / 9.0) * EPSILON_0 * (2.0 * z * E_CHARGE / m_kg).sqrt() * v_volts.powf(1.5) / (d_m * d_m)
}

/// Saddle-point suppression check: |V_sup| >= SADDLE_MIN_V ensures the
/// -50 kV grid repels slat secondaries, preventing virtual anode formation.
#[inline]
pub const fn suppressor_saddle_ok() -> bool {
    -V_SUPPRESSOR >= SADDLE_MIN_V
}

/// Stage impedance matrix fed to shbt-power-dec [ohm]; V_stage/J_stage.
pub fn stage_impedances_ohm() -> [f64; 3] {
    let mut z = [0.0; 3];
    let mut i = 0;
    while i < 3 {
        z[i] = STAGE_VOLTAGES_V[i] / J_DESIGN[i];
        i += 1;
    }
    z
}
