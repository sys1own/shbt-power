//! shbt-power-telemetry — SHBT-POWER-MMIO register driver, zero-copy SPSC
//! shared-memory telemetry ring, and CRC-32/Castagnoli integrity
//! (paper/main.tex §6, paper/supplementary.tex §12).
//!
//! C-ABI layout and SPSC ring semantics transferred from
//! `sys1own/shbt-recon`; SECDED/MMIO anchoring from `sys1own/shbt-qc`.

pub mod calibration;
pub mod gum;
pub mod metrology;

use shbt_power_core::constants::*;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};

/// 128-byte, dual-cacheline MMIO register block — Rust mirror of
/// `kernel/include/shbt_power_mmio.h` (identical field order).
/// Cacheline 0: plant control, grid state, output telemetry.
/// Cacheline 1: thermal-hydraulics, protection, isomer battery.
#[repr(C, align(64))]
#[derive(Clone, Copy, Debug, Default)]
pub struct ShbtPowerMmio {
    pub magic: u32,                      // 0x00
    pub version: u32,                    // 0x04
    pub plant_state: u32,                // 0x08
    pub control_flags: u32,              // 0x0C
    pub uptime_ticks: u64,               // 0x10
    pub net_export_mw: f32,              // 0x18
    pub gross_output_mw: f32,            // 0x1C
    pub recirc_load_mw: f32,             // 0x20
    pub linac_load_mw: f32,              // 0x24
    pub bop_load_mw: f32,                // 0x28
    pub linac_rf_freq_ghz: f32,          // 0x2C
    pub supercap_stored_mj: f32,         // 0x30
    pub grid_freq_hz: f32,               // 0x34
    pub grid_droop_pct: f32,             // 0x38
    pub fault_code: u32,                 // 0x3C
    pub she_loop_temp_cold_k: f32,       // 0x40
    pub she_loop_temp_hot_k: f32,        // 0x44
    pub she_loop_press_mpa: f32,         // 0x48
    pub teg_reclaim_kw: f32,             // 0x4C
    pub cryo_subloop_mass_flow: f32,     // 0x50
    pub pcss_crowbar_quench_ns: f32,     // 0x54
    pub pcss_inductive_recov_pct: f32,   // 0x58
    pub battery_core_temp_k: f32,        // 0x5C
    pub battery_cryo_headroom_k: f32,    // 0x60
    pub battery_soc: f32,                // 0x64
    pub battery_bus_voltage_kv: f32,     // 0x68
    pub battery_decay_heat_kw: f32,      // 0x6C
    pub mossbauer_recoil_frac: f32,      // 0x70
    pub borrmann_suppress_factor: f32,   // 0x74
    pub audit_gate_status_bits: u32,     // 0x78
    pub audit_gate_extended_bits: u32,   // 0x7C
}

const _: () = assert!(core::mem::size_of::<ShbtPowerMmio>() == 128);
const _: () = assert!(core::mem::align_of::<ShbtPowerMmio>() == 64);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, magic) == 0x00);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, plant_state) == 0x08);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, net_export_mw) == 0x18);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, fault_code) == 0x3C);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, she_loop_temp_cold_k) == 0x40);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, battery_core_temp_k) == 0x5C);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, battery_cryo_headroom_k) == 0x60);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, battery_soc) == 0x64);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, battery_bus_voltage_kv) == 0x68);
const _: () = assert!(core::mem::offset_of!(ShbtPowerMmio, audit_gate_extended_bits) == 0x7C);

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
        magic: MMIO_MAGIC,
        version: MMIO_VERSION,
        plant_state: state.phase,
        control_flags: (1 << 1) | (1 << 4), // PCSS_READY | DROOP_TRACK
        uptime_ticks: state.tick,
        net_export_mw: state.p_net_mw as f32,
        gross_output_mw: state.p_gross_mw as f32,
        recirc_load_mw: state.p_recirc_mw as f32,
        linac_load_mw: P_GRASER_ELEC_MW as f32,
        bop_load_mw: P_AUX_MW as f32,
        linac_rf_freq_ghz: F_RF_GHZ as f32,
        supercap_stored_mj: (state.supercap_soc * SUPERCAP_MJ) as f32,
        grid_freq_hz: GRID_FREQ_HZ as f32,
        grid_droop_pct: DROOP_PCT as f32,
        fault_code: 0,
        she_loop_temp_cold_k: TEG_T_COLD_K as f32,
        she_loop_temp_hot_k: TEG_T_HOT_K as f32,
        she_loop_press_mpa: HE_PRESSURE_MPA as f32,
        teg_reclaim_kw: TEG_SHIELD_KW as f32,
        cryo_subloop_mass_flow: CRYO_MDOT_KG_S as f32,
        pcss_crowbar_quench_ns: PCSS_QUENCH_NS as f32,
        pcss_inductive_recov_pct: ETA_INDUCTIVE_RECOVERY_PCT as f32,
        battery_core_temp_k: state.battery_core_temp_k as f32,
        battery_cryo_headroom_k: state.battery_cryo_headroom_k as f32,
        battery_soc: state.battery_soc as f32,
        battery_bus_voltage_kv: state.battery_bus_voltage_kv as f32,
        battery_decay_heat_kw: state.battery_decay_heat_kw as f32,
        mossbauer_recoil_frac: state.mossbauer_recoil_frac as f32,
        borrmann_suppress_factor: state.borrmann_suppress_factor as f32,
        audit_gate_status_bits: 0,
        audit_gate_extended_bits: 0,
    }
}

/// Stamp the audit-gate pass/fail bitfields (0x78 / 0x7C) on a frame.
pub fn finalize_frame(frame: &mut ShbtPowerMmio, gate_mask: u32, ext_mask: u32) {
    frame.audit_gate_status_bits = gate_mask;
    frame.audit_gate_extended_bits = ext_mask;
}

#[derive(Debug, Default)]
pub struct TelemetrySubsystem;

impl PhysicsSubsystem for TelemetrySubsystem {
    fn name(&self) -> &'static str {
        "shbt-power-telemetry"
    }
    fn update(&mut self, state: &mut PlantStateSnapshot) {
        let mut frame = snapshot_to_mmio(state);
        finalize_frame(&mut frame, 0, 0);
        debug_assert_eq!(frame.magic, MMIO_MAGIC);
        debug_assert_eq!(MMIO_BASE_ADDR, 0x7000_0000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmio_layout() {
        assert_eq!(core::mem::size_of::<ShbtPowerMmio>(), 128);
        assert_eq!(core::mem::offset_of!(ShbtPowerMmio, she_loop_temp_cold_k), 0x40);
        assert_eq!(core::mem::offset_of!(ShbtPowerMmio, battery_core_temp_k), 0x5C);
        assert_eq!(core::mem::offset_of!(ShbtPowerMmio, audit_gate_extended_bits), 0x7C);
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
        finalize_frame(&mut frame, 0xFFFF_FFFF, 0xFFFF_FFFF);
        assert!(ring.push(&frame));
        assert_eq!(
            ring.pop().unwrap().audit_gate_extended_bits,
            frame.audit_gate_extended_bits
        );
    }
}
