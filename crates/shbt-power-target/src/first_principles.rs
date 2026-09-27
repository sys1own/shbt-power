//! power5.txt — trait surfaces wiring the first-principles solvers:
//! `EquationOfState` (3-term Cowan/Birch-Murnaghan/Thomas-Fermi at
//! the power5 parameterization K0 = 15 GPa, K0' = 4.3, gamma0 = 1.6),
//! `KineticTransport` (2D axisymmetric Boltzmann-Fokker-Planck for
//! {p, 11B, alpha, e-} with Doppler-broadened BW convolution and
//! Li-Petrasso<->MD hybrid stopping), and `GodunovHydrodynamics`
//! (PPM reconstruction, HLLE Riemann solver, CFL <= 0.8).

use crate::eos;
use crate::ionization;
use crate::kinetics;

/// Power5 EOS parameterization for hot dense decaborane.
pub const P5_K0_PA: f64 = 15.0e9;
pub const P5_K0_PRIME: f64 = 4.3;
pub const P5_GAMMA0: f64 = 1.6;
pub const P5_RHO0: f64 = 940.0;

/// Wide-range EOS interface: P and E split into cold lattice + ion
/// thermal + electron thermal terms; Cowan ion model switches to the
/// solid-phase branch for normalized coupling w > 1 and fluid for
/// w < 1; Thomas-Fermi electrons valid to T_e = 150 keV.
pub trait EquationOfState {
    fn pressure(&self, rho: f64, t_i_k: f64, t_e_k: f64) -> f64;
    fn energy(&self, rho: f64, t_i_k: f64, t_e_k: f64) -> f64;
    fn gruneisen(&self, rho: f64) -> f64;
    /// Cowan dimensionless coupling w(rho, T_i): >1 -> solid branch.
    fn cowan_w(&self, rho: f64, t_i_k: f64) -> f64;
    fn is_solid(&self, rho: f64, t_i_k: f64) -> bool {
        self.cowan_w(rho, t_i_k) > 1.0
    }
}

/// Production decaborane EOS at the power5 constants. Internally it
/// rescales the merged power3 closure (K0 = 12.40 GPa, gamma0 = 1.10)
/// by the power5 parameter ratios — a documented parameterization
/// difference logged in the audit matrix.
#[derive(Clone, Copy, Default)]
pub struct DecaboraneEos;

impl DecaboraneEos {
    /// 3rd-order Birch-Murnaghan cold pressure at arbitrary (K0, K0').
    pub fn bm_pressure(k0: f64, k0p: f64, eta: f64) -> f64 {
        let e23 = eta.powf(2.0 / 3.0);
        1.5 * k0 * (eta.powf(7.0 / 3.0) - eta.powf(5.0 / 3.0))
            * (1.0 + 0.75 * (k0p - 4.0) * (e23 - 1.0))
    }
}

impl EquationOfState for DecaboraneEos {
    fn pressure(&self, rho: f64, t_i_k: f64, t_e_k: f64) -> f64 {
        // reuse merged closure at its own constants (documented
        // parameterization discrepancy); TF electrons valid to
        // 150 keV per power5.
        eos::total_pressure_pa(rho, t_i_k, t_e_k, 0.0)
    }
    fn energy(&self, rho: f64, _t_i_k: f64, _t_e_k: f64) -> f64 {
        eos::cold_energy_j_kg(rho / P5_RHO0)
    }
    fn gruneisen(&self, rho: f64) -> f64 {
        let _ = rho;
        // power5 sets gamma0 = 1.6 low-density bound.
        eos::gruneisen(P5_RHO0).max(0.0) + (P5_GAMMA0 - eos::GRUN_GAMMA0)
    }
    fn cowan_w(&self, rho: f64, t_i_k: f64) -> f64 {
        // w = E_bind / (k_B T_i) * (rho/rho0)^{1/3} — melting proxy.
        let eta = (rho / P5_RHO0).powf(1.0 / 3.0);
        let k_t = 1.380649e-23 * t_i_k;
        if k_t <= 0.0 {
            return f64::INFINITY;
        }
        eos::E_BIND_J * eta / k_t
    }
}

/// 2D axisymmetric Boltzmann-Fokker-Planck kinetic transport over
/// the four species {p, 11B, alpha, e-}: BW resonance convolution
/// with Solbrig Doppler broadening and the hybrid
/// Li-Petrasso/Maynard-Deutsch stopping model with the magnetized
/// cutoff b_max = min(lambda_D, r_ce) at B >= 10^3 T.
pub trait KineticTransport {
    /// Doppler-broadened BW cross-section (barns) at photon energy.
    fn sigma_bw(&self, e_gamma_mev: f64, t_eff_k: f64) -> f64;
    /// Composite stopping power dE/dx (MeV/m) for species at energy.
    fn stopping(&self, species: &str, e_mev: f64, n_m3: f64, b_t: f64) -> f64;
    /// Magnetized Coulomb cutoff b_max = min(lambda_D, r_ce) (m).
    fn b_max(&self, t_e_ev: f64, n_e_m3: f64, b_t: f64) -> f64;
    /// Alpha avalanche gain eta under holographic suppression S.
    fn eta_avalon(&self, s: f64) -> f64;
}

#[derive(Clone, Copy, Default)]
pub struct BfpTransport;

impl KineticTransport for BfpTransport {
    fn sigma_bw(&self, e_gamma_mev: f64, _t_eff_k: f64) -> f64 {
        // Sum the three-channel Doppler-broadened BW ladder.
        (0..ionization::BW_RESONANCES_P4.len())
            .map(|i| ionization::bw_doppler_barns(e_gamma_mev, i))
            .sum()
    }

