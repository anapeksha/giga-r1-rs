.syntax unified
.cpu cortex-m7
.fpu fpv5-d16
.thumb

.global _start
.type _start, %function

.section .isr_vector, "a", %progbits
.word 0x24080000
.word _start + 1

.section .text, "ax", %progbits
/*
 * Apply the option-byte value used by Arduino's GIGA bootloader recipe.
 * This disables autonomous CM4 boot; the M7 application starts CM4 when
 * a valid coprocessor image is present.
 */
_start:
    ldr r0, =0x52002008
    ldr r1, =0x08192a3b
    str r1, [r0]
    ldr r1, =0x4c5d6e7f
    str r1, [r0]
    dsb

    ldr r0, =0x52002020
    ldr r1, =0x0b86aaf0
    str r1, [r0]
    dsb

    ldr r0, =0x52002018
    ldr r1, [r0]
    orr r1, r1, #2
    str r1, [r0]
    dsb

1:
    ldr r0, =0x5200201c
    ldr r1, [r0]
    tst r1, #1
    bne 1b

    bkpt #0
    b .

.size _start, . - _start
