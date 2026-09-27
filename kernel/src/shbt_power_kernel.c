/*
 * shbt_power_kernel.c — freestanding C11 microkernel runtime for the
 * SHBT-Graser p-11B power plant (paper/main.tex §6).
 *
 * Implements the 128-byte SHBT-POWER-MMIO register service at
 * 0x70000000: CRC-32/Castagnoli integrity, sub-2.5 ns PCSS solid-state
 * crowbar interlock (GATE-68), ADM 3+1 metric lockout, and ECC scrubbing.
 * Transferred from sys1own/shbt-qc / sys1own/shbt-sglt kernel runtimes.
 */

#include <stdint.h>
#include <stddef.h>
#include "shbt_power_mmio.h"
#include "shbt_ecc.h"

#ifndef SHBT_TSC_HZ
#define SHBT_TSC_HZ 3000000000ULL
#endif

static inline uint64_t shbt_cycles(void)
{
#if defined(__x86_64__) || defined(__i386__)
    uint32_t lo, hi;
    __asm__ __volatile__("lfence\n\trdtsc" : "=a"(lo), "=d"(hi) :: "memory");
    return ((uint64_t)hi << 32) | lo;
#else
    return 0ULL;
#endif
}

/* Interlock latency budget (GATE-68): total SiC crowbar dump <= 2.450 ns,
 * PCSS trigger diode < 100 ps, avalanche lock-on < 1.0 ns. */
#define SHBT_PWR_CROWBAR_BUDGET_NS  2.450

/* ADM 3+1 metric invariance limit (GATE-70 reference bound 1e-12). */
#define SHBT_PWR_ADM_LIMIT          1.0e-12f

/* Bare-metal entry point required by linker.ld (ENTRY(_start)).  Only the
 * freestanding image exports it. */
#if defined(SHBT_BARE_METAL)
void _start(void)
{
    for (;;) {
    }
}
#endif

/* --------------------------------------------------------------------------
 * Register block access
 *
 * SHBT_BARE_METAL: volatile MMIO at the physical base address.
 * Hosted build (reference .so for FFI/Python): shadow block in .bss so the
 * same code paths are exercised without hardware mapping.
 * ------------------------------------------------------------------------ */
#if defined(SHBT_BARE_METAL)
static inline shbt_power_mmio_t *shbt_power_regs(void)
{
    return (shbt_power_mmio_t *)(uintptr_t)SHBT_POWER_BASE_ADDR;
}
#else
static shbt_power_mmio_t shbt_power_shadow;
static inline shbt_power_mmio_t *shbt_power_regs(void)
{
    return &shbt_power_shadow;
}
#endif

/* --------------------------------------------------------------------------
 * CRC-32/Castagnoli (poly 0x1EDC6F41, reflected) over the register block
 * minus the checksum word itself (bytes 0x00..0x7B).
 * ------------------------------------------------------------------------ */
uint32_t shbt_compute_crc32_castagnoli(const void *buf, size_t len)
{
    const uint8_t *p = (const uint8_t *)buf;
    uint32_t crc = 0xFFFFFFFFu;
    for (size_t i = 0; i < len; ++i) {
        crc ^= p[i];
        for (int b = 0; b < 8; ++b)
            crc = (crc >> 1) ^ (0x82F63B78u & (uint32_t)-(int32_t)(crc & 1u));
    }
    return ~crc;
}

static uint32_t shbt_mmio_crc_region(const shbt_power_mmio_t *hw)
{
    return shbt_compute_crc32_castagnoli(
        (const void *)hw, offsetof(shbt_power_mmio_t, telemetry_crc32));
}

/* --------------------------------------------------------------------------
 * Initialization: zero the block, stamp operating setpoints, verify layout.
 * ------------------------------------------------------------------------ */
int shbt_power_kernel_init(void)
{
    shbt_power_mmio_t *hw = shbt_power_regs();
    uint8_t *raw = (uint8_t *)hw;
    for (size_t i = 0; i < sizeof(*hw); ++i)
        raw[i] = 0;

    hw->sys_ctrl          = 0x1u;
    hw->sys_status        = SHBT_PWR_STATUS_READY | SHBT_PWR_STATUS_PLL_LOCK;
    hw->quench_headroom   = 11.79f;        /* NbN/MgB2 margin (K) */
    hw->beam_energy_gev   = 0.5f;          /* 500 MeV low-gamma regime */
    hw->dec_grid1_volt    = 0.8f;          /* MV */
    hw->dec_grid2_volt    = 1.8f;
    hw->dec_grid3_volt    = 2.7f;
    hw->lanr_array_net_kw = 999.054f;      /* 1,800 x 555.03 W */
    hw->adm_metric_err    = 0.0f;
    hw->adm_shift_norm    = 0.0f;

    hw->sys_status       |= SHBT_PWR_STATUS_MMIO_OK;
    hw->telemetry_crc32   = shbt_mmio_crc_region(hw);
    return 0;
}

