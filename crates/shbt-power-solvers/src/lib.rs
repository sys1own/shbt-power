//! shbt-power-solvers — Tier-1 first-principles solver workbench
//! (power6.txt; paper/power.pdf §§2–5, paper/supplementary.pdf App. A–G).
//!
//! Offline high-fidelity solvers whose reduced-order outputs feed the
//! zero-allocation 100 Hz real-time crates (shbt-power-*).
//! `target_kinetic` and `linac_bbu` are `no_std`-compatible module sets;
//! `dec_pic` and `mhd_chamber` provide the reduced-order boundary
//! surrogates derived from the 2D/3D PIC and GLM-MHD runs.

pub mod dec_pic;
pub mod linac_bbu;
pub mod mhd_chamber;
pub mod target_kinetic;
pub mod thermal_fea;
