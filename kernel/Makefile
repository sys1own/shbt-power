# shbt-power microkernel build rules (SHBT-MMIO-POWER normative specification).
#  - shbt_power_kernel.elf: freestanding image linked through linker.ld with
#    the 128-byte MMIO register arena anchored at 0x70000000.
#  - shbt_power_reference.so: hosted interface library consumed by the
#    shbt-power-telemetry FFI surface and the Python CLI.

CC      ?= gcc
CFLAGS  ?= -O3 -mavx512f -nostdlib -ffreestanding -fno-stack-protector \
           -fPIC -Iinclude -std=c11 -DSHBT_BARE_METAL

BIN     := ../crates/shbt-power-telemetry/bin
OBJS    := build/shbt_power_kernel.o build/shbt_ecc.o
OBJS_SO := build/host_power_kernel.o build/host_ecc.o

all: $(BIN)/shbt_power_reference.so build/shbt_power_kernel.elf

build:
	mkdir -p build $(BIN)

build/shbt_%.o: src/shbt_%.c include/shbt_power_mmio.h include/shbt_ecc.h | build
	$(CC) $(CFLAGS) -c $< -o $@

build/host_%.o: src/shbt_%.c include/shbt_power_mmio.h include/shbt_ecc.h | build
	$(CC) $(CFLAGS) -USHBT_BARE_METAL -c $< -o $@

build/shbt_power_kernel.elf: $(OBJS) linker.ld
	$(CC) $(CFLAGS) -static -Wl,-T,linker.ld $(OBJS) -o $@

$(BIN)/shbt_power_reference.so: $(OBJS_SO)
	$(CC) $(CFLAGS) -nostdlib -shared $(OBJS_SO) -o $@

clean:
	rm -rf build $(BIN)/shbt_power_reference.so

.PHONY: all clean
