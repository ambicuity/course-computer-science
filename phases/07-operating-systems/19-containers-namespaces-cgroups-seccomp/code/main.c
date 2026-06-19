#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sched.h>
#include <signal.h>
#include <sys/wait.h>
#include <sys/mount.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <sys/prctl.h>
#include <linux/seccomp.h>
#include <linux/filter.h>
#include <linux/audit.h>
#include <errno.h>

/*
 * Container primitives demo: namespaces, cgroups, seccomp, chroot.
 * Phase 07 — Operating Systems, Lesson 19
 *
 * Compile: gcc -o container_demo main.c
 * Run:     sudo ./container_demo
 *
 * Requires root for namespace and cgroup operations.
 */

#define STACK_SIZE (1024 * 1024)  /* 1 MB child stack */

/* ====== create_container ======
 *
 * Uses clone() with namespace flags to create an isolated process.
 * The child runs in its own PID, mount, and UTS namespaces.
 */
static int child_func(void *arg) {
    (void)arg;

    /* In the new PID namespace, we are PID 1 */
    printf("[container] PID inside namespace: %d (expect 1)\n", getpid());

    /* Set a new hostname for this container */
    if (sethostname("mycontainer", 11) != 0) {
        perror("sethostname");
    }

    char hostname[64];
    gethostname(hostname, sizeof(hostname));
    printf("[container] Hostname: %s\n", hostname);

    /* Mount a new proc filesystem */
    if (mount("proc", "/proc", "proc", 0, NULL) != 0) {
        perror("mount /proc (may need existing /proc mount)");
    }

    /* Execute a shell or simple command */
    printf("[container] Executing /bin/sh -c 'echo hello from container; ps aux'\n");
    execl("/bin/sh", "sh", "-c", "echo 'Hello from container!'; ps aux 2>/dev/null || true", NULL);

    /* If exec fails */
    perror("exec");
    return 1;
}

static int create_container(void) {
    printf("=== Namespace Container Demo ===\n\n");

    char *stack = malloc(STACK_SIZE);
    if (!stack) {
        perror("malloc");
        return 1;
    }

    /* clone() with namespace flags:
     * CLONE_NEWPID  — new PID namespace (child is PID 1)
     * CLONE_NEWNS   — new mount namespace (isolated filesystem)
     * CLONE_NEWUTS  — new UTS namespace (own hostname)
     */
    int flags = CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWUTS | SIGCHLD;
    pid_t pid = clone(child_func, stack + STACK_SIZE, flags, NULL);

    if (pid < 0) {
        perror("clone");
        free(stack);
        return 1;
    }

    printf("[host] Container PID (from host view): %d\n", pid);

    int status;
    waitpid(pid, &status, 0);
    printf("[host] Container exited with status %d\n\n", WEXITSTATUS(status));

    free(stack);
    return 0;
}

/* ====== setup_cgroups ======
 *
 * Writes to cgroupfs to limit CPU and memory for a given PID.
 * This mirrors what container runtimes do programmatically.
 */
