//! shbt-power-telemetry — SHBT-POWER-MMIO register driver, zero-copy SPSC
//! shared-memory telemetry ring, and CRC-32/Castagnoli integrity
//! (paper/main.tex §6, paper/supplementary.tex §12).
//!
//! C-ABI layout and SPSC ring semantics transferred from
//! `sys1own/shbt-recon`; SECDED/MMIO anchoring from `sys1own/shbt-qc`.

pub mod gum;
pub mod metrology;

use shbt_power_core::constants::MMIO_BASE_ADDR;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};

/// 128-byte, dual-cacheline MMIO register block — Rust mirror of
/// `kernel/include/shbt_power_mmio.h` (identical field order).
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, Default)]
pub struct ShbtPowerMmio {
    pub sys_ctrl: u32,
    pub sys_status: u32,
    pub clock_ticks_lo: u32,
    pub clock_ticks_hi: u32,
    pub pcss_gate_ctrl: u32,
    pub crowbar_status: u32,
    pub adm_metric_err: f32,
    pub adm_shift_norm: f32,
    pub precision_flags: u32,
    pub quench_headroom: f32,
    pub beam_energy_gev: f32,
    pub beam_focus_tune: f32,
    pub holo_entropy_gap: f32,
    pub dark_ledger_par: u32,
    pub recon_dma_stat: u32,
    pub _pad_align_cl0: u32,
    pub dec_grid1_volt: f32,
    pub dec_grid2_volt: f32,
    pub dec_grid3_volt: f32,
    pub dec_alpha_curr_ka: f32,
    pub mhd_pickup_curr_ka: f32,
    pub wbg_panel_temp_k: f32,
    pub supercap_soc_pct: f32,
    pub lanr_array_net_kw: f32,
    pub teg_reclaim_mw: f32,
    pub divertor_temp_k: f32,
    pub target_pos_dev_um: f32,
    pub target_sync_word: u32,
    pub interlock_latch: u32,
    pub fault_injection_k: u32,
    pub reserved_ext: u32,
    pub telemetry_crc32: u32,
}

const _: () = assert!(core::mem::size_of::<ShbtPowerMmio>() == 128);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, dec_grid1_volt) == 0x40);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, telemetry_crc32) == 0x7C);

extern "C" {
    fn shbt_power_kernel_init() -> i32;
    fn shbt_verify_mmio_integrity() -> i32;
    fn shbt_trigger_pcss_crowbar() -> i32;
    fn shbt_compute_crc32_castagnoli(buf: *const u8, len: usize) -> u32;
    fn shbt_ecc_encode(data: u64) -> u8;
    fn shbt_ecc_decode_data(data: u64, check_code: u8, flags: *mut u8) -> u64;
    fn shbt_power_simd_shunt_check(currents_ka: *const f32) -> i32;
}

/// MMIO register driver façade over the C11 kernel.
pub struct MmioDriver;

impl MmioDriver {
    pub fn init() -> i32 {
        unsafe { shbt_power_kernel_init() }
    }
    pub fn verify_integrity() -> i32 {
        unsafe { shbt_verify_mmio_integrity() }
    }
    pub fn trigger_crowbar() -> i32 {
        unsafe { shbt_trigger_pcss_crowbar() }
    }
    /// CRC-32/Castagnoli over an arbitrary byte slice (kernel-side impl).
    pub fn crc32c(buf: &[u8]) -> u32 {
        unsafe { shbt_compute_crc32_castagnoli(buf.as_ptr(), buf.len()) }
    }
    /// SECDED Hamming(72,64) roundtrip: encodes `data`, injects `flip_bit`
    /// (None = clean), decodes, and returns (corrected, flags).
    pub fn ecc_roundtrip(data: u64, flip_bit: Option<u8>) -> (u64, u8) {
        unsafe {
            let code = shbt_ecc_encode(data);
            let corrupted = match flip_bit {
                Some(b) if b < 64 => data ^ (1u64 << b),
                _ => data,
            };
            let mut flags = 0u8;
            let out = shbt_ecc_decode_data(corrupted, code, &mut flags);
            (out, flags)
        }
    }
    pub fn simd_shunt_check(currents: &[f32; 16]) -> i32 {
        unsafe { shbt_power_simd_shunt_check(currents.as_ptr()) }
    }
}

