//! shbt-power-core — shared numerics, constants, and the subsystem trait
//! surface for the SHBT-Graser p-11B power-plant digital twin.
//!
//! Transferred from `sys1own/shbt-precision`: canonical WZW affine branch
//! `(26, 8, 312)` state tracking, the physical SI constants module, and the
//! 512-bit arbitrary-precision wrapper role.  The upstream `rug::Float`
//! (MPFR) binding is replaced here by `Float512`, a dependency-free
//! two's-complement Q256x256 fixed-point type — the GMP/MPFR toolchain is
//! not provisioned on the build fleet and fixed-width determinism better
//! matches the zero-allocation HIL contract anyway.

pub mod f512;
pub mod constants;
pub mod rom;

pub use constants::*;
pub use f512::{Complex512, Float512};

/// Canonical SHBT WZW affine branch (k_l, k_q, K) = (26, 8, 312).
pub const CANONICAL_BRANCH: (u32, u32, u32) = (26, 8, 312);

/// Unified lifecycle trait implemented by every physics subsystem crate.
///
/// `update` is invoked once per `step_macro_tick` (100 Hz macro epoch) and
/// must not allocate: all scratch state lives inside the implementor.
pub trait PhysicsSubsystem {
    /// Registry name used in audit reports and MMIO routing.
    fn name(&self) -> &'static str;
    /// Advance the subsystem by one 100 Hz macro tick (`dt = 10.0 ms`).
    fn update(&mut self, state: &mut PlantStateSnapshot);
}

/// Plant-wide state vector crossing the FFI boundary every macro tick.
/// 64-byte aligned, C-ABI ordered so `shbt-power-telemetry` can publish it
/// into the SHBT-POWER-MMIO register block at 0x70000000.
#[repr(C, align(64))]
#[derive(Clone, Debug)]
pub struct PlantStateSnapshot {
    /// 100 Hz macro-epoch counter.
    pub tick: u64,
    /// Plant lifecycle phase (0..=4, see shbt-power-grid).
    pub phase: u32,
    /// Graser driver electrical draw (MW).
    pub p_graser_elec_mw: f64,
    /// Graser optical beam output (MW).
    pub p_graser_beam_mw: f64,
    /// Core fusion power yield (MW).
    pub p_fusion_mw: f64,
    /// Direct-conversion electrical output (MW).
    pub p_direct_total_mw: f64,
    /// TEG reclaimed electrical output (MW).
    pub p_teg_mw: f64,
    /// Gross electrical generation (MW).
    pub p_gross_mw: f64,
    /// Recirculating demand (MW).
    pub p_recirc_mw: f64,
    /// Net grid export (MW).
    pub p_net_mw: f64,
    /// 450 MJ buffer state of charge (0..1).
    pub supercap_soc: f64,
    /// LANR starter array net output (kW).
    pub lanr_net_kw: f64,
    /// Plasma stopping radius (m).
    pub stopping_radius_m: f64,
    /// ADM 3+1 lapse determinant error |det(g) + 1|.
    pub adm_metric_err: f64,
    /// ADM 3+1 shift vector norm |beta^i|.
    pub adm_shift_norm: f64,
    /// Magnet quench headroom (K).
    pub quench_headroom_k: f64,
    /// Pellet trajectory jitter (um).
    pub target_jitter_um: f64,
    /// Holographic topological entropy margin.
    pub holo_entropy_gap: f64,
    /// Emergency interlock latch.
    pub interlock_latch: u32,
    /// 64-byte alignment padding.
    pub _pad: [u32; 5],
}

impl Default for PlantStateSnapshot {
    fn default() -> Self {
        Self {
            tick: 0,
            phase: 0,
            p_graser_elec_mw: 0.0,
            p_graser_beam_mw: 0.0,
            p_fusion_mw: 0.0,
            p_direct_total_mw: 0.0,
            p_teg_mw: 0.0,
            p_gross_mw: 0.0,
            p_recirc_mw: 0.0,
            p_net_mw: 0.0,
            supercap_soc: 0.0,
            lanr_net_kw: 0.0,
            stopping_radius_m: 0.0,
            adm_metric_err: 0.0,
            adm_shift_norm: 0.0,
            quench_headroom_k: 11.79,
            target_jitter_um: 0.0,
            holo_entropy_gap: 0.0,
            interlock_latch: 0,
            _pad: [0; 5],
        }
    }
}

const _: () = assert!(core::mem::size_of::<PlantStateSnapshot>() % 64 == 0);
const _: () = assert!(core::mem::align_of::<PlantStateSnapshot>() == 64);

/// Tolerance helper used across the audit gates.
#[inline]
pub fn approx(measured: f64, expected: f64, rel_tol: f64) -> bool {
    (measured - expected).abs() <= rel_tol * expected.abs().max(1.0)
}
