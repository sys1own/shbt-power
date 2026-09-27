//! Physical SI constants and plant setpoints (paper/main.tex §2-§6, paper/supplementary.tex).
//! Values are exact decimal literals from the specification tables.
#![allow(clippy::inconsistent_digit_grouping)]

// ---- SI constants (exact SI / CODATA-defining) ----
pub const C_LIGHT: f64 = 299_792_458.0;          // m/s
pub const E_CHARGE: f64 = 1.602_176_634e-19;     // C
pub const M_ELECTRON_MEV: f64 = 0.510_998_95;    // MeV
pub const M_ALPHA_MEV: f64 = 3727.379_4;         // MeV (4He)
pub const M_ALPHA_KG: f64 = 6.644_657_3357e-27;  // kg
pub const EPS0: f64 = 8.854_187_8128e-12;        // F/m
pub const MU0: f64 = 1.256_637_062_12e-6;        // H/m
pub const H_EV_S: f64 = 4.135_667_696e-15;       // eV·s
pub const U_AMU_KG: f64 = 1.660_539_066_60e-27;  // kg

// ---- Graser driver / C-band linac (paper/main.tex §3, paper/supplementary.tex §1) ----
pub const F_RF_GHZ: f64 = 5.712;                 // C-band carrier
pub const T_RF_PS: f64 = 175.070;                // RF bucket period
pub const N_MICRO_BUNCHES: u32 = 2500;           // bunches per macro-burst
pub const E_MICRO_J: f64 = 100.0;                // per micro-pulse
pub const TAU_MICRO_PS: f64 = 1.0;               // micro-pulse width
pub const E_MACRO_KJ: f64 = 250.0;               // per macro-shot
pub const F_REP_HZ: f64 = 100.0;
pub const P_GRASER_BEAM_MW: f64 = 25.0;
pub const P_GRASER_ELEC_MW: f64 = 125.0;
pub const ETA_GRASER: f64 = 0.20;
pub const LAMBDA_SEED_NM: f64 = 1064.0;
pub const I_PEAK_LIMIT_W_CM2: f64 = 1.0e26;      // intra-cavity ceiling
pub const P_DRIFT_TORR: f64 = 1.0e-10;           // UHV drift bound

// ---- p-11B ignition kinetics (paper/main.tex §4) ----
pub const P_FUSION_MW: f64 = 8750.0;
pub const Q_FUSION: f64 = 350.0;
pub const F_BURN: f64 = 0.35;
pub const E_YIELD_MJ: f64 = 87.5;                // per shot
pub const Q_PB11_MEV: f64 = 8.7;                 // Q-value per reaction
pub const N_FUEL_M3: f64 = 1.0e26;
pub const E_ALPHA_MEV: f64 = 2.9;
pub const RES_C12_MEV: [f64; 3] = [2.12, 4.44, 8.92];

// ---- DEC channel allocation (paper/main.tex §5) ----
pub const FRAC_ALPHA: f64 = 0.80;
pub const FRAC_MHD: f64 = 0.15;
pub const FRAC_RAD: f64 = 0.05;
pub const ETA_ELEC: f64 = 0.8750;
pub const ETA_MHD: f64 = 0.90;
pub const ETA_RAD: f64 = 0.50;
pub const V_GRID_MV: [f64; 3] = [0.80, 1.80, 2.70];
pub const B_GUIDE_T: f64 = 1.0;
pub const B_CORE_T: f64 = 5.0;
pub const B_COLL_T: f64 = 0.05;
pub const R_CORE_M: f64 = 0.25;

// ---- Chamber / MHD (paper/main.tex §5, paper/supplementary.tex §7) ----
pub const R_WALL_M: f64 = 2.20;
pub const CUSHION_MIN_M: f64 = 0.50;
pub const B_MIN_T: f64 = 3.5;
pub const B_NOMINAL_T: f64 = 4.0;
pub const B_MAX_T: f64 = 5.0;
pub const E_PLASMA_MJ: f64 = 70.0;

// ---- Thermal reclamation (paper/main.tex §6) ----
pub const P_THERMAL_IN_MW: f64 = 1325.0;
pub const ETA_TEG: f64 = 0.33804;
pub const TEG_T_HOT_K: f64 = 900.0;
pub const TEG_T_MID_K: f64 = 600.0;
pub const TEG_T_COLD_K: f64 = 300.0;
pub const HE_FLOW_M_S: f64 = 150.0;
pub const HE_PRESSURE_MPA: f64 = 10.0;
pub const HE_MDOT_KG_S: f64 = 450.0;

// ---- LANR starter grid (paper/main.tex §6, upstream shbt-cf ledger) ----
pub const LANR_MODULE_COUNT: u32 = 1800;
pub const LANR_MODULE_NET_W: f64 = 555.03;
pub const LANR_MODULE_THERMAL_W: f64 = 3093.44;
pub const LANR_MODULE_TEG_W: f64 = 1045.58;
pub const SUPERCAP_MJ: f64 = 450.0;
pub const ETA_CHG: f64 = 0.95;
pub const ETA_SIC_CROWBAR: f64 = 0.9420;

// ---- Plant ledger (paper/main.tex §6) ----
pub const P_AUX_MW: f64 = 15.0;
pub const P_RECIRC_MW: f64 = P_GRASER_ELEC_MW + P_AUX_MW;

// ---- Fuel target (paper/supplementary.tex §9) ----
pub const PELLET_MASS_MG: f64 = 3.708;
pub const PELLET_DIAMETER_MM: f64 = 1.96;
pub const V_INJ_M_S: f64 = 250.0;
pub const L_FLIGHT_M: f64 = 2.50;
pub const JITTER_POS_UM: f64 = 50.0;
pub const JITTER_TIME_PS: f64 = 50.0;

// ---- Safety interlocks (paper/main.tex §7) ----
pub const PCSS_TRIGGER_PS: f64 = 100.0;
pub const PCSS_LOCKON_NS: f64 = 1.0;
pub const CROWBAR_DUMP_NS: f64 = 2.450;
pub const MU_PB_H_GAMMA: f64 = 0.476;            // cm^-1 at 2.223 MeV
pub const QUENCH_MARGIN_K: f64 = 11.79;
pub const MMIO_BASE_ADDR: u64 = 0x7000_0000;

// ---- Holography (paper/main.tex §2) ----
pub const ETA_DARK_NUM: u64 = 23;
pub const ETA_DENOM: u64 = 33;
pub const KOJIMA_C: f64 = 1.0e20;
pub const ADM_INVARIANCE_LIMIT: f64 = 1.0e-12;
