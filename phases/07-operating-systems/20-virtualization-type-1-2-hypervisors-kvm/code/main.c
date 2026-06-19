#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <linux/kvm.h>

/*
 * Minimal KVM guest: create a VM, load guest code, run it.
 * Phase 07 — Operating Systems, Lesson 20
 *
 * The guest writes "Hello from guest!" to I/O port 0x3f8 (serial),
 * then halts. The host intercepts the I/O write and prints it.
 *
 * Compile: gcc -o kvm_demo main.c
 * Run:     ./kvm_demo   (needs /dev/kvm access — may need sudo or kvm group)
 */

#define GUEST_MEM_SIZE 0x1000  /* 4 KB guest memory */

/*
 * Guest code (x86_16 real mode):
 *
 *   mov dx, 0x3f8          ; serial port
 *   mov al, 'H'            ; character to write
 *   out dx, al             ; write to I/O port — causes VM exit
 *   mov al, 'e'
 *   out dx, al
 *   mov al, 'l'
 *   out dx, al
 *   mov al, 'l'
 *   out dx, al
 *   mov al, 'o'
 *   out dx, al
 *   hlt                    ; halt — causes VM exit
 *
 * Assembled bytes:
 */
static const uint8_t guest_code[] = {
    0xba, 0xf8, 0x03,       /* mov dx, 0x3f8         */
    0xb0, 0x48,             /* mov al, 'H'           */
    0xee,                   /* out dx, al            */
    0xb0, 0x65,             /* mov al, 'e'           */
    0xee,                   /* out dx, al            */
    0xb0, 0x6c,             /* mov al, 'l'           */
    0xee,                   /* out dx, al            */
    0xb0, 0x6c,             /* mov al, 'l'           */
    0xee,                   /* out dx, al            */
    0xb0, 0x6f,             /* mov al, 'o'           */
    0xee,                   /* out dx, al            */
    0xf4,                   /* hlt                   */
};

static void die(const char *msg) {
    perror(msg);
    exit(1);
}