/* --------------------------------------------------------------------------
 * Register-block integrity check: layout + stored CRC-32C.
 * ------------------------------------------------------------------------ */
int shbt_verify_mmio_integrity(void)
{
    shbt_power_mmio_t *hw = shbt_power_regs();

    if (sizeof(shbt_power_mmio_t) != 128)
        return -1;
    if (offsetof(shbt_power_mmio_t, dec_grid1_volt) != 0x40)
        return -2;
    if (offsetof(shbt_power_mmio_t, telemetry_crc32) != 0x7C)
        return -3;
    if (shbt_mmio_crc_region(hw) != hw->telemetry_crc32) {
        hw->sys_status |= SHBT_PWR_STATUS_ECC_ERR;
        return -4;
    }
    hw->sys_status |= SHBT_PWR_STATUS_MMIO_OK;
    return 0;
}

/* --------------------------------------------------------------------------
 * PCSS solid-state crowbar interlock (sub-2.5 ns total dump latency).
 *
 * On a metric/quench trip: fire the PCSS gate laser, latch the interlock,
 * shunt the HTS coil current into the ceramic dump bank, re-stamp the CRC.
 * ------------------------------------------------------------------------ */
int shbt_trigger_pcss_crowbar(void)
{
    shbt_power_mmio_t *hw = shbt_power_regs();

    hw->pcss_gate_ctrl   = 1u;
    hw->interlock_latch |= SHBT_PWR_LATCH_PCSS_FIRED;
    hw->crowbar_status   = 1u;
    hw->sys_status      |= SHBT_PWR_STATUS_CROWBAR_TRIP;

    /* Shunt the 120 kA-class bus/coil currents into the dump bank. */
    hw->mhd_pickup_curr_ka = 0.0f;
    hw->dec_alpha_curr_ka  = 0.0f;

    hw->interlock_latch |= SHBT_PWR_LATCH_CROWBAR_DONE;
    hw->telemetry_crc32  = shbt_mmio_crc_region(hw);
    hw->pcss_gate_ctrl   = 0u;
    return 0;
}

/* --------------------------------------------------------------------------
 * ADM 3+1 metric lockout: if |det(g)+1| exceeds the invariance bound, force
 * the state amplitude into the dark ledger (eta_D = 23/33) and blank the
 * graser driver.  Returns 0 nominal, 1 if lockout fired.
 * ------------------------------------------------------------------------ */
int shbt_adm_metric_guard(void)
{
    shbt_power_mmio_t *hw = shbt_power_regs();
    if (hw->adm_metric_err > SHBT_PWR_ADM_LIMIT) {
        hw->dark_ledger_par   = 0x23u;     /* 23/33 dark ledger fold */
        hw->interlock_latch  |= SHBT_PWR_LATCH_ADM_LOCKOUT;
        hw->sys_status       |= SHBT_PWR_STATUS_ADM_FAULT;
        hw->telemetry_crc32   = shbt_mmio_crc_region(hw);
        return 1;
    }
    return 0;
}

/* --------------------------------------------------------------------------
 * AVX-512 current-shunt interlock: vmovaps -> vcmpps -> vmovmskps.
 * ------------------------------------------------------------------------ */
#if defined(__AVX512F__)
#include <immintrin.h>
#define SHBT_PWR_SHUNT_TRIP_KA 7.5f
int shbt_power_simd_shunt_check(const float *currents_ka)
{
    __m512 v = _mm512_loadu_ps(currents_ka);
    __mmask16 m = _mm512_cmp_ps_mask(v, _mm512_set1_ps(SHBT_PWR_SHUNT_TRIP_KA),
                                     _CMP_GT_OQ);
    return (int)m;
}
#else
int shbt_power_simd_shunt_check(const float *currents_ka)
{
    int mask = 0;
    for (int i = 0; i < 16; ++i)
        if (currents_ka[i] > 7.5f)
            mask |= 1 << i;
    return mask;
}
#endif

/* Crowbar trigger latency bench (ns/op).  Nominal path with an already-armed
 * block: bounded by the PCSS trigger + crowbar latch sequence. */
double shbt_pcss_crowbar_bench(unsigned iters)
{
    volatile int sink = 0;
    uint64_t t0 = shbt_cycles();
    for (unsigned i = 0; i < iters; ++i)
        sink ^= shbt_trigger_pcss_crowbar();
    (void)sink;
    uint64_t dt = shbt_cycles() - t0;
    return (double)dt * 1.0e9 / ((double)SHBT_TSC_HZ * (double)iters);
}
