#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

/*
 * System call demonstration using Linux syscalls.
 *
 * On RISC-V:
 *   - a7 = syscall number
 *   - a0-a6 = arguments
 *   - a0 = return value
 *   - ecall triggers the trap into kernel mode
 *
 * On x86_64:
 *   - rax = syscall number
 *   - rdi, rsi, rdx, r10, r8, r9 = arguments
 *   - rax = return value
 *   - syscall triggers the trap into kernel mode
 *
 * We use glibc wrappers (write, read, exit) which internally
 * perform the syscall. The inline assembly versions are shown
 * in the comments.
 */

/* ---------- Inline Syscall Wrappers (RISC-V style) ---------- */

/*
 * RISC-V inline assembly for write:
 *
 *   li a7, 64        # sys_write
 *   mv a0, fd        # arg1: file descriptor
 *   mv a1, buf       # arg2: buffer pointer
 *   mv a2, len       # arg3: length
 *   ecall            # trap to kernel
 *
 * The kernel's trap handler:
 *   1. sstatus -> saved, cpu switches to S-mode
 *   2. scause  -> tells us it's an ecall from U-mode
 *   3. a7      -> 64, so dispatch to sys_write
 *   4. kernel copies buf from user memory to device
 *   5. a0      -> return bytes written (or error)
 *   6. sret    -> back to U-mode, resume at ecall+4
 */

/* Use glibc write for portability, but we trace the path */
void syscall_write(int fd, const char *buf, int len) {
    /* Equivalent RISC-V assembly:
       mv a0, fd;  mv a1, buf;  mv a2, len
       li a7, 64   -- sys_write
       ecall       -- trap to kernel
       Constraints: "r"(fd), "r"(buf), "r"(len)
       Clobbers: a0, a1, a2, a7, memory */
    write(fd, buf, len);
}

/* Exit the process — never returns */
void syscall_exit(int code) {
    /* Equivalent RISC-V assembly:
       mv a0, code
       li a7, 93   -- sys_exit
       ecall       -- trap to kernel (never returns)
       Constraints: "r"(code)
       Clobbers: a0, a7 */
    _exit(code);
    __builtin_unreachable();
}

/* ---------- Trap Handler Skeleton (Kernel-Side) ---------- */

/*
 * This is a simplified trap handler showing the context save/restore
 * pattern. On real hardware this runs in S-mode (supervisor) and
 * is installed by setting the stvec CSR.
 *
 * A "trap frame" is a struct holding all register state that must
 * be preserved across the trap.
 */

typedef struct {
    /* General-purpose registers */
    unsigned long ra, sp, gp, tp;
    unsigned long t0, t1, t2;
    unsigned long s0, s1;
    unsigned long a0, a1, a2, a3, a4, a5, a6, a7;
    unsigned long s2, s3, s4, s5, s6, s7, s8, s9, s10, s11;
    unsigned long t3, t4, t5, t6;

    /* Trap metadata */
    unsigned long epc;      /* PC to resume at */
    unsigned long status;   /* sstatus */
    unsigned long cause;    /* scause */
    unsigned long tval;     /* stval */
} trap_frame_t;

/* Kernel-side syscall counter for our custom syscall */
static int syscall_call_count = 0;

/* Syscall dispatch table */
typedef long (*syscall_fn_t)(long a0, long a1, long a2);

static long do_sys_write(long fd, long buf, long len) {
    /* In real kernel: copy_from_user, write to device */
    printf("  [kernel] sys_write(fd=%ld, buf=%p, len=%ld)\n", fd, (void *)buf, len);
    return len;
}

static long do_sys_exit(long code, long unused1, long unused2) {
    (void)unused1;
    (void)unused2;
    printf("  [kernel] sys_exit(code=%ld)\n", code);
    return 0;
}

static long do_sys_counter(long unused0, long unused1, long unused2) {
    (void)unused0;
    (void)unused1;
    (void)unused2;
    syscall_call_count++;
    printf("  [kernel] sys_counter called (total calls: %d)\n", syscall_call_count);
    return syscall_call_count;
}

/* Syscall table — indexed by syscall number */
#define MAX_SYSCALLS 256
static syscall_fn_t syscall_table[MAX_SYSCALLS];

static void init_syscall_table(void) {
    memset(syscall_table, 0, sizeof(syscall_table));
    syscall_table[64] = do_sys_write;  /* sys_write */
    syscall_table[93] = do_sys_exit;   /* sys_exit  */
    syscall_table[255] = do_sys_counter; /* custom syscall */
}

