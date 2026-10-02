/*
 * shbt_power_mmio.h — Memory-Mapped I/O Register Interface for SHBT-POWER.
 *
 * Contract: Exactly 128 bytes, 64-byte aligned, zero heap allocation.
 * Cacheline 0 (0x00 - 0x3F): Plant Control, Grid State, and Output Telemetry.
 * Cacheline 1 (0x40 - 0x7F): Thermal-Hydraulics, Protection, and Isomer
 * Battery telemetry.
 *
 * The solid-state coherent graser nuclear isomer battery (376.99 kg of
 * enriched 178m2Hf, 500 TJ stored) and the 3-stage relativistic DEC stack
 * originate upstream in sys1own/shbt-warp and sys1own/shbt-ghost; they
 * supersede the legacy 1,800-module LANR starter array whose register at
 * 0x5C is replaced by the isomer battery telemetry block.
 */
#ifndef SHBT_POWER_MMIO_H
#define SHBT_POWER_MMIO_H

#include <stdint.h>
#include <stddef.h>

#define SHBT_POWER_BASE_ADDR          0x70000000UL

#define SHBT_MMIO_MAGIC_VALUE         0x53484254U /* "SHBT" in ASCII */
#define SHBT_MMIO_VERSION_CURRENT     0x00020000U /* Version 2.0.0 */
#define SHBT_MMIO_TOTAL_SIZE_BYTES    128U
#define SHBT_CACHELINE_SIZE_BYTES     64U

typedef enum shbt_plant_lifecycle_state {
    SHBT_STATE_COLD_STANDBY            = 0U,
    SHBT_STATE_ISOMER_ARMING           = 1U,
    SHBT_STATE_GRASER_IGNITION_PULSE   = 2U,
    SHBT_STATE_DEC_BOOTSTRAP           = 3U,
    SHBT_STATE_STEADY_STATE_RECIRC     = 4U
} shbt_plant_lifecycle_state_t;

typedef struct shbt_power_mmio {
    /* ===================================================================== */
    /* CACHELINE 0: 0x00 - 0x3F (64 Bytes)                                   */
    /* ===================================================================== */
    volatile uint32_t magic;                    /* 0x00: Hardware Magic (0x53484254) */
    volatile uint32_t version;                  /* 0x04: Interface Version */
    volatile uint32_t plant_state;              /* 0x08: shbt_plant_lifecycle_state_t */
    volatile uint32_t control_flags;            /* 0x0C: Control and Command Bits */
    volatile uint64_t uptime_ticks;             /* 0x10: 10 kHz Kernel Timer Ticks */
    volatile float    net_export_mw;            /* 0x18: Net Export (7832.903 MW nom) */
    volatile float    gross_output_mw;          /* 0x1C: Gross Output (7972.903 MW nom) */
    volatile float    recirc_load_mw;           /* 0x20: Total Recirc (140.000 MW nom) */
    volatile float    linac_load_mw;            /* 0x24: Linac Load (125.000 MW nom) */
    volatile float    bop_load_mw;              /* 0x28: BoP Load (15.000 MW nom) */
    volatile float    linac_rf_freq_ghz;        /* 0x2C: Linac RF Freq (5.712 GHz nom) */
    volatile float    supercap_stored_mj;       /* 0x30: Supercap Buffer (450.0 MJ nom) */
    volatile float    grid_freq_hz;             /* 0x34: Grid Frequency (50.00 Hz nom) */
    volatile float    grid_droop_pct;           /* 0x38: Configured Droop (4.0 - 5.0%) */
    volatile uint32_t fault_code;               /* 0x3C: System Fault Bitfield */

    /* ===================================================================== */
    /* CACHELINE 1: 0x40 - 0x7F (64 Bytes)                                   */
    /* ===================================================================== */
    volatile float    she_loop_temp_cold_k;     /* 0x40: sHe Loop Cold Leg (300.0 K) */
    volatile float    she_loop_temp_hot_k;      /* 0x44: sHe Loop Hot Leg (900.0 K) */
    volatile float    she_loop_press_mpa;       /* 0x48: sHe Pressure (10.0 MPa nom) */
    volatile float    teg_reclaim_kw;           /* 0x4C: TEG Standby Reclaim (~28.5 kW) */
    volatile float    cryo_subloop_mass_flow;   /* 0x50: 20K He Flow (~21.795 kg/s) */
    volatile float    pcss_crowbar_quench_ns;   /* 0x54: Crowbar Speed (<= 2.10 ns) */
    volatile float    pcss_inductive_recov_pct; /* 0x58: Inductive Recovery (>= 94.2%) */

    /* Telemetry fields replacing legacy LANR at 0x5C */
    volatile float    battery_core_temp_k;      /* 0x5C: Isomer Core Temp (21.13 K nom) */
    volatile float    battery_cryo_headroom_k;  /* 0x60: Cryo Headroom (11.79 K nom) */
    volatile float    battery_soc;              /* 0x64: Battery SoC (0.000 - 1.000) */
    volatile float    battery_bus_voltage_kv;   /* 0x68: Bus Discharge (15.0 - 400.0 kV)*/

    /* Secondary Telemetry & Audit Registers */
    volatile float    battery_decay_heat_kw;    /* 0x6C: Quiescent Heat (354.27 kW nom) */
    volatile float    mossbauer_recoil_frac;    /* 0x70: f_M Fraction (>= 0.74 nom) */
    volatile float    borrmann_suppress_factor; /* 0x74: epsilon_B Factor (>= 0.985) */
    volatile uint32_t audit_gate_status_bits;   /* 0x78: Gates 01-32 Pass/Fail Bits */
    volatile uint32_t audit_gate_extended_bits; /* 0x7C: Gates 33-64 Pass/Fail Bits */
} __attribute__((aligned(64))) shbt_power_mmio_t;