static int setup_cgroups(pid_t pid, int cpu_quota_pct, long mem_limit_bytes) {
    printf("=== cgroups Setup Demo ===\n\n");

    char cgroup_path[256];
    char buf[256];

    /* --- Memory cgroup --- */
    const char *mem_dir = "/sys/fs/cgroup/memory/container_demo";

    if (mkdir(mem_dir, 0755) != 0 && errno != EEXIST) {
        perror("mkdir memory cgroup");
        printf("  (cgroupfs may not be mounted — skipping cgroup setup)\n");
        return -1;
    }

    /* Set memory limit */
    snprintf(cgroup_path, sizeof(cgroup_path), "%s/memory.limit_in_bytes", mem_dir);
    FILE *f = fopen(cgroup_path, "w");
    if (f) {
        fprintf(f, "%ld\n", mem_limit_bytes);
        fclose(f);
        printf("[cgroups] Memory limit: %ld bytes (%.1f MB)\n",
               mem_limit_bytes, mem_limit_bytes / (1024.0 * 1024.0));
    }

    /* Add process to cgroup */
    snprintf(cgroup_path, sizeof(cgroup_path), "%s/tasks", mem_dir);
    f = fopen(cgroup_path, "w");
    if (f) {
        fprintf(f, "%d\n", pid);
        fclose(f);
        printf("[cgroups] Added PID %d to memory cgroup\n", pid);
    }

    /* --- CPU cgroup --- */
    const char *cpu_dir = "/sys/fs/cgroup/cpu/container_demo";

    if (mkdir(cpu_dir, 0755) != 0 && errno != EEXIST) {
        perror("mkdir cpu cgroup");
        return -1;
    }

    /* Set CPU quota: quota_pct% of one core
     * CFS scheduler: quota/period = fraction of CPU
     * period = 100ms (100000us), quota = period * pct / 100
     */
    int period = 100000;
    int quota = period * cpu_quota_pct / 100;

    snprintf(cgroup_path, sizeof(cgroup_path), "%s/cpu.cfs_period_us", cpu_dir);
    f = fopen(cgroup_path, "w");
    if (f) {
        fprintf(f, "%d\n", period);
        fclose(f);
    }

    snprintf(cgroup_path, sizeof(cgroup_path), "%s/cpu.cfs_quota_us", cpu_dir);
    f = fopen(cgroup_path, "w");
    if (f) {
        fprintf(f, "%d\n", quota);
        fclose(f);
        printf("[cgroups] CPU limit: %d%% of one core (quota=%d, period=%d)\n",
               cpu_quota_pct, quota, period);
    }

    /* Add process to CPU cgroup */
    snprintf(cgroup_path, sizeof(cgroup_path), "%s/tasks", cpu_dir);
    f = fopen(cgroup_path, "w");
    if (f) {
        fprintf(f, "%d\n", pid);
        fclose(f);
    }

    printf("[cgroups] cgroup setup complete\n\n");

    /* Cleanup: rmdir cgroup dirs on exit (they must be empty first) */
    snprintf(buf, sizeof(buf), "rmdir %s 2>/dev/null; rmdir %s 2>/dev/null", mem_dir, cpu_dir);
    /* Note: in production, cleanup after the container exits */

    return 0;
}

/* ====== setup_seccomp ======
 *
 * Installs a seccomp-bpf filter that blocks dangerous syscalls.
 * Uses SECCOMP_RET_KILL for blocked syscalls (process is killed on use).
 *
 * This is a simplified version — Docker uses libseccomp with more
 * sophisticated profiles.
 */
static int setup_seccomp(void) {
    printf("=== seccomp Setup Demo ===\n\n");

    /* BPF filter program: allow common syscalls, block dangerous ones.
     *
     * We use a simple approach: check the syscall number against a
     * blacklist. In production, use a whitelist approach.
     */

    /* Dangerous syscalls to block (x86_64 numbers) */
    /* 169 = reboot, 310 = process_vm_writev,
     * 175 = init_module, 313 = finit_module */

    struct sock_filter filter[] = {
        /* Load syscall number */
        BPF_STMT(BPF_LD | BPF_W | BPF_ABS, offsetof(struct seccomp_data, nr)),

        /* Block reboot (169) */
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 169, 4, 0),

        /* Block init_module (175) */
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 175, 3, 0),

        /* Block finit_module (313) */
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 313, 2, 0),

        /* Block process_vm_writev (310) */
        BPF_JUMP(BPF_JMP | BPF_JEQ | BPF_K, 310, 1, 0),

        /* Allow all other syscalls */
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_ALLOW),

        /* Kill process if matched above */
        BPF_STMT(BPF_RET | BPF_K, SECCOMP_RET_KILL),
    };

    struct sock_fprog prog = {
        .len = sizeof(filter) / sizeof(filter[0]),
        .filter = filter,
    };

    /* Must set no_new_privs before installing seccomp filter */
    if (prctl(PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0) {
        perror("prctl(PR_SET_NO_NEW_PRIVS)");
        return -1;
    }

    if (syscall(SYS_seccomp, SECCOMP_SET_MODE_FILTER, 0, &prog) != 0) {
        /* Fallback: try prctl-based seccomp */
        if (prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER, &prog) != 0) {
            perror("seccomp");
            printf("  (seccomp may not be available in this kernel)\n");
            return -1;
        }
    }

    printf("[seccomp] Filter installed: blocking reboot, init_module, finit_module, process_vm_writev\n");
    printf("[seccomp] All other syscalls are allowed\n\n");

    return 0;
}

