//! Compiles the freestanding C11 shbt-power microkernel sources (MMIO
//! service, PCSS crowbar, SECDED ECC) into a static library for the Rust
//! FFI surface.  The kernel Makefile produces the hosted dynamic library
//! `bin/shbt_power_reference.so` separately for the Python layer.

use std::path::PathBuf;

fn main() {
    let kernel_dir = PathBuf::from("../../kernel");
    let include = kernel_dir.join("include");

    let mut build = cc::Build::new();
    for src in [
        kernel_dir.join("src/shbt_power_kernel.c"),
        kernel_dir.join("src/shbt_ecc.c"),
    ] {
        println!("cargo:rerun-if-changed={}", src.display());
        build.file(&src);
    }
    for hdr in ["shbt_power_mmio.h", "shbt_ecc.h"] {
        println!("cargo:rerun-if-changed={}", include.join(hdr).display());
    }

    build
        .include(&include)
        .flag("-O3")
        .flag("-ffreestanding")
        .warnings(false);
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("x86_64")
    {
        build.flag("-mavx512f");
    }
    build.compile("shbt_power_kernel");
}
