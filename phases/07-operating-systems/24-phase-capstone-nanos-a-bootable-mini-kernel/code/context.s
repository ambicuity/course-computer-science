/* context.s — RISC-V 64-bit context switch for nanos
 *
 * void context_switch(uint64_t **old_sp, uint64_t *new_sp);
 *
 *   a0 = pointer to old process's sp slot (or NULL for first switch)
 *   a1 = new process's saved sp
 *
 * Saves all callee-saved registers into *old_sp, then restores
 * all callee-saved registers from new_sp and returns.
 */

.section .text
.global context_switch

context_switch:
    /* ---- Save callee-saved registers of outgoing process ---- */
    addi sp, sp, -112          /* 14 registers × 8 bytes */

    sd ra,   0(sp)
    sd s0,   8(sp)
    sd s1,  16(sp)
    sd s2,  24(sp)
    sd s3,  32(sp)
    sd s4,  40(sp)
    sd s5,  48(sp)
    sd s6,  56(sp)
    sd s7,  64(sp)
    sd s8,  72(sp)
    sd s9,  80(sp)
    sd s10, 88(sp)
    sd s11, 96(sp)
    sd gp, 104(sp)

    /* Save current sp into *old_sp (skip if old_sp == NULL) */
    beqz a0, _skip_save
    sd sp, 0(a0)
_skip_save:

    /* ---- Restore callee-saved registers of incoming process ---- */
    mv sp, a1

    ld ra,   0(sp)
    ld s0,   8(sp)
    ld s1,  16(sp)
    ld s2,  24(sp)
    ld s3,  32(sp)
    ld s4,  40(sp)
    ld s5,  48(sp)
    ld s6,  56(sp)
    ld s7,  64(sp)
    ld s8,  72(sp)
    ld s9,  80(sp)
    ld s10, 88(sp)
    ld s11, 96(sp)
    ld gp, 104(sp)

    addi sp, sp, 112

    ret