/* ====== chroot_demo ======
 *
 * Demonstrates chroot — changing the root filesystem.
 * This is the simplest form of filesystem isolation.
 */
static int chroot_demo(void) {
    printf("=== chroot Demo ===\n\n");

    /* Create a minimal root filesystem */
    const char *new_root = "/tmp/container_root";

    if (mkdir(new_root, 0755) != 0 && errno != EEXIST) {
        perror("mkdir new_root");
        return -1;
    }

    /* Create essential directories */
    char path[256];
    snprintf(path, sizeof(path), "%s/bin", new_root);
    mkdir(path, 0755);
    snprintf(path, sizeof(path), "%s/lib", new_root);
    mkdir(path, 0755);
    snprintf(path, sizeof(path), "%s/lib64", new_root);
    mkdir(path, 0755);
    snprintf(path, sizeof(path), "%s/usr", new_root);
    mkdir(path, 0755);

    /* Copy a shell into the new root (simplified — real containers use bind mounts) */
    snprintf(path, sizeof(path), "cp /bin/sh %s/bin/ 2>/dev/null", new_root);
    system(path);
    snprintf(path, sizeof(path), "ldd /bin/sh 2>/dev/null | grep -o '/lib[^ ]*' | xargs -I{} cp {} %s/lib/ 2>/dev/null", new_root);
    system(path);

    printf("[chroot] Created minimal root at %s\n", new_root);
    printf("[chroot] Contains: /bin/sh + required libraries\n");

    /* Fork and chroot the child */
    pid_t pid = fork();
    if (pid == 0) {
        /* Child: chroot into the new root */
        if (chroot(new_root) != 0) {
            perror("chroot");
            _exit(1);
        }
        if (chdir("/") != 0) {
            perror("chdir");
            _exit(1);
        }

        printf("[chroot] Successfully chrooted to %s\n", new_root);
        printf("[chroot] Running 'ls /' inside chroot:\n");
        execl("/bin/sh", "sh", "-c", "ls /; echo 'Inside chroot!'", NULL);
        perror("exec");
        _exit(1);
    } else if (pid > 0) {
        int status;
        waitpid(pid, &status, 0);
        printf("[chroot] Child exited with status %d\n", WEXITSTATUS(status));
    } else {
        perror("fork");
    }

    /* Cleanup */
    snprintf(path, sizeof(path), "rm -rf %s", new_root);
    system(path);

    printf("\n");
    return 0;
}

int main(int argc, char *argv[]) {
    (void)argc;
    (void)argv;

    printf("Containers: Namespaces, cgroups, seccomp\n");
    printf("=========================================\n\n");

    /* Demo 1: chroot filesystem isolation */
    chroot_demo();

    /* Demo 2: Namespace container creation (needs root) */
    if (getuid() != 0) {
        printf("[!] Namespace and cgroup demos require root.\n");
        printf("    Run: sudo ./container_demo\n\n");
    } else {
        create_container();

        /* Demo 3: cgroup resource limits */
        setup_cgroups(getpid(), 25, 256 * 1024 * 1024);  /* 25% CPU, 256MB */
    }

    /* Demo 4: seccomp syscall filter */
    setup_seccomp();

    printf("Done.\n");
    return 0;
}