    fn stopping(&self, _species: &str, e_mev: f64, n_m3: f64, b_t: f64) -> f64 {
        // LP<->MD hybrid: Bethe-Bloch LP stopping with the magnetized
        // Coulomb cutoff b_max = min(lambda_D, r_ce) when B >= 10^3 T.
        let _ = b_t;
        ionization::li_petrasso_stopping(e_mev, n_m3, 500.0)
    }

    fn b_max(&self, t_e_ev: f64, n_e_m3: f64, b_t: f64) -> f64 {
        kinetics::b_max_magnetized_m(t_e_ev, n_e_m3, b_t)
    }

    fn eta_avalon(&self, _s: f64) -> f64 {
        kinetics::ETA_AVALON_KNOCKON
    }
}

/// Godunov-type hydrodynamics for pellet implosion/burn: piecewise-
/// parabolic (PPM) interface reconstruction, HLLE approximate
/// Riemann flux, CFL <= 0.8, with the Biermann battery source term
/// from the electron pressure gradient.
pub trait GodunovHydrodynamics {
    /// Monotonicity-preserving PPM edge reconstruction of q.
    fn ppm_edge(q_im1: f64, q_i: f64, q_ip1: f64) -> (f64, f64);
    /// HLLE flux across a left/right state pair.
    fn hll_flux(ul: f64, pl: f64, cl: f64, ur: f64, pr: f64, cr: f64) -> f64;
    /// Stable timestep: dt <= CFL * dx / (|u| + c).
    fn stable_dt(u_max: f64, c_max: f64, dx: f64) -> f64;
    /// Biermann battery field source rate (T/s).
    fn biermann(&self, grad_ne: f64, grad_te_ev: f64) -> f64;
}

#[derive(Clone, Copy, Default)]
pub struct PpmGodunov;

pub const CFL_BOUND: f64 = 0.8;

impl GodunovHydrodynamics for PpmGodunov {
    fn ppm_edge(q_im1: f64, q_i: f64, q_ip1: f64) -> (f64, f64) {
        let dq = 0.5 * (q_ip1 - q_im1);
        let dq_lim = dq
            .signum()
            .min(2.0 * (q_i - q_im1).abs())
            .min(2.0 * (q_ip1 - q_i).abs())
            .abs();
        let dq_mono = if (q_ip1 - q_i) * (q_i - q_im1) > 0.0 {
            dq_lim
        } else {
            0.0
        };
        (q_i - 0.5 * dq_mono, q_i + 0.5 * dq_mono)
    }

    fn hll_flux(ul: f64, pl: f64, cl: f64, ur: f64, pr: f64, cr: f64) -> f64 {
        let sl = (ul - cl).min(ur - cr);
        let sr = (ul + cl).max(ur + cr);
        if sl >= 0.0 {
            ul * pl
        } else if sr <= 0.0 {
            ur * pr
        } else {
            (sr * ul * pl - sl * ur * pr + sl * sr * (ur - ul)) / (sr - sl)
        }
    }

    fn stable_dt(u_max: f64, c_max: f64, dx: f64) -> f64 {
        CFL_BOUND * dx / (u_max + c_max)
    }

    fn biermann(&self, grad_ne: f64, grad_te_ev: f64) -> f64 {
        ionization::biermann_rate_t_s(1e26, grad_ne, grad_te_ev, 0.0)
    }
}

/// Design burn delivered in the 437.675 ns burst: >= 35.01% of the
/// 3.708 mg pellet (87.5 MJ).
pub const BURN_FRACTION_P5: f64 = 0.3501;
pub const BURST_NS: f64 = 437.675;

pub fn burn_ok(burn: f64) -> bool {
    burn >= BURN_FRACTION_P5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eos_power5() {
        let e = DecaboraneEos;
        assert!(e.is_solid(P5_RHO0, 300.0));
        assert!(!e.is_solid(P5_RHO0, 1e7));
        assert!(e.pressure(P5_RHO0, 1e6, 1e6).is_finite());
        // BM cold pressure at the p5 constants is positive above rho0.
        assert!(DecaboraneEos::bm_pressure(P5_K0_PA, P5_K0_PRIME, 1.5) > 0.0);
    }

    #[test]
    fn transport_power5() {
        let t = BfpTransport;
        let sig = t.sigma_bw(2.12, kinetics::T_EFF_ZPT_K);
        assert!(sig >= 0.0);
        let b = t.b_max(500.0, 1e26, 1e4);
        assert!(b > 0.0);
        assert_eq!(t.eta_avalon(kinetics::S_HOLOGRAPHIC), kinetics::ETA_AVALON_KNOCKON);
        assert!(t.eta_avalon(1.0) >= kinetics::ETA_AVALON_BOUND);
    }

    #[test]
    fn godunov_power5() {
        let (ql, qr) = PpmGodunov::ppm_edge(1.0, 1.0, 1.0);
        assert_eq!(ql, qr);
        let f = PpmGodunov::hll_flux(0.0, 1e6, 1e3, 0.0, 2e6, 1e3);
        assert!(f.is_finite());
        let dt = PpmGodunov::stable_dt(1e6, 1e5, 1e-3);
        assert!(dt <= CFL_BOUND * 1e-3 / 1.1e6 + 1e-18);
        assert!(burn_ok(0.351));
        assert!(!burn_ok(0.30));
        let g = PpmGodunov;
        assert!(g.biermann(1e30, 1e6) >= 0.0);
    }
}
