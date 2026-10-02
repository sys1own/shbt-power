//! End-to-end closed-loop integration test: drives all subsystems through
//! the 5-phase handover FSM and asserts the master ledger invariants
//! (paper/main.tex §6-§8).

use shbt_power_chamber::ChamberSubsystem;
use shbt_power_core::{PhysicsSubsystem, PlantStateSnapshot};
use shbt_power_dec::DecSubsystem;
use shbt_power_grid::{GridSubsystem, PlantLedger, PlantPhase, TegArray};
use shbt_power_holography::HolographySubsystem;
use shbt_power_linac::LinacSubsystem;
use shbt_power_target::TargetSubsystem;
use shbt_power_telemetry::{finalize_frame, snapshot_to_mmio, MmioDriver};

fn drive_ticks(n: u64) -> (
    PlantStateSnapshot,
    LinacSubsystem,
    ChamberSubsystem,
    DecSubsystem,
    GridSubsystem,
    TargetSubsystem,
    HolographySubsystem,
) {
    let mut state = PlantStateSnapshot::default();
    let mut linac = LinacSubsystem::default();
    let mut chamber = ChamberSubsystem::default();
    let mut dec = DecSubsystem::default();
    let mut grid = GridSubsystem::default();
    let mut target = TargetSubsystem::default();
    let mut holo = HolographySubsystem::default();
    for _ in 0..n {
        state.tick += 1;
        linac.update(&mut state);
        chamber.update(&mut state);
        dec.update(&mut state);
        grid.update(&mut state);
        target.update(&mut state);
        holo.update(&mut state);
    }
    (state, linac, chamber, dec, grid, target, holo)
}

#[test]
fn closed_loop_reaches_steady_state_and_balances() {
    let (state, _l, _c, _d, grid, _t, _h) = drive_ticks(200_000);

    // The FSM must have reached steady-state recirculation.
    assert_eq!(state.phase, PlantPhase::SteadyStateRecirculation as u32);

    // Buffer saturated to 100 % SoC.
    assert!((grid.buffer.soc() - 1.0).abs() < 1e-12);

    // Master ledger closes: gross - recirc = net.
    let ledger = PlantLedger::solve(&TegArray::default());
    assert!((ledger.p_net_mw - (ledger.p_gross_mw - ledger.p_recirc_mw)).abs() < 1e-9);
    assert!((ledger.p_net_mw - 7832.903).abs() < 0.05);
    assert!((state.p_net_mw - 7832.903).abs() < 0.05);
    assert!((state.p_gross_mw - 7972.903).abs() < 0.05);

    // MMIO frame carries magic + battery telemetry + audit bitfields.
    let mut frame = snapshot_to_mmio(&state);
    finalize_frame(&mut frame, 0xFFFF_FFFF, 0xFFFF_FFFF);
    assert_ne!(frame.magic, 0);
    assert_eq!(frame.audit_gate_status_bits, 0xFFFF_FFFF);
    assert!((frame.battery_core_temp_k - 21.13).abs() < 0.01);
}

#[test]
fn kernel_integrity_and_interlocks() {
    assert_eq!(MmioDriver::init(), 0);
    assert_eq!(MmioDriver::verify_integrity(), 0);
    assert_eq!(MmioDriver::trigger_crowbar(), 0);
}

#[test]
fn burst_hierarchy_and_dec_bounds() {
    let dec = DecSubsystem::default();
    assert!(dec.below_cl_bound());
    assert!((dec.trumpet.d_coll_m - 5.0).abs() < 0.01);
}
