//! Empirical metrological calibration hierarchy (power6.txt §F):
//! GUM JCGM 100:2008 covariance budget and NIST-traceable ADC
//! transfer functions for the 128-byte MMIO telemetry block.

pub mod gum_evaluator;
pub mod transfer_functions;

pub use gum_evaluator::{GumCovarianceEvaluator, GumEvaluationResult, ThermalPowerBudget};
pub use transfer_functions::CalibrationTransferEngine;
