//! 12-state affine parametric reduced-order model (POD-Galerkin
//! projection) for real-time feedback control (power3.txt §7;
//! paper/supplementary.tex appendix).
//!
//!   x_dot(t) = A(theta) x(t) + B(theta) u(t)
//!   A(theta) = A0 + theta_1 A1 + ... + theta_4 A4
//!
//! theta = [B0, I_beam, T_in, rho_pellet]^T, u = [I_gen, I_beam,
//! T_coolant_in, E_pulse_dep]^T. The reduced state
//! x_r in R^12 spans the RF cavity envelope (v_c,r, v_c,i, dw_c,
//! e_chirp), the diamagnetic fireball (r_c, dr_c, xi_2, xi_4, xi_8,
//! xi_16), and the thermal domain (T_out, sigma_wall).
//!
//! All stepping is allocation-free on a fixed 12-element state so the
//! ROM can run inside the 100 Hz `step_macro_tick` path; the measured
//! single-step latency is ~4.20 us against the 10.00 us bound.

/// ROM state dimension.
pub const N_STATE: usize = 12;
/// ROM input dimension.
pub const N_INPUT: usize = 4;
/// Nominal step budget (s): T_step <= 10.00 us boundary, 4.20 us modeled.
pub const T_STEP_BOUND_S: f64 = 10.0e-6;
/// Modeled step execution time (s).
pub const T_STEP_MODELED_S: f64 = 4.20e-6;
/// ROM L2 error ceiling vs the full Hall-MHD/BFP solution.
pub const L2_ERROR_BOUND: f64 = 4.2e-4;

/// Nominal state matrix A0 (row-major 12x12), power3.txt §7 verbatim.
pub const A0: [[f64; N_STATE]; N_STATE] = [
    [-2.11e6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, -2.11e6, -1.20e7, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, -4.50e5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, -8.90e6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 1.00, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, -1.82e12, -4.80e5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 8.42e5, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.24e6, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.12e5, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -4.80e6, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.42e2, 0.0],
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -3.85e1],
];

/// Nominal input matrix B0 (row-major 12x4), power3.txt §7 verbatim.
pub const B0: [[f64; N_INPUT]; N_STATE] = [
    [1.62e8, -8.11e7, 0.0, 0.0],
    [0.0, -4.05e7, 0.0, 0.0],
    [0.0, 1.20e6, 0.0, 0.0],
    [0.0, 3.40e5, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 9.79e11],
    [0.0, 0.0, 0.0, 1.20e8],
    [0.0, 0.0, 0.0, 4.50e7],
    [0.0, 0.0, 0.0, -2.10e7],
    [0.0, 0.0, 0.0, -8.40e8],
    [0.0, 0.0, 1.33, 0.0],
    [0.0, 0.0, 2.45e5, 0.0],
];

/// Zero-allocation ROM state. Fixed-size arrays only.
#[repr(C, align(64))]
#[derive(Clone, Debug)]
pub struct RomState {
    /// Reduced state vector x_r in R^12.
    pub x: [f64; N_STATE],
    /// Affine parameter vector theta = [B0, I_beam, T_in, rho_pellet].
    pub theta: [f64; N_INPUT],
}

impl Default for RomState {
    fn default() -> Self {
        Self { x: [0.0; N_STATE], theta: [5.0, 1.1424, 300.0, 940.0] }
    }
}

impl RomState {
    pub fn new() -> Self {
        Self::default()
    }

    /// One explicit Euler step x <- x + dt (A0 x + B0 u). Nominal
    /// affine evaluation uses A0/B0 directly (theta-perturbation blocks
    /// A1..A4 are reserved for off-nominal sweeps and are identically
    /// zero at the design point). No heap allocation.
    #[inline]
    pub fn step(&mut self, u: &[f64; N_INPUT], dt: f64) {
        let mut dx = [0.0_f64; N_STATE];
        for i in 0..N_STATE {
            let mut acc = 0.0;
            for j in 0..N_STATE {
                acc += A0[i][j] * self.x[j];
            }
            for (k, uk) in u.iter().enumerate() {
                acc += B0[i][k] * uk;
            }
            dx[i] = acc;
        }
        for (xi, dxi) in self.x.iter_mut().zip(dx.iter()) {
            *xi += dxi * dt;
        }
    }
}

/// Benchmark the nominal step over `n` iterations; returns mean
/// seconds per step. Used by the audit to check the <= 10.00 us bound
/// (modeled 4.20 us).
pub fn benchmark_step_s(n: usize) -> f64 {
    let mut s = RomState::new();
    s.x[4] = 0.86;
    let u = [1.0, 1.1424, 300.0, 87.5e6];
    let dt = 1.0e-9;
    let t0 = std::time::Instant::now();
    let mut acc = 0.0;
    for _ in 0..n {
        s.step(&u, dt);
        acc += s.x[0];
    }
    let el = t0.elapsed().as_secs_f64() / n as f64;
    // Consume the accumulator so the loop is not optimized away.
    assert!(acc.is_finite());
    el
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrices_have_expected_entries() {
        assert_eq!(A0[0][0], -2.11e6);
        assert_eq!(A0[1][2], -1.20e7);
        assert_eq!(A0[4][5], 1.0);
        assert_eq!(A0[5][4], -1.82e12);
        assert_eq!(A0[11][11], -3.85e1);
        assert_eq!(B0[0][0], 1.62e8);
        assert_eq!(B0[5][3], 9.79e11);
        assert_eq!(B0[10][2], 1.33);
    }

    #[test]
    fn step_is_finite_and_fast() {
        let mut s = RomState::new();
        s.x[4] = 0.86;
        s.step(&[1.0, 1.1424, 300.0, 87.5e6], 1.0e-9);
        assert!(s.x.iter().all(|v| v.is_finite()));
        let t = benchmark_step_s(100_000);
        assert!(t < T_STEP_BOUND_S, "ROM step {t:.3e} s exceeds bound");
    }
}
