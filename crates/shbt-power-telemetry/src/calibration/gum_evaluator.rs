
/// High-precision GUM (JCGM 100:2008) Covariance Budget Evaluator for the
/// 1,325 MW Thermal Enthalpy Balance Loop. Operates strictly without heap allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThermalPowerBudget {
    pub mass_flow_kg_s: f64,
    pub mass_flow_unc_kg_s: f64,
    pub delta_temp_k: f64,
    pub delta_temp_unc_k: f64,
    pub cp_mean_j_kg_k: f64,
    pub cp_mean_unc_j_kg_k: f64,
    pub heat_leak_mw: f64,
    pub heat_leak_unc_mw: f64,
    pub corr_flow_temp: f64,
}

impl Default for ThermalPowerBudget {
    fn default() -> Self {
        Self {
            mass_flow_kg_s: 425.23812,
            mass_flow_unc_kg_s: 0.850,
            delta_temp_k: 600.0,
            delta_temp_unc_k: 0.950,
            cp_mean_j_kg_k: 5193.15,
            cp_mean_unc_j_kg_k: 4.200,
            heat_leak_mw: 15.005,
            heat_leak_unc_mw: 1.150,
            corr_flow_temp: 0.135503,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GumEvaluationResult {
    pub p_thermal_mw: f64,
    pub combined_variance_mw2: f64,
    pub combined_uncertainty_mw: f64,
    pub sensitivity_flow: f64,
    pub sensitivity_temp: f64,
    pub sensitivity_cp: f64,
    pub sensitivity_leak: f64,
    pub expanded_uncertainty_k2_mw: f64,
}

pub struct GumCovarianceEvaluator;

impl GumCovarianceEvaluator {
    /// Computes the combined standard uncertainty u_c(P) via the full sensitivity matrix
    /// u_c^2(P) = sum_i (dP/dx_i)^2 u^2(x_i) + 2 sum_{i != j} (dP/dx_i)(dP/dx_j) u(x_i, x_j)
    #[inline]
    pub fn evaluate_thermal_uncertainty(budget: &ThermalPowerBudget) -> GumEvaluationResult {
        // P = m_dot * Cp * Delta_T * 1e-6 - Q_leak  [MW]
        let p_gross = budget.mass_flow_kg_s * budget.cp_mean_j_kg_k * budget.delta_temp_k * 1e-6;
        let p_thermal = p_gross - budget.heat_leak_mw;

        // First-order partial sensitivity coefficients
        let c_flow = budget.cp_mean_j_kg_k * budget.delta_temp_k * 1e-6; // MW / (kg/s)
        let c_temp = budget.mass_flow_kg_s * budget.cp_mean_j_kg_k * 1e-6; // MW / K
        let c_cp   = budget.mass_flow_kg_s * budget.delta_temp_k * 1e-6;   // MW / (J/(kg K))
        let c_leak = -1.0_f64;                                            // MW / MW

        // Uncorrelated variance components: c_i^2 * u^2(x_i)
        let var_flow = (c_flow * budget.mass_flow_unc_kg_s).powi(2);
        let var_temp = (c_temp * budget.delta_temp_unc_k).powi(2);
        let var_cp   = (c_cp * budget.cp_mean_unc_j_kg_k).powi(2);
        let var_leak = (c_leak * budget.heat_leak_unc_mw).powi(2);

        let var_uncorrelated = var_flow + var_temp + var_cp + var_leak;

        // Correlated covariance cross-term between mass flow and temperature rise
        // Cov(m, T) = r(m, T) * u(m) * u(T)
        let cov_flow_temp = budget.corr_flow_temp * budget.mass_flow_unc_kg_s * budget.delta_temp_unc_k;
        let cross_term = 2.0 * c_flow * c_temp * cov_flow_temp;

        let total_variance = var_uncorrelated + cross_term;
        let combined_unc = total_variance.sqrt();
        let expanded_unc = combined_unc * 2.0; // Coverage factor k = 2 (95.45% CL)

        GumEvaluationResult {
            p_thermal_mw: p_thermal,
            combined_variance_mw2: total_variance,
            combined_uncertainty_mw: combined_unc,
            sensitivity_flow: c_flow,
            sensitivity_temp: c_temp,
            sensitivity_cp: c_cp,
            sensitivity_leak: c_leak,
            expanded_uncertainty_k2_mw: expanded_unc,
        }
    }
}