# shbt-power

Multi-physics digital twin of the SHBT-Graser aneutronic p-11B fusion power
plant (8,750 MW fusion / 7,832.903 MW net export), implementing the
specification in `paper/main.tex` + `paper/supplementary.tex` and the 70-gate verification
matrix.

## Layout

- `kernel/` — C11 freestanding microkernel: exact 128-byte MMIO register
  block at `0x70000000`, SECDED Hamming(72,64) ECC, CRC-32C integrity,
  PCSS crowbar interlock, AVX-512 SIMD shunt check, custom linker script.
- `crates/shbt-power-*` — Rust workspace: `core` (Float512 Q256x256 fixed
  point replacing `rug`, plant state snapshot, canonical branch), `linac`
  (`cavity_dynamics`: C-band envelope ODE, PI+feedforward LLRF, LSC/CSR
  wakefields, 3-regime optical-klystron FEL), `chamber` (MHD fireball
  stopping + `resistive_mhd`: 13.125 MJ channel model, MRT flutes, HTS
  pickup coupled circuits, Bean J_c), `dec` (`sheath`: guiding-center
  expander optics, Child-Langmuir + thermionic neutralization, Sternglass
  SEE/suppressor), `grid` (`helium_network`: Churchill friction +
  compressor duty, `teg_nodal`: T-dependent ZT dual-stage model,
  `interconnect`: swing equation, synthetic-inertia droop, Bode margins,
  5-phase handover), `target` (`kinetics`: 5-level Breit-Wigner ladder,
  knock-on avalanche, Birch-Murnaghan EOS, RTI/RMI growth), `holography`,
  `telemetry` (FFI mirror + SPSC ring), `audit` (GATE-01..70 engine plus
  `extended_checks` for the higher-order solvers).
- `bindings/shbt-power-py/` — PyO3 `PyShbtDigitalTwin` C-extension.
- `python/shbt_power/` — CLI (`run`, `audit`), HUD, 5-regime sweep.
- `tests/` — closed-loop integration test + `run_all_tests.py` master runner.

## Build & validate

```sh
make -C kernel all                                       # C11 kernel + .so
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run --release -p shbt-power-audit                  # -> verification_matrix.json
maturin develop --manifest-path bindings/shbt-power-py/Cargo.toml
python3 tests/run_all_tests.py
```

`verification_matrix.json` reports 70/70 gates PASS plus 27/27
`extended_checks` PASS, and a `discrepancies` array for values where the
computed physics differs from the printed spec (e.g. collector diameter
5.0 m vs 12 m, stopping radius 1.508 m vs 0.863 m, cavity droop 76.7% vs
12.4%, TEG efficiency ~14% vs 33.804%, compressor duty 9.29 MW vs
13.382 MW).
