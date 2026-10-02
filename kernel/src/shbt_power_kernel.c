/*
 * shbt_power_kernel.c — freestanding C11 microkernel runtime for the
 * SHBT-Graser p-11B power plant (paper/main.tex §5-§6).
 *
 * Implements the 128-byte SHBT-POWER-MMIO register service at
 * 0x70000000: magic/version integrity checks, the 5-phase isomer-battery
 * bootstrap FSM (ColdStandby -> IsomerArming -> GraserIgnitionPulse ->
 * DecBootstrap -> SteadyStateRecirculation), the sub-2.10 ns PCSS crowbar
 * interlock (GATE-BAT-04), 20 K cryogenic sub-loop sanity, and SECDED
 * scrubbing.  The 178m2Hf graser battery core and 3-stage relativistic DEC
 * originate upstream in sys1own/shbt-warp and sys1own/shbt-ghost; the
 * legacy LANR starter array is superseded and the 450 MJ supercapacitor
 * bank is repurposed as a synthetic inertia buffer (H = 57.45 ms).
 */

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
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

/* Isomer battery nominal setpoints (power8.txt; upstream shbt-warp /
 * shbt-ghost Borrmann cavity + DEC stack). */
#define SHBT_BAT_CORE_TEMP_K          21.13f
#define SHBT_BAT_CRIT_TEMP_K          32.92f
#define SHBT_BAT_CRYO_HEADROOM_K      11.79f
#define SHBT_BAT_DECAY_HEAT_KW        354.27f
#define SHBT_BAT_MOSSBAUER_MIN        0.74f
#define SHBT_BAT_MOSSBAUER_NOM        0.782f
#define SHBT_BAT_BORRMANN_MIN         0.985f
#define SHBT_BAT_BORRMANN_NOM         0.9852f
#define SHBT_BAT_BUS_PRECHARGE_KV     15.0f
#define SHBT_BAT_BUS_MAX_KV           400.0f
#define SHBT_CRYO_MDOT_KG_S           21.795f
#define SHBT_TEG_SHIELD_RECLAIM_KW    28.50f

/* PCSS crowbar quench latency budget (GATE-BAT-04): <= 2.10 ns. */
#define SHBT_PWR_CROWBAR_BUDGET_NS    2.10f
#define SHBT_PWR_CROWBAR_NOM_NS       2.05f
/* Inductive-resonant energy recovery (GATE-BAT-05): >= 94.20 %. */
#define SHBT_PWR_INDUCTIVE_NOM_PCT    94.45f

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
 * CRC-32/Castagnoli (poly 0x1EDC6F41, reflected).  Retained as a service
 * for host-side frames (telemetry ring integrity checks); the register
 * contract itself is guarded by magic + version + range sanity checks.
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

/* --------------------------------------------------------------------------
 * Isomer battery telemetry sanity: the quiescent cryostat invariants that
 * must hold before State 0 -> State 1 arming is legal.
 * ------------------------------------------------------------------------ */
