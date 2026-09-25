/*
 * shbt_ecc.h — SECDED Hamming(72,64) forward error correction interface.
 * Transferred from sys1own/shbt-qc (kernel/src/ecc_hamming.c) via
 * sys1own/shbt-sglt (kernel/src/shbt_ecc_avx512.c).
 */
#ifndef SHBT_ECC_H
#define SHBT_ECC_H

#include <stdint.h>

/* Encode 64 data bits into the 7-bit Hamming check code (+ overall parity). */
uint8_t  shbt_ecc_encode(uint64_t data);

/* Decode + correct.  *flags: bit0 = single-bit error corrected,
 * bit1 = uncorrectable double-bit error.  Returns corrected data. */
uint64_t shbt_ecc_decode_data(uint64_t data, uint8_t check_code, uint8_t *flags);

/* Mask-parallel encode latency bench (ns/op). */
double shbt_ecc_encode_bench(unsigned iters);

#endif
