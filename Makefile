# shbt-power unified build system (paper/main.tex §8).
#  - kernel/: C11 freestanding microkernel (MMIO @ 0x70000000)
#  - crates/: Rust workspace (multi-physics digital twin + audit engine)
#  - bindings/shbt-power-py: PyO3 C-extension
#  - tests/: workspace integration + Python test runner

.PHONY: all kernel rust audit pyo3 test clean

all: kernel rust

kernel:
	$(MAKE) -C kernel all

rust:
	cargo build --workspace --all-targets

audit: rust
	cargo run --release -p shbt-power-audit

pyo3:
	maturin develop --manifest-path bindings/shbt-power-py/Cargo.toml

test:
	python3 tests/run_all_tests.py

clean:
	$(MAKE) -C kernel clean
	cargo clean
