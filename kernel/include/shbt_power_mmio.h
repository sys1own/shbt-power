/*
 * shbt_power_mmio.h — 128-byte, dual-cacheline C-ABI MMIO register standard
 * for the SHBT-Graser p-11B power plant (SHBT-MMIO-POWER normative
 * specification).
 *
 * Cacheline 0 (0x00-0x3F): microkernel control + ADM metric interlocks.
 * Cacheline 1 (0x40-0x7F): DEC telemetry + plant balance extension.
 * Anchored at physical base address 0x70000000, 64-byte aligned, zero-copy.
 */
#ifndef SHBT_POWER_MMIO_H
#define SHBT_POWER_MMIO_H

#include <stdint.h>
#include <stddef.h>

#define SHBT_POWER_BASE_ADDR 0x70000000UL

typedef struct __attribute__((packed, aligned(64))) {
    /* CACHELINE 0: Microkernel Control & ADM Metric Interlocks (0x00 - 0x3F) */
    volatile uint32_t sys_ctrl;            /* 0x00: System Control Word */
    volatile uint32_t sys_status;          /* 0x04: System Status Word */
    volatile uint32_t clock_ticks_lo;      /* 0x08: 100 Hz Epoch Counter Lo */
    volatile uint32_t clock_ticks_hi;      /* 0x0C: 100 Hz Epoch Counter Hi */
    volatile uint32_t pcss_gate_ctrl;      /* 0x10: PCSS Gate Laser Enable */
    volatile uint32_t crowbar_status;      /* 0x14: Solid-State Crowbar Flags */
    volatile float    adm_metric_err;      /* 0x18: ADM 3+1 Lapse Error (|det(g)+1|) */
    volatile float    adm_shift_norm;      /* 0x1C: ADM 3+1 Shift Norm (beta^i) */
    volatile uint32_t precision_flags;     /* 0x20: Arbitrary-Precision Flags */
    volatile float    quench_headroom;     /* 0x24: Magnet Quench Margin (K) */
    volatile float    beam_energy_gev;     /* 0x28: Linac Beam Energy (GeV) */
    volatile float    beam_focus_tune;     /* 0x2C: Graser Focus Quad Trim */
    volatile float    holo_entropy_gap;    /* 0x30: Topological Entropy Margin */
    volatile uint32_t dark_ledger_par;     /* 0x34: 23/33 Dark Ledger Checksum */
    volatile uint32_t recon_dma_stat;      /* 0x38: DMA Ring Buffer Status */
    volatile uint32_t _pad_align_cl0;      /* 0x3C: Enforces 64-Byte Cacheline 0 Boundary */

    /* CACHELINE 1: Direct Energy Conversion & Plant Balance Extension (0x40 - 0x7F) */
    volatile float    dec_grid1_volt;      /* 0x40: Venetian Stage 1 Voltage (MV) */
    volatile float    dec_grid2_volt;      /* 0x44: Venetian Stage 2 Voltage (MV) */
    volatile float    dec_grid3_volt;      /* 0x48: Venetian Stage 3 Voltage (MV) */
    volatile float    dec_alpha_curr_ka;   /* 0x4C: Collected Alpha Current (kA) */
    volatile float    mhd_pickup_curr_ka;  /* 0x50: Inductive HTS Coil Current (kA) */
    volatile float    wbg_panel_temp_k;    /* 0x54: First-Wall WBG Temperature (K) */
    volatile float    supercap_soc_pct;    /* 0x58: 450 MJ Buffer State-of-Charge */
    volatile float    lanr_array_net_kw;   /* 0x5C: 1,800-Module LANR Array Power */
    volatile float    teg_reclaim_mw;      /* 0x60: Waste Heat TEG Yield (MW) */
    volatile float    divertor_temp_k;     /* 0x64: Spindle-Cusp Divertor Temp */
    volatile float    target_pos_dev_um;   /* 0x68: Target Trajectory Jitter (um) */
    volatile uint32_t target_sync_word;    /* 0x6C: 100 Hz Laser Gate Lock */
    volatile uint32_t interlock_latch;     /* 0x70: Emergency Trip Latch */
    volatile uint32_t fault_injection_k;   /* 0x74: Sim Test Fault Injection Code */
    volatile uint32_t reserved_ext;        /* 0x78: Reserved Expansion */
    volatile uint32_t telemetry_crc32;     /* 0x7C: CRC-32/Castagnoli Checksum */
} shbt_power_mmio_t;

_Static_assert(sizeof(shbt_power_mmio_t) == 128, "shbt_power_mmio_t must be exactly 128 bytes");
_Static_assert(offsetof(shbt_power_mmio_t, dec_grid1_volt) == 64, "dec_grid1_volt must start Cacheline 1 at 0x40");
_Static_assert(offsetof(shbt_power_mmio_t, telemetry_crc32) == 124, "telemetry_crc32 must reside at offset 0x7C");

/* sys_status bit field */
#define SHBT_PWR_STATUS_READY        (1u << 0)
#define SHBT_PWR_STATUS_PLL_LOCK     (1u << 1)
#define SHBT_PWR_STATUS_ECC_ERR      (1u << 2)
#define SHBT_PWR_STATUS_CROWBAR_TRIP (1u << 3)
#define SHBT_PWR_STATUS_ADM_FAULT    (1u << 4)
#define SHBT_PWR_STATUS_MMIO_OK      (1u << 5)

/* interlock_latch bit field */
#define SHBT_PWR_LATCH_PCSS_FIRED    (1u << 0)
#define SHBT_PWR_LATCH_CROWBAR_DONE  (1u << 1)
#define SHBT_PWR_LATCH_ADM_LOCKOUT   (1u << 2)

#endif
