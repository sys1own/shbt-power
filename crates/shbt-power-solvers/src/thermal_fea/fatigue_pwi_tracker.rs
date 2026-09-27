// crates/shbt-power-solvers/src/thermal_fea/fatigue_pwi_tracker.rs

use std::f64::consts::PI;

/// Material mechanical and thermal parameters.
#[derive(Debug, Clone, Copy)]
pub struct MaterialProperties {
    pub k_th: f64,          // W/(m K)
    pub rho: f64,           // kg/m^3
    pub cp: f64,            // J/(kg K)
    pub young_modulus: f64, // Pa
    pub poisson_ratio: f64, // dimensionless
    pub alpha_th: f64,      // K^-1
    pub yield_stress: f64,  // Pa
    pub sigma_f_prime: f64, // Pa (Fatigue strength coefficient)
    pub eps_f_prime: f64,   // Fatigue ductility coefficient
    pub b_exponent: f64,    // Fatigue strength exponent
    pub c_exponent: f64,    // Fatigue ductility exponent
    pub c_paris: f64,       // Paris law coefficient
    pub m_paris: f64,       // Paris law exponent
    pub k_ic: f64,          // Fracture toughness (Pa m^0.5)
    pub u_s: f64,           // Surface binding energy (eV)
    pub atomic_z: f64,      // Atomic number Z2
    pub atomic_m: f64,      // Atomic mass M2 (amu)
}

/// Projectile ion properties for sputtering calculations.
#[derive(Debug, Clone, Copy)]
pub struct IonBeam {
    pub z1: f64,          // Ion atomic number
    pub m1: f64,          // Ion mass (amu)
    pub energy_ev: f64,   // Kinetic energy (eV)
    pub flux: f64,        // Ion flux (ions/m^2/s)
    pub theta_rad: f64,   // Incident angle relative to normal
}

pub struct ThermoFatiguePwiTracker {
    pub material: MaterialProperties,
    pub initial_flaw_m: f64,
}

impl ThermoFatiguePwiTracker {
    pub fn new(material: MaterialProperties, initial_flaw_m: f64) -> Self {
        Self {
            material,
            initial_flaw_m,
        }
    }

    /// Computes transient surface temperature excursion (K) for a heat pulse.
    pub fn surface_temp_rise(&self, fluence_j_m2: f64, t_flash_s: f64) -> f64 {
        let eff = self.material.k_th * self.material.rho * self.material.cp;
        (2.0 * fluence_j_m2) / (PI * eff * t_flash_s).sqrt()
    }

    /// Evaluates quasi-steady thermo-elastic surface stress range (Pa).
    pub fn thermoelastic_stress_range(&self, delta_t: f64) -> f64 {
        (self.material.young_modulus * self.material.alpha_th * delta_t) 
            / (1.0 - self.material.poisson_ratio)
    }

    /// Solves Coffin-Manson relation for number of cycles to fatigue failure N_f.
    pub fn calculate_fatigue_life(&self, delta_sigma: f64) -> f64 {
        let delta_eps_total = delta_sigma / self.material.young_modulus;
        let delta_eps_e = if delta_sigma > self.material.yield_stress {
            self.material.yield_stress / self.material.young_modulus
        } else {
            delta_eps_total
        };
        let delta_eps_p = (delta_eps_total - delta_eps_e).max(0.0);

        let n_f_elastic = 0.5 * ((delta_sigma / 2.0) / self.material.sigma_f_prime)
            .powf(1.0 / self.material.b_exponent);

        if delta_eps_p > 0.0 {
            let n_f_plastic = 0.5 * ((delta_eps_p / 2.0) / self.material.eps_f_prime)
                .powf(1.0 / self.material.c_exponent);
            n_f_elastic.min(n_f_plastic)
        } else {
            n_f_elastic
        }
    }

    /// Integrates Paris law crack propagation da/dN = C (Delta K)^m.
    pub fn calculate_paris_crack_life(&self, delta_sigma: f64, max_thickness_m: f64) -> f64 {
        let y_geo = 1.12;
        let k_ic = self.material.k_ic;
        let a_critical = (0.5 * (k_ic / (y_geo * delta_sigma * PI.sqrt())).powi(2))
            .min(max_thickness_m);
        let a_i = self.initial_flaw_m;

        if a_critical <= a_i {
            return 0.0;
        }

        let m = self.material.m_paris;
        let c = self.material.c_paris;
        let stress_factor = c * (y_geo * delta_sigma * PI.sqrt()).powf(m);

        if (m - 2.0).abs() < 1e-6 {
            (a_critical / a_i).ln() / stress_factor
        } else {
            let exp = 1.0 - m / 2.0;
            (a_critical.powf(exp) - a_i.powf(exp)) / (exp * stress_factor)
        }
    }

    /// Yamamura-Eckstein physical sputtering yield formulation.
    pub fn yamamura_sputtering_yield(&self, ion: &IonBeam) -> f64 {
        if ion.energy_ev <= 0.0 {
            return 0.0;
        }

        let z1 = ion.z1;
        let m1 = ion.m1;
        let z2 = self.material.atomic_z;
        let m2 = self.material.atomic_m;
        let u_s = self.material.u_s;

        let a_tf = 0.8853 * 0.529177 / (z1.powf(2.0 / 3.0) + z2.powf(2.0 / 3.0)).sqrt();
        let eps = (m2 / (m1 + m2)) * (a_tf / (z1 * z2 * 14.4)) * ion.energy_ev;

        let sn_eps = 3.441 * eps.sqrt() * (eps + std::f64::consts::E).ln() 
            / (1.0 + 6.355 * eps.sqrt() + eps * (6.882 * eps.sqrt() - 1.708));

        let sn_e = 8.478 * z1 * z2 * m1 
            / ((m1 + m2) * (z1.powf(2.0 / 3.0) + z2.powf(2.0 / 3.0)).sqrt()) * sn_eps;

        let gamma = 4.0 * m1 * m2 / (m1 + m2).powi(2);
        let e_th = if m1 / m2 < 0.2 {
            u_s / (gamma * (1.0 - gamma))
        } else {
            8.0 * u_s * (m1 / m2).powf(-1.0 / 3.0)
        };

        if ion.energy_ev <= e_th {
            return 0.0;
        }

        let k_e = 0.1337 * z1.powf(1.0 / 6.0) * (z1 / m1).sqrt();
        let se_eps = k_e * eps.sqrt();

        let alpha_star = 0.249 * (m2 / m1).powf(0.56) + 0.0035 * (m2 / m1).powf(1.5);
        let q_val = 1.5 + 0.008 * z2;
        let costh = ion.theta_rad.cos().max(0.01);

        0.042 * (q_val * alpha_star / u_s) 
            * (sn_e / (1.0 + 0.35 * u_s * se_eps)) 
            * (1.0 - (e_th / ion.energy_ev).sqrt()).powf(2.5) 
            * costh.powf(-1.5)
    }

    /// Evaluates slat surface erosion velocity in mm/year.
    pub fn surface_erosion_rate_mm_per_year(&self, ion: &IonBeam, shield_efficiency: f64) -> f64 {
        let yield_atoms = self.yamamura_sputtering_yield(ion);
        let eff_flux = ion.flux * (1.0 - shield_efficiency);
        let n_avogadro = 6.02214076e23;
        
        let m_target_kg_mol = self.material.atomic_m * 1e-3;
        let erosion_m_per_s = (yield_atoms * eff_flux * m_target_kg_mol) 
            / (self.material.rho * n_avogadro);

        erosion_m_per_s * 1000.0 * 3600.0 * 24.0 * 365.25
    }
}