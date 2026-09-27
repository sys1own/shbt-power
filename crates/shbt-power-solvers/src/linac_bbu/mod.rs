//! Linac multi-bunch dipole wakefield / BBU tracking (Q_ext <= 200, SiC HOM).

pub mod bbu_tracker;
pub mod reduced_order_wake;

pub use bbu_tracker::{BbuTracker, LinacElement, PhaseSpaceState};
pub use reduced_order_wake::{QualityGateStatus, ReducedWakeMatrix};