int main(int argc, char *argv[]) {
    (void)argc;
    (void)argv;

    int ret;
    struct kvm_run *run;
    struct kvm_sregs sregs;
    struct kvm_regs regs;

    printf("KVM Minimal Guest Demo\n");
    printf("======================\n\n");

    /* Step 1: Open /dev/kvm */
    int kvm_fd = open("/dev/kvm", O_RDWR | O_CLOEXEC);
    if (kvm_fd < 0) {
        die("open /dev/kvm");
    }

    /* Check API version */
    int api_ver = ioctl(kvm_fd, KVM_GET_API_VERSION, 0);
    if (api_ver != KVM_API_VERSION) {
        fprintf(stderr, "KVM API version mismatch: got %d, expected %d\n",
                api_ver, KVM_API_VERSION);
        exit(1);
    }
    printf("[kvm] API version: %d\n", api_ver);

    /* Step 2: Create a VM */
    int vm_fd = ioctl(kvm_fd, KVM_CREATE_VM, 0);
    if (vm_fd < 0) {
        die("KVM_CREATE_VM");
    }
    printf("[kvm] VM created (fd=%d)\n", vm_fd);

    /* Step 3: Allocate guest memory */
    void *guest_mem = mmap(NULL, GUEST_MEM_SIZE,
                           PROT_READ | PROT_WRITE,
                           MAP_SHARED | MAP_ANONYMOUS,
                           -1, 0);
    if (guest_mem == MAP_FAILED) {
        die("mmap guest memory");
    }

    /* Load guest code into guest memory */
    memcpy(guest_mem, guest_code, sizeof(guest_code));
    printf("[kvm] Guest memory: %d bytes at host %p\n", GUEST_MEM_SIZE, guest_mem);

    /* Set up guest physical memory region (KVM_SET_USER_MEMORY_REGION) */
    struct kvm_userspace_memory_region region = {
        .slot = 0,
        .flags = 0,
        .guest_phys_addr = 0,         /* guest physical address 0 */
        .memory_size = GUEST_MEM_SIZE,
        .userspace_addr = (uint64_t)guest_mem,
    };

    ret = ioctl(vm_fd, KVM_SET_USER_MEMORY_REGION, &region);
    if (ret < 0) {
        die("KVM_SET_USER_MEMORY_REGION");
    }
    printf("[kvm] Memory region mapped: guest PA 0 → host %p (%d bytes)\n",
           guest_mem, GUEST_MEM_SIZE);

    /* Step 4: Create a vCPU */
    int vcpu_fd = ioctl(vm_fd, KVM_CREATE_VCPU, 0);
    if (vcpu_fd < 0) {
        die("KVM_CREATE_VCPU");
    }
    printf("[kvm] vCPU created (fd=%d)\n", vcpu_fd);

    /* Map the vCPU's run structure */
    int mmap_size = ioctl(kvm_fd, KVM_GET_VCPU_MMAP_SIZE, 0);
    if (mmap_size < 0) {
        die("KVM_GET_VCPU_MMAP_SIZE");
    }

    run = mmap(NULL, mmap_size, PROT_READ | PROT_WRITE, MAP_SHARED, vcpu_fd, 0);
    if (run == MAP_FAILED) {
        die("mmap vcpu run");
    }

    /* Step 5: Initialize vCPU registers */
    /* Get special registers */
    ret = ioctl(vcpu_fd, KVM_GET_SREGS, &sregs);
    if (ret < 0) {
        die("KVM_GET_SREGS");
    }

    /* Set CS to start executing at guest physical address 0 (real mode) */
    sregs.cs.base = 0;
    sregs.cs.selector = 0;
    ret = ioctl(vcpu_fd, KVM_SET_SREGS, &sregs);
    if (ret < 0) {
        die("KVM_SET_SREGS");
    }

    /* Get and set general registers */
    ret = ioctl(vcpu_fd, KVM_GET_REGS, &regs);
    if (ret < 0) {
        die("KVM_GET_REGS");
    }

    regs.rip = 0;       /* instruction pointer: start at address 0 */
    regs.rflags = 0x2;  /* bit 1 is always set in rflags */
    regs.rax = 0;
    regs.rbx = 0;
    regs.rcx = 0;
    regs.rdx = 0;
    regs.rsi = 0;
    regs.rdi = 0;
    regs.rsp = GUEST_MEM_SIZE;  /* stack at top of memory */

    ret = ioctl(vcpu_fd, KVM_SET_REGS, &regs);
    if (ret < 0) {
        die("KVM_SET_REGS");
    }

    printf("[kvm] vCPU registers initialized (rip=0, rflags=0x2)\n\n");

    /* Step 6: Run the guest */
    printf("[kvm] Running guest...\n\n");

    while (1) {
        ret = ioctl(vcpu_fd, KVM_RUN, 0);
        if (ret < 0) {
            if (errno == EINTR)
                continue;
            die("KVM_RUN");
        }

        switch (run->exit_reason) {
        case KVM_EXIT_IO:
            if (run->io.direction == KVM_EXIT_IO_OUT &&
                run->io.port == 0x3f8 &&
                run->io.size == 1) {
                /* Guest wrote a byte to the serial port */
                uint8_t *data = (uint8_t *)run + run->io.data_offset;
                printf("%c", *data);
                fflush(stdout);
            } else {
                printf("[kvm] Unexpected I/O: port=0x%x %s size=%d\n",
                       run->io.port,
                       run->io.direction == KVM_EXIT_IO_OUT ? "OUT" : "IN",
                       run->io.size);
            }
            break;

        case KVM_EXIT_HLT:
            printf("\n\n[kvm] Guest halted (HLT instruction).\n");
            goto done;

        case KVM_EXIT_FAIL_ENTRY:
            fprintf(stderr, "[kvm] FAIL_ENTRY: hardware_entry_failure_reason=0x%llx\n",
                    (unsigned long long)run->fail_entry.hardware_entry_failure_reason);
            goto done;

        case KVM_EXIT_INTERNAL_ERROR:
            fprintf(stderr, "[kvm] INTERNAL_ERROR: suberror=%d\n",
                    run->internal.suberror);
            goto done;

        case KVM_EXIT_SHUTDOWN:
            printf("[kvm] Guest shutdown (triple fault).\n");
            goto done;

        default:
            fprintf(stderr, "[kvm] Unhandled exit reason: %d\n", run->exit_reason);
            goto done;
        }
    }

done:
    /* Cleanup */
    munmap(run, mmap_size);
    close(vcpu_fd);
    close(vm_fd);
    munmap(guest_mem, GUEST_MEM_SIZE);
    close(kvm_fd);

    printf("\nDone.\n");
    return 0;
}