/* ========================================================================= */
/* COMPILE-TIME VERIFICATION ASSERTIONS (C11)                                */
/* ========================================================================= */

_Static_assert(sizeof(shbt_power_mmio_t) == 128,
    "SHBT MMIO Contract Violation: Struct size must be exactly 128 bytes");

_Static_assert(_Alignof(shbt_power_mmio_t) == 64,
    "SHBT MMIO Contract Violation: Struct must be 64-byte cacheline aligned");

_Static_assert(offsetof(shbt_power_mmio_t, magic) == 0x00,
    "Offset mismatch: magic must be at 0x00");

_Static_assert(offsetof(shbt_power_mmio_t, plant_state) == 0x08,
    "Offset mismatch: plant_state must be at 0x08");

_Static_assert(offsetof(shbt_power_mmio_t, net_export_mw) == 0x18,
    "Offset mismatch: net_export_mw must be at 0x18");

_Static_assert(offsetof(shbt_power_mmio_t, fault_code) == 0x3C,
    "Offset mismatch: fault_code must be at 0x3C");

_Static_assert(offsetof(shbt_power_mmio_t, she_loop_temp_cold_k) == 0x40,
    "Offset mismatch: Cacheline 1 start must be at 0x40");

_Static_assert(offsetof(shbt_power_mmio_t, battery_core_temp_k) == 0x5C,
    "Offset mismatch: battery_core_temp_k must replace legacy LANR at 0x5C");

_Static_assert(offsetof(shbt_power_mmio_t, battery_cryo_headroom_k) == 0x60,
    "Offset mismatch: battery_cryo_headroom_k must be at 0x60");

_Static_assert(offsetof(shbt_power_mmio_t, battery_soc) == 0x64,
    "Offset mismatch: battery_soc must be at 0x64");

_Static_assert(offsetof(shbt_power_mmio_t, battery_bus_voltage_kv) == 0x68,
    "Offset mismatch: battery_bus_voltage_kv must be at 0x68");

_Static_assert(offsetof(shbt_power_mmio_t, audit_gate_extended_bits) == 0x7C,
    "Offset mismatch: audit_gate_extended_bits must be at 0x7C");

/* control_flags bit field */
#define SHBT_PWR_CTRL_START_CMD        (1u << 0) /* verified plant start */
#define SHBT_PWR_CTRL_PCSS_READY       (1u << 1) /* PCSS crowbar armed+nominal */
#define SHBT_PWR_CTRL_PCSS_FIRED       (1u << 2) /* crowbar shunt triggered */
#define SHBT_PWR_CTRL_SEED_LASER_ON    (1u << 3) /* 40 keV graser seed active */
#define SHBT_PWR_CTRL_DROOP_TRACK      (1u << 4) /* synthetic-inertia droop */

/* fault_code bit field */
#define SHBT_PWR_FAULT_CRYO_LOW        (1u << 0) /* cryo headroom collapsed */
#define SHBT_PWR_FAULT_MOSSBAUER       (1u << 1) /* f_M below 0.74 */
#define SHBT_PWR_FAULT_BORRMANN        (1u << 2) /* eps_B below 0.985 */
#define SHBT_PWR_FAULT_BUS_OVERVOLT    (1u << 3) /* V_bus > 400 kV */
#define SHBT_PWR_FAULT_MAGIC           (1u << 4) /* magic/version mismatch */

#endif /* SHBT_POWER_MMIO_H */