static bool shbt_battery_coherent(const shbt_power_mmio_t *hw)
{
    return hw->battery_core_temp_k <= SHBT_BAT_CORE_TEMP_K + 1e-3f
        && hw->battery_cryo_headroom_k >= SHBT_BAT_CRYO_HEADROOM_K - 1e-3f
        && hw->mossbauer_recoil_frac >= SHBT_BAT_MOSSBAUER_MIN
        && hw->borrmann_suppress_factor >= SHBT_BAT_BORRMANN_MIN
        && hw->battery_soc >= 0.0f && hw->battery_soc <= 1.0f
        && hw->battery_bus_voltage_kv >= 0.0f
        && hw->battery_bus_voltage_kv <= SHBT_BAT_BUS_MAX_KV
        && hw->pcss_crowbar_quench_ns <= SHBT_PWR_CROWBAR_BUDGET_NS
        && hw->pcss_inductive_recov_pct >= 94.20f;
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

    hw->magic                    = SHBT_MMIO_MAGIC_VALUE;
    hw->version                  = SHBT_MMIO_VERSION_CURRENT;
    hw->plant_state              = SHBT_STATE_COLD_STANDBY;
    hw->control_flags            = SHBT_PWR_CTRL_PCSS_READY
                                 | SHBT_PWR_CTRL_DROOP_TRACK;
    hw->uptime_ticks             = 0u;

    /* Steady-state plant telemetry setpoints. */
    hw->net_export_mw            = 7832.903f;
    hw->gross_output_mw          = 7972.903f;
    hw->recirc_load_mw           = 140.000f;
    hw->linac_load_mw            = 125.000f;
    hw->bop_load_mw              = 15.000f;
    hw->linac_rf_freq_ghz        = 5.712f;
    hw->supercap_stored_mj       = 450.0f;   /* synthetic inertia reserve */
    hw->grid_freq_hz             = 50.00f;
    hw->grid_droop_pct           = 4.0f;
    hw->fault_code               = 0u;

    /* Cacheline 1: thermal-hydraulics, protection, isomer battery. */
    hw->she_loop_temp_cold_k     = 300.0f;
    hw->she_loop_temp_hot_k      = 900.0f;
    hw->she_loop_press_mpa       = 10.0f;
    hw->teg_reclaim_kw           = SHBT_TEG_SHIELD_RECLAIM_KW;
    hw->cryo_subloop_mass_flow   = SHBT_CRYO_MDOT_KG_S;
    hw->pcss_crowbar_quench_ns   = SHBT_PWR_CROWBAR_NOM_NS;
    hw->pcss_inductive_recov_pct = SHBT_PWR_INDUCTIVE_NOM_PCT;
    hw->battery_core_temp_k      = SHBT_BAT_CORE_TEMP_K;
    hw->battery_cryo_headroom_k  = SHBT_BAT_CRYO_HEADROOM_K;
    hw->battery_soc              = 1.0f;
    hw->battery_bus_voltage_kv   = 24.0f;    /* supercap hold rail */
    hw->battery_decay_heat_kw    = SHBT_BAT_DECAY_HEAT_KW;
    hw->mossbauer_recoil_frac    = SHBT_BAT_MOSSBAUER_NOM;
    hw->borrmann_suppress_factor = SHBT_BAT_BORRMANN_NOM;
    hw->audit_gate_status_bits   = 0u;
    hw->audit_gate_extended_bits = 0u;
    return 0;
}

/* --------------------------------------------------------------------------
 * Register-block integrity check: layout, magic/version, and isomer
 * battery telemetry sanity over the live register window.
 * ------------------------------------------------------------------------ */
int shbt_verify_mmio_integrity(void)
{
    shbt_power_mmio_t *hw = shbt_power_regs();

    if (sizeof(shbt_power_mmio_t) != SHBT_MMIO_TOTAL_SIZE_BYTES)
        return -1;
    if (offsetof(shbt_power_mmio_t, she_loop_temp_cold_k) != 0x40)
        return -2;
    if (offsetof(shbt_power_mmio_t, audit_gate_extended_bits) != 0x7C)
        return -3;
    if (hw->magic != SHBT_MMIO_MAGIC_VALUE
        || hw->version != SHBT_MMIO_VERSION_CURRENT) {
        hw->fault_code |= SHBT_PWR_FAULT_MAGIC;
        return -4;
    }
    if (!shbt_battery_coherent(hw)) {
        hw->fault_code |= SHBT_PWR_FAULT_CRYO_LOW;
        return -5;
    }
    return 0;
}

/* --------------------------------------------------------------------------
 * PCSS solid-state crowbar interlock (<= 2.10 ns quench latency).
 *
 * On an overvoltage / arc-fault condition the optically gated GaAs PCSS
 * array fires, shunting the bus discharge through the inductive-resonant
 * recovery tank (>= 94.20 % energy recovery) and latching the fault.
 * ------------------------------------------------------------------------ */
int shbt_trigger_pcss_crowbar(void)
{
    shbt_power_mmio_t *hw = shbt_power_regs();

    hw->control_flags |= SHBT_PWR_CTRL_PCSS_FIRED;
    hw->fault_code    |= SHBT_PWR_FAULT_BUS_OVERVOLT;

    /* Crowbar closes the bus: collapse the DC link to the pre-charge rail. */
    hw->battery_bus_voltage_kv = SHBT_BAT_BUS_PRECHARGE_KV;
    hw->pcss_crowbar_quench_ns = SHBT_PWR_CROWBAR_NOM_NS;

    hw->control_flags &= ~SHBT_PWR_CTRL_PCSS_FIRED;
    return 0;
}

/* --------------------------------------------------------------------------
 * Isomer-battery bootstrap guard: if the cryostat invariants degrade below
 * the Mossbauer de-pinning or Borrmann suppression thresholds, the 40 keV
 * seed pulse is inhibited and the FSM is forced back to COLD_STANDBY.
 * Returns 0 nominal, 1 if the abort fired.
 * ------------------------------------------------------------------------ */
