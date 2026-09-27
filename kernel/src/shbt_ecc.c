/*
 * shbt_ecc.c — SECDED Hamming(72,64) encode/decode.
 * Transferred from sys1own/shbt-qc via sys1own/shbt-sglt
 * (kernel/src/shbt_ecc_avx512.c) — identical mask tables and syndrome
 * decode path, bounded inside the 1.20-ns encode budget.
 */

#include <stdint.h>
#include "shbt_ecc.h"

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

#ifndef SHBT_TSC_HZ
#define SHBT_TSC_HZ 3000000000ULL
#endif

static const uint64_t shbt_ecc_mask[7] = {
    UINT64_C(0xab55555556aaad5b),
    UINT64_C(0xcd9999999b33366d),
    UINT64_C(0xf1e1e1e1e3c3c78e),
    UINT64_C(0x01fe01fe03fc07f0),
    UINT64_C(0x001fffe0003fff800),
    UINT64_C(0x001fffffffc000000),
    UINT64_C(0xfe00000000000000),
};

static const int8_t shbt_ecc_data_of_pos[128] = {
     -1,  -1,  -1,   0,  -1,   1,   2,   3,
     -1,   4,   5,   6,   7,   8,   9,  10,
     -1,  11,  12,  13,  14,  15,  16,  17,
     18,  19,  20,  21,  22,  23,  24,  25,
     -1,  26,  27,  28,  29,  30,  31,  32,
     33,  34,  35,  36,  37,  38,  39,  40,
     41,  42,  43,  44,  45,  46,  47,  48,
     49,  50,  51,  52,  53,  54,  55,  56,
     -1,  57,  58,  59,  60,  61,  62,  63,
     -1,  -1,  -1,  -1,  -1,  -1,  -1,  -1,
     -1,  -1,  -1,  -1,  -1,  -1,  -1,  -1,
     -1,  -1,  -1,  -1,  -1,  -1,  -1,  -1,
     -1,  -1,  -1,  -1,  -1,  -1,  -1,  -1,
};

static inline uint8_t shbt_ecc_hamming_bits(uint64_t data)
{
    uint8_t c = 0;
    for (int j = 0; j < 7; ++j)
        c |= (uint8_t)(__builtin_parityll(data & shbt_ecc_mask[j]) << j);
    return c;
}

uint8_t shbt_ecc_encode(uint64_t data)
{
    uint8_t c = shbt_ecc_hamming_bits(data);
    /* bit7: overall parity for SECDED double-error detection. */
    c |= (uint8_t)((__builtin_parityll(data) ^ __builtin_parity(c & 0x7F)) << 7);
    return c;
}

uint64_t shbt_ecc_decode_data(uint64_t data, uint8_t check_code, uint8_t *flags)
{
    uint8_t expected = shbt_ecc_hamming_bits(data);
    uint8_t syn = (uint8_t)((check_code ^ expected) & 0x7FU);
    uint8_t parity = (uint8_t)(__builtin_parityll(data) ^ __builtin_parity(check_code));

    *flags = 0;

    if (syn == 0)
        return data;                    /* clean or parity-bit-only error */

    if (parity == 0) {
        *flags |= 2U;                   /* even flips: detected, uncorrectable */
        return data;
    }

    if ((syn & (syn - 1)) == 0) {
        *flags |= 1U;                   /* check-bit error: data already clean */
        return data;
    }

    int8_t db = shbt_ecc_data_of_pos[syn];
    if (db < 0) {
        *flags |= 2U;
        return data;
    }

    *flags |= 1U;
    return data ^ (UINT64_C(1) << db);
}

double shbt_ecc_encode_bench(unsigned iters)
{
    volatile uint8_t sink = 0;
    uint64_t data = 0xA5A5C3C39E3779B9ULL;
    uint64_t t0 = shbt_cycles();
    for (unsigned i = 0; i < iters; ++i)
        sink ^= shbt_ecc_encode(data + i);
    (void)sink;
    uint64_t dt = shbt_cycles() - t0;
    return (double)dt * 1.0e9 / ((double)SHBT_TSC_HZ * (double)iters);
}
