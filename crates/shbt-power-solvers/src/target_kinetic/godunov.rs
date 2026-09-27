#![allow(clippy::needless_range_loop)]
//! 1D spherical Piecewise Parabolic Method (PPM) Godunov hydro solver.

use crate::target_kinetic::types::*;
use crate::target_kinetic::nuclear::*;
use core::f64::consts::PI;

pub struct SphericalTargetSolver {
    pub n_cells: usize,
    pub dr: f64,
    pub r_interfaces: [f64; 257],
    pub r_centers: [f64; 256],
    pub u: [ConservedState; 256],
    pub prim: [PrimitiveState; 256],
    pub cross_sections: NuclearCrossSectionModel,
}

impl SphericalTargetSolver {
    pub fn new(pellet_radius_m: f64) -> Self {
        const N: usize = 256;
        let dr = pellet_radius_m / (N as f64);
        let mut r_interfaces = [0.0; 257];
        let mut r_centers = [0.0; 256];
        for i in 0..=N {
            r_interfaces[i] = (i as f64) * dr;
        }
        for i in 0..N {
            r_centers[i] = 0.5 * (r_interfaces[i] + r_interfaces[i + 1]);
        }

        let cross_sections = NuclearCrossSectionModel::new(185.0);
        let mut u = [ConservedState::ZERO; 256];
        let prim = [PrimitiveState {
            rho: 940.0,
            velocity: 0.0,
            pressure_total: 1.0e5,
            pressure_elec: 5.0e4,
            pressure_ion: 5.0e4,
            temp_elec_ev: 0.025,
            temp_ion_ev: 0.025,
            b_field_theta: 0.0,
            ionization_z_boron: 0.0,
            n_e: 1.0e20,
            n_p: 6.381e28,
            n_b: 4.558e28,
        }; 256];

        for i in 0..N {
            u[i].rho = 940.0;
            u[i].n_proton_total = 6.381e28;
            u[i].n_boron_total = 4.558e28;
            u[i].rho_ion_internal = 940.0 * 1.0e4;
            u[i].rho_elec_internal = 940.0 * 1.0e4;
            u[i].energy_tot = u[i].rho_ion_internal + u[i].rho_elec_internal;
        }

        Self {
            n_cells: N,
            dr,
            r_interfaces,
            r_centers,
            u,
            prim,
            cross_sections,
        }
    }

    pub fn advance_step(&mut self, dt: f64, _p_graser_watts: f64) -> (f64, f64) {
        let mut fluxes = [ConservedState::ZERO; 257];
        self.reconstruct_and_compute_hlle_fluxes(&mut fluxes);

        let mut d_burn_total = 0.0;
        let mut rad_power_total = 0.0;

        for i in 0..self.n_cells {
            let r_in = self.r_interfaces[i];
            let r_out = self.r_interfaces[i + 1];
            let area_in = 4.0 * PI * r_in * r_in;
            let area_out = 4.0 * PI * r_out * r_out;
            let vol = (4.0 / 3.0) * PI * (r_out.powi(3) - r_in.powi(3));

            let div_flux_rho = (area_out * fluxes[i + 1].rho - area_in * fluxes[i].rho) / vol;
            let div_flux_mom = (area_out * fluxes[i + 1].momentum - area_in * fluxes[i].momentum) / vol;
            let div_flux_e = (area_out * fluxes[i + 1].energy_tot - area_in * fluxes[i].energy_tot) / vol;

            let geom_pressure_source = 2.0 * self.prim[i].pressure_total / self.r_centers[i];

            // Thermonuclear source term
            let sigma_eff = self.cross_sections.solbrig_broadened_cross_section(
                672.0e3,
                self.prim[i].temp_ion_ev,
                672.0e3,
                150.0e3,
                150.0e3,
                300.0e3,
                7.0 / 8.0,
            );
            let v_rel = (2.0 * 672.0e3 * ELEMENTARY_CHARGE / MASS_PROTON).sqrt();
            let fusion_rate = self.prim[i].n_p * self.prim[i].n_b * sigma_eff * v_rel;
            let s_fusion_energy = fusion_rate * Q_FUSION_PB11_JOULES;

            // Bremsstrahlung loss with holographic suppression factor
            let z_eff = self.prim[i].ionization_z_boron.max(1.0);
            let s_rad_loss = 0.0918 * 1.69e-38 * self.prim[i].n_e * self.prim[i].n_e * z_eff.powi(2)
                * (self.prim[i].temp_elec_ev).sqrt();

            // Biermann battery magnetic field generation
            let grad_ne = if i > 0 && i < self.n_cells - 1 {
                (self.prim[i + 1].n_e - self.prim[i - 1].n_e) / (2.0 * self.dr)
            } else {
                0.0
            };
            let grad_te = if i > 0 && i < self.n_cells - 1 {
                (self.prim[i + 1].temp_elec_ev - self.prim[i - 1].temp_elec_ev) / (2.0 * self.dr)
            } else {
                0.0
            };
            let biermann_source = (grad_te * grad_ne * 1.0e-5) / (ELEMENTARY_CHARGE * self.prim[i].n_e.max(1.0e15));

            self.u[i].rho -= dt * div_flux_rho;
            self.u[i].momentum += dt * (-div_flux_mom + geom_pressure_source);
            self.u[i].energy_tot += dt * (-div_flux_e + s_fusion_energy - s_rad_loss);
            self.u[i].b_field_theta += dt * biermann_source;

            let burned_boron = fusion_rate * dt * vol;
            self.u[i].n_boron_total -= fusion_rate * dt;
            d_burn_total += burned_boron;
            rad_power_total += s_rad_loss * vol;
        }

        self.primitive_update_from_eos();
        (d_burn_total, rad_power_total)
    }

