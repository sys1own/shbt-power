//! shbt-power-py — PyO3 C-extension exporting `PyShbtDigitalTwin`, a
//! zero-copy handle over the multi-physics plant state for the Python
//! orchestration layer (paper/main.tex §6).
// pyo3 0.20's #[pymethods] emits a non-local impl; harmless, allowed here.
#![allow(non_local_definitions)]

use pyo3::prelude::*;

use shbt_power_chamber::ChamberSubsystem;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};
use shbt_power_dec::DecSubsystem;
use shbt_power_grid::GridSubsystem;
use shbt_power_holography::HolographySubsystem;
use shbt_power_linac::LinacSubsystem;
use shbt_power_target::TargetSubsystem;
use shbt_power_telemetry::TelemetrySubsystem;

/// Aggregated digital twin: every physics subsystem stepped in lockstep
/// over the 100 Hz macro-tick. `step_macro_tick` is allocation-free inside
/// the loop (all subsystem state is stack/struct owned).
#[pyclass]
pub struct PyShbtDigitalTwin {
    state: PlantStateSnapshot,
    linac: LinacSubsystem,
    chamber: ChamberSubsystem,
    dec: DecSubsystem,
    grid: GridSubsystem,
    target: TargetSubsystem,
    holo: HolographySubsystem,
    telem: TelemetrySubsystem,
}

impl Default for PyShbtDigitalTwin {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PyShbtDigitalTwin {
    #[new]
    pub fn new() -> Self {
        Self {
            state: PlantStateSnapshot::default(),
            linac: LinacSubsystem::default(),
            chamber: ChamberSubsystem::default(),
            dec: DecSubsystem::default(),
            grid: GridSubsystem::default(),
            target: TargetSubsystem::default(),
            holo: HolographySubsystem::default(),
            telem: TelemetrySubsystem,
        }
    }

    /// Advance the plant by one 100 Hz macro tick.
    pub fn step_macro_tick(&mut self) {
        self.state.tick += 1;
        self.linac.update(&mut self.state);
        self.chamber.update(&mut self.state);
        self.dec.update(&mut self.state);
        self.grid.update(&mut self.state);
        self.target.update(&mut self.state);
        self.holo.update(&mut self.state);
        self.telem.update(&mut self.state);
    }

    /// Run `n` macro ticks; returns the net grid export (MW) afterwards.
    pub fn run(&mut self, n: u64) -> f64 {
        for _ in 0..n {
            self.step_macro_tick();
        }
        self.state.p_net_mw
    }

    #[getter]
    pub fn tick(&self) -> u64 {
        self.state.tick
    }
    #[getter]
    pub fn phase(&self) -> u32 {
        self.state.phase
    }
    #[getter]
    pub fn p_net_mw(&self) -> f64 {
        self.state.p_net_mw
    }
    #[getter]
    pub fn p_gross_mw(&self) -> f64 {
        self.state.p_gross_mw
    }
    #[getter]
    pub fn p_fusion_mw(&self) -> f64 {
        self.state.p_fusion_mw
    }
    #[getter]
    pub fn p_teg_mw(&self) -> f64 {
        self.state.p_teg_mw
    }
    #[getter]
    pub fn supercap_soc(&self) -> f64 {
        self.state.supercap_soc
    }
    #[getter]
    pub fn battery_core_temp_k(&self) -> f64 {
        self.state.battery_core_temp_k
    }
    #[getter]
    pub fn battery_soc(&self) -> f64 {
        self.state.battery_soc
    }
    #[getter]
    pub fn battery_bus_voltage_kv(&self) -> f64 {
        self.state.battery_bus_voltage_kv
    }
    #[getter]
    pub fn battery_decay_heat_kw(&self) -> f64 {
        self.state.battery_decay_heat_kw
    }
    #[getter]
    pub fn adm_metric_err(&self) -> f64 {
        self.state.adm_metric_err
    }
    #[getter]
    pub fn stopping_radius_m(&self) -> f64 {
        self.state.stopping_radius_m
    }
}

#[pymodule]
fn shbt_power_py(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyShbtDigitalTwin>()?;
    Ok(())
}
