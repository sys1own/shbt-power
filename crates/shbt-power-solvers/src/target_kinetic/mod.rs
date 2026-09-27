//! Spherical Godunov-PPM hydrodynamics + Fokker-Planck knock-on transport
//! for the decaborane graser target (power6.txt §target_kinetic).

pub mod godunov;
pub mod nuclear;
pub mod surrogate;
pub mod transport;
pub mod types;

pub use surrogate::{ReducedOrderTargetResponse, TargetSurrogateModel};