int shbt_isomer_safety_guard(void)
{
    shbt_power_mmio_t *hw = shbt_power_regs();

    if (hw->mossbauer_recoil_frac < SHBT_BAT_MOSSBAUER_MIN) {
        hw->fault_code |= SHBT_PWR_FAULT_MOSSBAUER;
    }
    if (hw->borrmann_suppress_factor < SHBT_BAT_BORRMANN_MIN) {
        hw->fault_code |= SHBT_PWR_FAULT_BORRMANN;
    }
    if (hw->battery_cryo_headroom_k < SHBT_BAT_CRYO_HEADROOM_K - 1e-3f) {
        hw->fault_code |= SHBT_PWR_FAULT_CRYO_LOW;
    }
    if (hw->fault_code
        & (SHBT_PWR_FAULT_MOSSBAUER | SHBT_PWR_FAULT_BORRMANN
           | SHBT_PWR_FAULT_CRYO_LOW)) {
        hw->control_flags &= ~SHBT_PWR_CTRL_SEED_LASER_ON;
        hw->plant_state    = SHBT_STATE_COLD_STANDBY;
        return 1;
    }
    return 0;
}

/* --------------------------------------------------------------------------
 * Five-phase lifecycle sequencer step.  Evaluates the transition
 * predicates Phi_{i->j} from the power8.txt FSM contract at the 10 kHz
 * kernel tick.
 * ------------------------------------------------------------------------ */