/*
 * Simplified trap handler.
 *
 * In real RISC-V hardware:
 *
 *   trap_handler:
 *       # On entry: sscratch points to trap_frame
 *       csrrw sp, sscratch, sp    # swap sp with trap frame ptr
 *
 *       # Save all registers into trap frame
 *       sd ra,  0*8(sp)
 *       sd gp,  2*8(sp)
 *       sd tp,  3*8(sp)
 *       sd t0,  4*8(sp)
 *       ...
 *       sd t6, 31*8(sp)
 *
 *       # Save trap metadata
 *       csrr t0, sepc
 *       sd   t0, 32*8(sp)
 *       csrr t0, sstatus
 *       sd   t0, 33*8(sp)
 *
 *       # Check cause: is it an ecall?
 *       csrr t0, scause
 *       # If t0 == 8 or 9 (ecall from U-mode or S-mode), handle syscall
 *       # Otherwise, handle exception/interrupt
 *
 *       # Dispatch syscall
 *       ld   a7, 17*8(sp)      # syscall number from saved a7
 *       call syscall_dispatch  # call the C handler
 *
 *       # Store return value
 *       sd   a0, 10*8(sp)      # saved a0 = return value
 *
 *       # Advance epc past the ecall instruction (4 bytes)
 *       ld   t0, 32*8(sp)
 *       addi t0, t0, 4
 *       csrw sepc, t0
 *
 *       # Restore all registers
 *       ld ra,  0*8(sp)
 *       ...
 *       ld t6, 31*8(sp)
 *
 *       csrrw sp, sscratch, sp  # swap back
 *       sret                    # return to user mode
 */
static long trap_handler_simulated(trap_frame_t *frame) {
    unsigned long syscall_num = frame->a7;
    unsigned long arg0 = frame->a0;
    unsigned long arg1 = frame->a1;
    unsigned long arg2 = frame->a2;

    printf("  [kernel] trap_handler: syscall %ld called\n", syscall_num);

    if (syscall_num < MAX_SYSCALLS && syscall_table[syscall_num]) {
        return syscall_table[syscall_num](arg0, arg1, arg2);
    }

    printf("  [kernel] unknown syscall %ld\n", syscall_num);
    return -1;
}

/* ---------- Demo Programs ---------- */

static void demo_syscall_path(void) {
    printf("=== Syscall Path Demo ===\n\n");

    printf("User-space calls write(1, \"hello\\n\", 6):\n");
    printf("  [user]   load a7=64 (sys_write)\n");
    printf("  [user]   load a0=1, a1=buf, a2=6\n");
    printf("  [user]   execute ecall\n");

    /* Simulate the trap */
    trap_frame_t frame;
    memset(&frame, 0, sizeof(frame));
    frame.a7 = 64;          /* syscall number */
    frame.a0 = 1;           /* fd = stdout */
    frame.a1 = 0;           /* buf pointer (simulated) */
    frame.a2 = 6;           /* length */
    frame.epc = 0x1000;     /* simulated PC */

    long result = trap_handler_simulated(&frame);

    printf("  [user]   a0 = %ld (return value)\n", result);
    printf("  [user]   execution resumes at ecall+4\n\n");
}

static void demo_custom_syscall(void) {
    printf("=== Custom Syscall (counter) Demo ===\n\n");

    for (int i = 0; i < 3; i++) {
        trap_frame_t frame;
        memset(&frame, 0, sizeof(frame));
        frame.a7 = 255; /* custom syscall number */

        printf("User call %d:\n", i + 1);
        long result = trap_handler_simulated(&frame);
        printf("  [user]   returned: %ld\n\n", result);
    }
}

static void demo_privilege_violation(void) {
    printf("=== Privilege Violation Demo ===\n\n");

    printf("What happens if user code tries to:\n");
    printf("  - Read a CSR (csrr t0, sstatus)?  -> Illegal instruction exception\n");
    printf("  - Access unmapped memory?          -> Page fault exception\n");
    printf("  - Execute a privileged instruction? -> Illegal instruction exception\n\n");

    printf("In each case, the CPU:\n");
    printf("  1. Saves PC in sepc\n");
    printf("  2. Sets scause to the exception code\n");
    printf("  3. Sets stval to extra info (faulting address)\n");
    printf("  4. Jumps to stvec (trap handler)\n");
    printf("  5. Kernel decides: fix it, kill process, or deliver signal\n\n");
}

int main(void) {
    init_syscall_table();

    printf("Privilege Modes, Traps, and System Calls\n");
    printf("=========================================\n\n");

    demo_syscall_path();
    demo_custom_syscall();
    demo_privilege_violation();

    printf("=== Real Syscall (write) ===\n\n");
    const char *msg = "This went through a real write() syscall!\n";
    syscall_write(STDOUT_FILENO, msg, strlen(msg));
    printf("\nDone.\n");

    return 0;
}
