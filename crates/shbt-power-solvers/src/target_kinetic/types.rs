//! High-precision type definitions and physical state vectors.

pub const SPEED_OF_LIGHT: f64 = 299_792_458.0;
pub const ELEMENTARY_CHARGE: f64 = 1.602_176_634e-19;
pub const EPSILON_0: f64 = 8.8541878128e-12;
pub const BOLTZMANN_K: f64 = 1.380_649e-23;
pub const ATOMIC_MASS_UNIT: f64 = 1.660_539_066_60e-27;
pub const MASS_ELECTRON: f64 = 9.1093837015e-31;
pub const MASS_PROTON: f64 = 1.672_621_923_69e-27;
pub const MASS_ALPHA: f64 = 6.644_657_230e-27;
pub const MASS_B11: f64 = 1.828_279e-26;
pub const Q_FUSION_PB11_JOULES: f64 = 8.680e6 * 1.602_176_634e-19;

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConservedState {
    pub rho: f64,
    pub momentum: f64,
    pub energy_tot: f64,
    pub rho_ion_internal: f64,
    pub rho_elec_internal: f64,
    pub b_field_theta: f64,
    pub n_boron_total: f64,
    pub n_proton_total: f64,
}

impl ConservedState {
    pub const ZERO: Self = Self {
        rho: 0.0,
        momentum: 0.0,
        energy_tot: 0.0,
        rho_ion_internal: 0.0,
        rho_elec_internal: 0.0,
        b_field_theta: 0.0,
        n_boron_total: 0.0,
        n_proton_total: 0.0,
    };
}

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimitiveState {
    pub rho: f64,
    pub velocity: f64,
    pub pressure_total: f64,
    pub pressure_elec: f64,
    pub pressure_ion: f64,
    pub temp_elec_ev: f64,
    pub temp_ion_ev: f64,
    pub b_field_theta: f64,
    pub ionization_z_boron: f64,
    pub n_e: f64,
    pub n_p: f64,
    pub n_b: f64,
}

#[repr(C, align(32))]
#[derive(Debug, Clone, Copy)]
pub struct AlphaPacket {
    pub energy_ev: f64,
    pub weight: f64,
    pub radius: f64,
    pub direction_mu: f64,
}