/// Rust-side CRC-32/Castagnoli (reflected poly 0x82F63B78) for cross-check.
pub fn crc32c_rust(buf: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in buf {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0x82F6_3B78 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

/// Zero-copy SPSC telemetry ring over a caller-provided arena (POSIX
/// shared memory in HIL deployments, a plain heap arena in tests).
/// Capacity is in whole 128-byte `ShbtPowerMmio` frames.
#[repr(C, align(64))]
pub struct SpscRingHeader {
    pub write_idx: u64,
    pub read_idx: u64,
    pub capacity: u64,
    pub dropped: u64,
}

pub struct SpscRing<'a> {
    pub header: &'a mut SpscRingHeader,
    pub frames: &'a mut [ShbtPowerMmio],
}

impl<'a> SpscRing<'a> {
    pub fn new(header: &'a mut SpscRingHeader, frames: &'a mut [ShbtPowerMmio]) -> Self {
        header.capacity = frames.len() as u64;
        Self { header, frames }
    }
    pub fn push(&mut self, frame: &ShbtPowerMmio) -> bool {
        let next = self.header.write_idx.wrapping_add(1) % self.header.capacity;
        if next == self.header.read_idx % self.header.capacity
            && self.header.write_idx != self.header.read_idx
        {
            self.header.dropped += 1;
            return false;
        }
        let idx = (self.header.write_idx % self.header.capacity) as usize;
        self.frames[idx] = *frame;
        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
        self.header.write_idx += 1;
        true
    }
    pub fn pop(&mut self) -> Option<ShbtPowerMmio> {
        if self.header.read_idx == self.header.write_idx {
            return None;
        }
        let idx = (self.header.read_idx % self.header.capacity) as usize;
        let f = self.frames[idx];
        core::sync::atomic::fence(core::sync::atomic::Ordering::Acquire);
        self.header.read_idx += 1;
        Some(f)
    }
}

/// Publish the live `PlantStateSnapshot` into a register frame.
pub fn snapshot_to_mmio(state: &PlantStateSnapshot) -> ShbtPowerMmio {
    ShbtPowerMmio {
        sys_ctrl: 1,
        sys_status: (1 << 0) | (1 << 1) | (1 << 5),
        clock_ticks_lo: state.tick as u32,
        clock_ticks_hi: (state.tick >> 32) as u32,
        pcss_gate_ctrl: 0,
        crowbar_status: 0,
        adm_metric_err: state.adm_metric_err as f32,
        adm_shift_norm: state.adm_shift_norm as f32,
        precision_flags: 1,
        quench_headroom: state.quench_headroom_k as f32,
        beam_energy_gev: 0.5,
        beam_focus_tune: 0.0,
        holo_entropy_gap: state.holo_entropy_gap as f32,
        dark_ledger_par: 0x23,
        recon_dma_stat: 1,
        _pad_align_cl0: 0,
        dec_grid1_volt: 0.8,
        dec_grid2_volt: 1.8,
        dec_grid3_volt: 2.7,
        dec_alpha_curr_ka: (state.p_direct_total_mw * 1e3 / 2.7e6) as f32,
        mhd_pickup_curr_ka: 120.0,
        wbg_panel_temp_k: 1146.0,
        supercap_soc_pct: (state.supercap_soc * 100.0) as f32,
        lanr_array_net_kw: state.lanr_net_kw as f32,
        teg_reclaim_mw: state.p_teg_mw as f32,
        divertor_temp_k: 450.0,
        target_pos_dev_um: state.target_jitter_um as f32,
        target_sync_word: 0xAA55,
        interlock_latch: state.interlock_latch,
        fault_injection_k: 0,
        reserved_ext: 0,
        telemetry_crc32: 0, // stamped by finalize_crc
    }
}

/// Stamp the CRC over bytes 0x00..0x7B of a frame.
pub fn finalize_crc(frame: &mut ShbtPowerMmio) {
    let bytes =
        unsafe { core::slice::from_raw_parts(frame as *const _ as *const u8, 124) };
    frame.telemetry_crc32 = MmioDriver::crc32c(bytes);
}

#[derive(Debug, Default)]
pub struct TelemetrySubsystem;

impl PhysicsSubsystem for TelemetrySubsystem {
    fn name(&self) -> &'static str {
        "shbt-power-telemetry"
    }
    fn update(&mut self, state: &mut PlantStateSnapshot) {
        let mut frame = snapshot_to_mmio(state);
        finalize_crc(&mut frame);
        debug_assert_eq!(crc32c_rust(unsafe {
            core::slice::from_raw_parts(&frame as *const _ as *const u8, 124)
        }), frame.telemetry_crc32);
        debug_assert_eq!(MMIO_BASE_ADDR, 0x7000_0000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmio_layout() {
        assert_eq!(core::mem::size_of::<ShbtPowerMmio>(), 128);
        assert_eq!(core::mem::offset_of!(ShbtPowerMmio, dec_grid1_volt), 0x40);
    }

    #[test]
    fn kernel_init_and_integrity() {
        assert_eq!(MmioDriver::init(), 0);
        assert_eq!(MmioDriver::verify_integrity(), 0);
    }

    #[test]
    fn crc_matches_rust() {
        let buf = b"shbt-power";
        assert_eq!(MmioDriver::crc32c(buf), crc32c_rust(buf));
    }

    #[test]
    fn ecc_single_bit_correction() {
        let (out, flags) = MmioDriver::ecc_roundtrip(0xDEAD_BEEF_CAFE_F00D, Some(17));
        assert_eq!(out, 0xDEAD_BEEF_CAFE_F00D);
        assert_eq!(flags & 1, 1);
    }

    #[test]
    fn crowbar_and_ring() {
        assert_eq!(MmioDriver::trigger_crowbar(), 0);
        let mut header = SpscRingHeader {
            write_idx: 0,
            read_idx: 0,
            capacity: 0,
            dropped: 0,
        };
        let mut storage = [ShbtPowerMmio::default(); 4];
        let mut ring = SpscRing::new(&mut header, &mut storage);
        let mut frame = ShbtPowerMmio::default();
        finalize_crc(&mut frame);
        assert!(ring.push(&frame));
        assert_eq!(ring.pop().unwrap().telemetry_crc32, frame.telemetry_crc32);
    }
}
