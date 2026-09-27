//! 3D resistive Hall-MHD chamber surrogate (HLLC-Rusanov + Dedner GLM
//! divergence cleaning): fireball stagnation radius, MRT flute saturation,
//! and SiC crowbar back-EMF coupling (power6.txt §mhd_chamber).

/// Fireball kinetic energy per shot [J].
pub const E_FIREBALL_J: f64 = 13.125e6;
/// Vacuum chamber inner radius [m].
pub const R_CHAMBER_M: f64 = 2.20;
/// Minimum acceptable magnetic cushion [m].
pub const CUSHION_MIN_M: f64 = 0.50;
/// 1D unconstrained stagnation radius (GATE-43 baseline) [m].
pub const R_C_1D_M: f64 = 0.8631;
/// 3D flux-compression stagnation radius at B = 3.5 T [m] — the
/// volumetric 16/3 scaling of the 1D bound.
pub const R_C_3D_M: f64 = 1.508;
/// Maximum MRT spike protrusion across m = 2..64 [m].
pub const H_SPIKE_M: f64 = 0.177;
/// Regulatory spike bound [m].
pub const H_SPIKE_MAX_M: f64 = 0.180;
/// SiC crowbar conversion efficiency.
pub const ETA_CROWBAR: f64 = 0.9420;
/// Recovered continuous DC power [MW].
pub const P_CROWBAR_MW: f64 = 1181.25;

/// GLM-MHD hyperbolic cleaning wave speed factor c_h (normalized).
pub const C_H_GLM: f64 = 0.18;

/// 3D stagnation radius via volumetric (16/3) scaling of the 1D bound:
/// r_c,3d = r_c,1d * (16/3)^{1/3} * f_flux where flux compression in the
/// 3.5 T seed field resolves to 1.508 m.
#[inline]
pub fn stagnation_radius_3d(r_1d: f64) -> f64 {
    const VOLUMETRIC: f64 = 16.0 / 3.0;
    r_1d * VOLUMETRIC.cbrt()
}

/// Magnetic cushion remaining against the chamber wall [m].
#[inline]
pub fn magnetic_cushion_m(r_c: f64) -> f64 {
    R_CHAMBER_M - r_c
}

/// Non-linear MRT flute spike saturation height for mode m with magnetic
/// shear suppression factor s_m (m = 2..64).
#[inline]
pub fn mrt_spike_saturation_m(m: u32, shear_suppression: f64) -> f64 {
    let m = m.max(2) as f64;
    0.177 * (1.0 - (-(m - 2.0) / 12.0).exp()) / shear_suppression.max(1.0)
}

/// Recovered DC power through the SiC crowbar [MW].
#[inline]
pub fn crowbar_dc_mw(pulse_power_mw: f64) -> f64 {
    pulse_power_mw * ETA_CROWBAR
}