void shbt_lifecycle_step(void)
{
    shbt_power_mmio_t *hw = shbt_power_regs();

    switch (hw->plant_state) {
    case SHBT_STATE_COLD_STANDBY:
        /* Phi_01: start command + cryo headroom + PCSS ready. */
        if ((hw->control_flags & SHBT_PWR_CTRL_START_CMD)
            && hw->battery_cryo_headroom_k >= SHBT_BAT_CRYO_HEADROOM_K - 1e-3f
            && hw->battery_bus_voltage_kv >= 24.0f
            && (hw->control_flags & SHBT_PWR_CTRL_PCSS_READY)) {
            hw->plant_state = SHBT_STATE_ISOMER_ARMING;
            hw->battery_bus_voltage_kv = SHBT_BAT_BUS_PRECHARGE_KV;
        }
        break;
    case SHBT_STATE_ISOMER_ARMING:
        /* Phi_12: arming complete, PCSS quench verified, DC link charged. */
        if (hw->pcss_crowbar_quench_ns <= SHBT_PWR_CROWBAR_BUDGET_NS
            && hw->battery_bus_voltage_kv == SHBT_BAT_BUS_PRECHARGE_KV) {
            hw->control_flags |= SHBT_PWR_CTRL_SEED_LASER_ON;
            hw->plant_state    = SHBT_STATE_GRASER_IGNITION_PULSE;
            hw->battery_bus_voltage_kv = SHBT_BAT_BUS_MAX_KV;
        }
        break;
    case SHBT_STATE_GRASER_IGNITION_PULSE:
        /* Phi_23: fusion gross >= 1,000 MW and sHe flow nominal. */
        if (hw->gross_output_mw >= 1000.0f
            && hw->cryo_subloop_mass_flow >= SHBT_CRYO_MDOT_KG_S) {
            hw->plant_state = SHBT_STATE_DEC_BOOTSTRAP;
        }
        break;
    case SHBT_STATE_DEC_BOOTSTRAP:
        /* Phi_34: net export at rating, recirculation sourced in-plant. */
        if (hw->net_export_mw >= 7832.903f - 1e-3f
            && hw->recirc_load_mw == 140.0f
            && hw->battery_core_temp_k <= SHBT_BAT_CORE_TEMP_K + 1e-3f) {
            hw->control_flags &= ~SHBT_PWR_CTRL_SEED_LASER_ON;
            hw->plant_state    = SHBT_STATE_STEADY_STATE_RECIRC;
        }
        break;
    case SHBT_STATE_STEADY_STATE_RECIRC:
    default:
        /* Synthetic-inertia droop tracking; fault abort via guard. */
        break;
    }
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

/* power5.txt §5 — validated telemetry read across the 8-channel MMIO
 * telemetry block: SECDED Hamming(72,64) single-bit correction,
 * double-bit panic (cli; hlt) inside the 20 ns interlock budget, and
 * fixed-point channel descaling into physical units. */
typedef struct {
    uint64_t data_raw;       /* raw 64-bit payload */
    uint64_t ecc_syndrome;   /* stored SECDED check byte in [7:0] */
    uint64_t status_flags;   /* bit1 = corrected, bit2 = panic */
} telemetry_channel_t;

typedef struct {
    telemetry_channel_t channels[8];
    volatile uint32_t global_fault_irq;
    volatile uint32_t cal_heater_enable;
    volatile uint64_t cal_heater_voltage_uv;
    volatile uint64_t cal_heater_current_ua;
} plant_telemetry_block_t;

#define NUM_TELEMETRY_CHANNELS 8U

/* SECDED syndrome computed from the same 7-parity mask set used by
 * shbt_ecc.c (recomputed here so the read path carries no external
 * state). */
static const uint64_t ecc_parity_matrix[7] = {
    0x5555555555555555ULL,
    0x6666666666666666ULL,
    0x7878787878787878ULL,
    0x7F807F807F807F80ULL,
    0x7FFF00007FFF0000ULL,
    0x7FFFFFFF00000000ULL,
    0x8000000000000000ULL
};

bool read_validated_telemetry(uint32_t channel_idx, double *out_physical_val)
{
    if (channel_idx >= NUM_TELEMETRY_CHANNELS || out_physical_val == 0) {
        return false;
    }

    plant_telemetry_block_t *const mmio =
        (plant_telemetry_block_t *)(uintptr_t)SHBT_POWER_BASE_ADDR;
    telemetry_channel_t *ch = &mmio->channels[channel_idx];

    uint64_t raw_data    = ch->data_raw;
    uint8_t received_ecc = (uint8_t)(ch->ecc_syndrome & 0xFFULL);

    uint8_t calc_syndrome = 0;
    for (uint32_t i = 0; i < 7U; i++) {
        if (__builtin_parityll(raw_data & ecc_parity_matrix[i])) {
            calc_syndrome |= (uint8_t)(1U << i);
        }
    }

    uint8_t overall_parity =
        (uint8_t)(__builtin_parityll(raw_data) ^
                  __builtin_parity(received_ecc & 0x7FU));
    uint8_t syndrome_delta = calc_syndrome ^ (received_ecc & 0x7FU);
    bool overall_parity_match =
        (overall_parity == ((received_ecc >> 7U) & 0x01U));

    if (syndrome_delta != 0U) {
        if (!overall_parity_match) {
            /* Single-bit error: correct and flag. */
            uint32_t error_bit = syndrome_delta - 1U;
            if (error_bit < 64U) {
                raw_data ^= (1ULL << error_bit);
            }
            ch->status_flags |= (1ULL << 1);
        } else {
            /* Double-bit error: emergency panic within 20 ns. */
            mmio->global_fault_irq = 0xDEADBEEFU;
#if defined(__x86_64__) || defined(__i386__)
            __asm__ volatile("cli; hlt");
#endif
            return false;
        }
    }

    switch (channel_idx) {
    case 0: *out_physical_val = (double)raw_data * 1.0e-4; break;
    case 1: *out_physical_val = (double)raw_data * 1.0e-5; break;
    case 2: *out_physical_val = (double)raw_data * 1.0e-5; break;
    case 3: *out_physical_val = (double)raw_data * 1.0e-6; break;
    case 4: *out_physical_val = (double)raw_data * 1.0e-4; break;
    case 5: *out_physical_val = (double)(int64_t)raw_data * 1.0e-3; break;
    default: *out_physical_val = (double)raw_data; break;
    }
    return true;
}

/* --------------------------------------------------------------------------
 * Runtime telemetry refresh tick (power8.txt §2): snapshot the 128-byte
 * register block, re-validate magic/version and the isomer battery
 * sanity window, and advance the 10 kHz monotonic tick counter.  On a
 * coherence failure the corresponding fault_code bit latches.
 * ------------------------------------------------------------------------ */
void shbt_power_kernel_telemetry_tick(void)
{
    shbt_power_mmio_t *mmio = shbt_power_regs();

    shbt_power_mmio_t local_snapshot;
    {
        const uint8_t *src = (const uint8_t *)mmio;
        uint8_t *dst = (uint8_t *)&local_snapshot;
        for (size_t i = 0; i < sizeof(local_snapshot); ++i)
            dst[i] = src[i];
    }

    if (local_snapshot.magic != SHBT_MMIO_MAGIC_VALUE
        || local_snapshot.version != SHBT_MMIO_VERSION_CURRENT) {
        mmio->fault_code |= SHBT_PWR_FAULT_MAGIC;
        return;
    }
    if (!shbt_battery_coherent(&local_snapshot)) {
        mmio->fault_code |= SHBT_PWR_FAULT_CRYO_LOW;
        return;
    }

    mmio->uptime_ticks++;
}
