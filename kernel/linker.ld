/* linker.ld — contiguous SRAM arena for the shbt-power microkernel.
 * MMIO register arena anchored at SHBT_POWER_BASE_ADDR = 0x70000000
 * (power.txt §6); the kernel image loads low and the 128-byte register
 * block is owned by the device mapping.
 */

ENTRY(_start)

MEMORY
{
    SRAM (rxw)  : ORIGIN = 0x00010000, LENGTH = 512K
    MMIO (rw)   : ORIGIN = 0x70000000, LENGTH = 128
}

SECTIONS
{
    .text :
    {
        *(.text.start)
        *(.text*)
    } > SRAM

    .rodata :
    {
        *(.rodata*)
    } > SRAM

    .data :
    {
        *(.data*)
    } > SRAM

    .bss :
    {
        *(.bss*)
        *(COMMON)
    } > SRAM

    .mmio_regs (NOLOAD) :
    {
        . = ALIGN(64);
        *(.mmio_regs)
        . = . + 128;
    } > MMIO

    /DISCARD/ :
    {
        *(.comment)
        *(.note*)
        *(.eh_frame*)
    }
}
