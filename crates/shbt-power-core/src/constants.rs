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

// ---- Legacy LANR aggregation constants (retained for the repurposed
// TEG/bus thermal model; the starter role is superseded by the graser
// isomer battery — upstream sys1own/shbt-warp & sys1own/shbt-ghost) ----
pub const LANR_MODULE_COUNT: u32 = 1800;
pub const LANR_MODULE_NET_W: f64 = 555.03;
pub const LANR_MODULE_THERMAL_W: f64 = 3093.44;
pub const LANR_MODULE_TEG_W: f64 = 1045.58;
pub const SUPERCAP_MJ: f64 = 450.0;
pub const ETA_CHG: f64 = 0.95;
pub const ETA_SIC_CROWBAR: f64 = 0.9420;

// ---- Solid-state coherent graser nuclear isomer battery (paper/main.tex
// §5.4; upstream sys1own/shbt-warp & sys1own/shbt-ghost) ----
pub const ISOMER_CORE_MASS_KG: f64 = 376.99;      // enriched 178m2Hf core
pub const ISOMER_ENERGY_DENSITY_TJ_KG: f64 = 1.326_295; // rho_E
pub const ISOMER_STORED_TJ: f64 = 500.0;          // E_stored
pub const ISOMER_HALF_LIFE_S: f64 = 9.782e8;      // T_1/2 = 31.0 yr
pub const ISOMER_SEED_KEV: f64 = 40.0;            // X-ray trigger seed
pub const ISOMER_RELEASE_MEV: f64 = 2.446;        // per-nucleus release
pub const ISOMER_TRIGGER_GAIN: f64 = 61.15;       // G >= 60.0
pub const ETA_DEC_ISOMER: f64 = 0.458;            // 3-stage relativistic DEC
pub const P_BATTERY_BURST_MW: f64 = 140.0;        // bootstrap bus injection
pub const TAU_BOOT_S: f64 = 0.85;                 // bootstrap pulse window
pub const TAU_BOOT_LIMIT_S: f64 = 1.00;           // cold-start gate bound
pub const BUS_PRECHARGE_KV: f64 = 15.0;
pub const BUS_DISCHARGE_MAX_KV: f64 = 400.0;

// ---- 20 K cryogenic helium sub-loop & shield reclamation ----
pub const CRYO_CORE_TEMP_K: f64 = 21.13;
pub const CRYO_CRIT_TEMP_K: f64 = 32.92;          // Mossbauer de-pinning
pub const CRYO_HEADROOM_K: f64 = 11.79;
pub const CRYO_PRESS_MPA: f64 = 2.0;
pub const CRYO_CP_KJ_KG_K: f64 = 5.193;
pub const CRYO_DT_K: f64 = 3.13;                  // 18.00 -> 21.13 K lift
pub const CRYO_MDOT_KG_S: f64 = 21.795;
pub const CRYO_ELEC_MW: f64 = 16.70;              // cryocooler electric draw
pub const DECAY_HEAT_KW: f64 = 354.27;            // quiescent decay heat
pub const TEG_SHIELD_KW: f64 = 28.50;             // 80 K shield TEG recovery

// ---- Lattice-coherence metrics ----
pub const MOSSBAUER_RECOIL_FRAC: f64 = 0.782;     // f_M >= 0.74
pub const MOSSBAUER_MIN: f64 = 0.74;
pub const BORRMANN_SUPPRESS: f64 = 0.9852;        // eps_B >= 0.985
pub const BORRMANN_MIN: f64 = 0.985;

// ---- Repurposed supercapacitor synthetic inertia ----
pub const SYNTH_INERTIA_MS: f64 = 57.45;          // H_buffer = E/S_base
pub const DROOP_INJECT_MW: f64 = 1958.23;         // R = 4%, df = -0.50 Hz
pub const DROOP_HOLD_MS: f64 = 229.8;             // 450 MJ / 1958.23 MW
pub const DROOP_PCT: f64 = 4.0;                   // governor droop 4-5 %
pub const GRID_FREQ_HZ: f64 = 50.0;

// ---- PCSS crowbar protection (GATE-BAT-04/05) ----
pub const PCSS_QUENCH_NS: f64 = 2.05;             // <= 2.10 ns budget
pub const PCSS_QUENCH_BUDGET_NS: f64 = 2.10;
pub const ETA_INDUCTIVE_RECOVERY_PCT: f64 = 94.45; // >= 94.20 %
pub const ETA_INDUCTIVE_RECOVERY_MIN_PCT: f64 = 94.20;

// ---- Plant ledger (paper/main.tex §6) ----
pub const P_AUX_MW: f64 = 15.0;
pub const P_RECIRC_MW: f64 = P_GRASER_ELEC_MW + P_AUX_MW;
pub const P_NET_EXPORT_MW: f64 = 7832.903;
pub const P_GROSS_MW: f64 = 7972.903;

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
pub const MMIO_MAGIC: u32 = 0x5348_4254;          // "SHBT"
pub const MMIO_VERSION: u32 = 0x0002_0000;        // v2.0.0

// ---- Holography (paper/main.tex §2) ----
pub const ETA_DARK_NUM: u64 = 23;
pub const ETA_DENOM: u64 = 33;
pub const KOJIMA_C: f64 = 1.0e20;
pub const ADM_INVARIANCE_LIMIT: f64 = 1.0e-12;