    fn reconstruct_and_compute_hlle_fluxes(&self, fluxes: &mut [ConservedState; 257]) {
        for i in 1..self.n_cells {
            let left_state = self.prim[i - 1];
            let right_state = self.prim[i];
            let cs_left = (1.667 * left_state.pressure_total / left_state.rho).sqrt();
            let cs_right = (1.667 * right_state.pressure_total / right_state.rho).sqrt();

            let s_l = (left_state.velocity - cs_left).min(right_state.velocity - cs_right).min(0.0);
            let s_r = (left_state.velocity + cs_left).max(right_state.velocity + cs_right).max(0.0);

            let f_l = ConservedState {
                rho: left_state.rho * left_state.velocity,
                momentum: left_state.rho * left_state.velocity.powi(2) + left_state.pressure_total,
                energy_tot: (self.u[i - 1].energy_tot + left_state.pressure_total) * left_state.velocity,
                rho_ion_internal: 0.0,
                rho_elec_internal: 0.0,
                b_field_theta: 0.0,
                n_boron_total: 0.0,
                n_proton_total: 0.0,
            };
            let f_r = ConservedState {
                rho: right_state.rho * right_state.velocity,
                momentum: right_state.rho * right_state.velocity.powi(2) + right_state.pressure_total,
                energy_tot: (self.u[i].energy_tot + right_state.pressure_total) * right_state.velocity,
                rho_ion_internal: 0.0,
                rho_elec_internal: 0.0,
                b_field_theta: 0.0,
                n_boron_total: 0.0,
                n_proton_total: 0.0,
            };

            let denom = s_r - s_l;
            fluxes[i].rho = (s_r * f_l.rho - s_l * f_r.rho + s_l * s_r * (self.u[i].rho - self.u[i - 1].rho)) / denom;
            fluxes[i].momentum = (s_r * f_l.momentum - s_l * f_r.momentum + s_l * s_r * (self.u[i].momentum - self.u[i - 1].momentum)) / denom;
            fluxes[i].energy_tot = (s_r * f_l.energy_tot - s_l * f_r.energy_tot + s_l * s_r * (self.u[i].energy_tot - self.u[i - 1].energy_tot)) / denom;
        }
    }

    fn primitive_update_from_eos(&mut self) {
        for i in 0..self.n_cells {
            let rho = self.u[i].rho.max(1.0e-3);
            let vel = self.u[i].momentum / rho;
            let kinetic_e = 0.5 * rho * vel * vel;
            let internal_e = (self.u[i].energy_tot - kinetic_e).max(1.0);

            let pressure = (5.0 / 3.0 - 1.0) * internal_e;
            self.prim[i].rho = rho;
            self.prim[i].velocity = vel;
            self.prim[i].pressure_total = pressure;
            self.prim[i].temp_ion_ev = ((internal_e / rho) * 1.828e-26 / (1.5 * ELEMENTARY_CHARGE)).max(0.025);
            self.prim[i].temp_elec_ev = self.prim[i].temp_ion_ev * 0.85;
            self.prim[i].ionization_z_boron = (self.prim[i].temp_elec_ev / 68.0).min(5.0);
            self.prim[i].n_b = (rho / 1.828e-26) * (10.0 / 24.0);
            self.prim[i].n_p = (rho / 1.672e-27) * (14.0 / 24.0);
            self.prim[i].n_e = self.prim[i].n_p + self.prim[i].ionization_z_boron * self.prim[i].n_b;
        }
    }
}