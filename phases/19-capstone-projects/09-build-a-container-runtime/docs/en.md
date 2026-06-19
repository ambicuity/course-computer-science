# Build a Container Runtime

> Isolation primitives plus process lifecycle control form the runtime core.

**Type:** Build
**Languages:** Go, C
**Prerequisites:** Phase 19 lessons 01-08
**Time:** ~600 minutes

## Learning Objectives

- Understand container runtime core responsibilities.
- Model namespace/cgroup setup workflow conceptually.
- Implement minimal process launch scaffold and config handling.
- Define safety checks and observability for runtime operations.

## The Problem

Container runtimes touch sensitive OS primitives. Someone starts building a runtime, calls `clone()` with `CLONE_NEWPID`, discovers that the child process can't see `/proc`, adds `CLONE_NEWNS` without mounting a new procfs, and gets a cascade of permission errors. Or they set up cgroups but forget to move the child process into the cgroup, so resource limits don't apply.

The root cause: namespaces, cgroups, and filesystem isolation are separate kernel mechanisms that must be composed correctly. Each has its own setup sequence, its own error modes, and its own cleanup requirements. When you blur the boundaries, a namespace bug looks like a cgroup bug, and a cgroup bug looks like a permissions bug.

Incremental scaffolds and clear contracts are necessary before full namespace/cgroup integration. The first milestone: parse a container spec (JSON config) and spawn a child process. The second: isolate the child in a PID namespace so it can't see the parent. Third: add cgroup resource limits. Each milestone is independently testable.

## The Concept

A container runtime has four responsibilities:

```
Container spec (OCI config.json)
        │
        ▼
┌───────────────┐
│ 1. Parse spec  │  Read container config, extract command, env, limits
└───────────────┘
        │
        ▼
┌───────────────┐
│ 2. Prepare     │  Set up namespaces, cgroups, filesystem
│  isolation     │  clone() with CLONE_NEWPID, CLONE_NEWNS, etc.
└───────────────┘
        │
        ▼
┌───────────────┐
│ 3. Spawn       │  exec() the container command in isolated environment
│  process       │  with new PID 1, new mount namespace
└───────────────┘
        │
        ▼
┌───────────────┐
│ 4. Track &     │  Wait for exit, forward signals, cleanup cgroups
│  cleanup       │  reap zombie processes
└───────────────┘
```

Linux namespaces isolate what a process can see:

| Namespace | Flag | What it isolates |
|---|---|---|
| PID | CLONE_NEWPID | Process IDs (container sees PID 1) |
| Mount | CLONE_NEWNS | Mount points (container has its own /) |
| Network | CLONE_NEWNET | Network interfaces (container has its own eth0) |
| UTS | CLONE_NEWUTS | Hostname |
| User | CLONE_NEWUID | User/group IDs |

Cgroups control what resources a process can use:

| Controller | What it limits |
|---|---|
| cpu | CPU time (shares, quota) |
| memory | Memory usage (limit, OOM behavior) |
| pids | Number of processes |
| blkio | Block I/O bandwidth |

## Build It

### Step 1: Container Spec Parser (Go)

```go
package main

import (
    "encoding/json"
    "fmt"
    "os"
    "os/exec"
    "path/filepath"
    "runtime"
    "syscall"
)

// ContainerSpec represents a minimal OCI-like container configuration
type ContainerSpec struct {
    Rootfs   string            `json:"rootfs"`   // Path to root filesystem
    Args     []string          `json:"args"`      // Command to run
    Env      map[string]string `json:"env"`       // Environment variables
    Hostname string            `json:"hostname"`
    Limits   ResourceLimits    `json:"limits"`
}

type ResourceLimits struct {
    MemoryLimitMB int `json:"memory_limit_mb"` // cgroup memory limit
    CPUShares     int `json:"cpu_shares"`       // cgroup CPU shares
    PidLimit      int `json:"pid_limit"`        // max processes
}

func loadSpec(path string) (*ContainerSpec, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        return nil, fmt.Errorf("reading spec: %w", err)
    }
    var spec ContainerSpec
    if err := json.Unmarshal(data, &spec); err != nil {
        return nil, fmt.Errorf("parsing spec: %w", err)
    }
    return &spec, nil
}
```

### Step 2: Namespace Setup (C)

```c
// nsenter.c — Namespace setup for container child
#define _GNU_SOURCE
#include <sched.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/mount.h>
#include <sys/wait.h>
#include <signal.h>

// Set up a minimal container environment
int container_child(void *arg) {
    char **argv = (char **)arg;

    // Set hostname
    sethostname("container", 9);

    // Mount proc filesystem (requires PID namespace)
    mount("proc", "/proc", "proc", 0, NULL);

    // Set environment
    setenv("container", "true", 1);

    // Execute the container command
    execvp(argv[0], argv);
    perror("execvp failed");
    return 1;
}

#define STACK_SIZE (1024 * 1024) // 1MB child stack

int main(int argc, char *argv[]) {
    if (argc < 3) {
        fprintf(stderr, "Usage: %s <rootfs> <command> [args...]\n", argv[0]);
        return 1;
    }

    char *rootfs = argv[1];
    char **cmd = &argv[2];

    // Allocate stack for child
    char *stack = malloc(STACK_SIZE);
    if (!stack) { perror("malloc"); return 1; }

    // Clone with new PID and mount namespaces
    int flags = CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWUTS | SIGCHLD;

    pid_t child = clone(container_child, stack + STACK_SIZE, flags, cmd);
    if (child == -1) { perror("clone"); return 1; }

    printf("Container started with PID %d (host), PID 1 (container)\n", child);

    // Wait for container to exit
    int status;
    waitpid(child, &status, 0);

    if (WIFEXITED(status)) {
        printf("Container exited with status %d\n", WEXITSTATUS(status));
    } else if (WIFSIGNALED(status)) {
        printf("Container killed by signal %d\n", WTERMSIG(status));
    }

    free(stack);
    return 0;
}
```

### Step 3: Cgroup Setup (Go)

```go
import (
    "fmt"
    "os"
    "path/filepath"
    "strconv"
)

const cgroupRoot = "/sys/fs/cgroup"

func setupCgroup(containerID string, limits ResourceLimits) (string, error) {
    cgroupPath := filepath.Join(cgroupRoot, "mini-container", containerID)

    // Create cgroup directory
    if err := os.MkdirAll(cgroupPath, 0755); err != nil {
        return "", fmt.Errorf("creating cgroup: %w", err)
    }

    // Set memory limit
    if limits.MemoryLimitMB > 0 {
        limitBytes := limits.MemoryLimitMB * 1024 * 1024
        memPath := filepath.Join(cgroupPath, "memory.max")
        if err := os.WriteFile(memPath, []byte(strconv.Itoa(limitBytes)), 0644); err != nil {
            return "", fmt.Errorf("setting memory limit: %w", err)
        }
    }

    // Set CPU shares
    if limits.CPUShares > 0 {
        cpuPath := filepath.Join(cgroupPath, "cpu.weight")
        if err := os.WriteFile(cpuPath, []byte(strconv.Itoa(limits.CPUShares)), 0644); err != nil {
            return "", fmt.Errorf("setting CPU shares: %w", err)
        }
    }

    // Set PID limit
    if limits.PidLimit > 0 {
        pidPath := filepath.Join(cgroupPath, "pids.max")
        if err := os.WriteFile(pidPath, []byte(strconv.Itoa(limits.PidLimit)), 0644); err != nil {
            return "", fmt.Errorf("setting PID limit: %w", err)
        }
    }

    return cgroupPath, nil
}

func addToCgroup(cgroupPath string, pid int) error {
    procsPath := filepath.Join(cgroupPath, "cgroup.procs")
    return os.WriteFile(procsPath, []byte(strconv.Itoa(pid)), 0644)
}

func cleanupCgroup(cgroupPath string) error {
    return os.RemoveAll(cgroupPath)
}
```

### Step 4: Full Runtime

```go
func runContainer(spec *ContainerSpec) error {
    containerID := "container-001"

    // Set up cgroup
    cgroupPath, err := setupCgroup(containerID, spec.Limits)
    if err != nil {
        return fmt.Errorf("cgroup setup: %w", err)
    }
    defer cleanupCgroup(cgroupPath)

    // Build command
    cmd := exec.Command(spec.Args[0], spec.Args[1:]...)
    cmd.Stdin = os.Stdin
    cmd.Stdout = os.Stdout
    cmd.Stderr = os.Stderr

    // Set namespace flags (on Linux)
    cmd.SysProcAttr = &syscall.SysProcAttr{
        Cloneflags: syscall.CLONE_NEWPID | syscall.CLONE_NEWNS | syscall.CLONE_NEWUTS,
    }

    // Set environment
    env := []string{"container=true", fmt.Sprintf("hostname=%s", spec.Hostname)}
    for k, v := range spec.Env {
        env = append(env, fmt.Sprintf("%s=%s", k, v))
    }
    cmd.Env = env

    // Start container
    if err := cmd.Start(); err != nil {
        return fmt.Errorf("starting container: %w", err)
    }

    fmt.Printf("Container %s started (PID %d)\n", containerID, cmd.Process.Pid)

    // Add to cgroup
    if err := addToCgroup(cgroupPath, cmd.Process.Pid); err != nil {
        cmd.Process.Kill()
        return fmt.Errorf("adding to cgroup: %w", err)
    }

    // Wait for exit
    return cmd.Wait()
}

func main() {
    if len(os.Args) < 2 {
        fmt.Println("Usage: minicontainer <spec.json>")
        os.Exit(1)
    }

    spec, err := loadSpec(os.Args[1])
    if err != nil {
        fmt.Fprintf(os.Stderr, "Error loading spec: %v\n", err)
        os.Exit(1)
    }

    if err := runContainer(spec); err != nil {
        fmt.Fprintf(os.Stderr, "Error: %v\n", err)
        os.Exit(1)
    }
}
```

## Use It

This structure mirrors real container runtimes:

- **runc**: the reference OCI runtime. Its `libcontainer` package handles namespace creation, cgroup setup, and seccomp filtering. The `factory.go` creates containers; `container.go` manages lifecycle; `process.go` handles process execution.
- **containerd**: a higher-level runtime that manages container lifecycle (create, start, stop, delete) and delegates to runc for the actual process isolation. Used by Docker and Kubernetes.
- **gVisor**: Google's application kernel that intercepts syscalls in user space, providing isolation without full Linux namespaces. Uses a Sentry process that implements the Linux kernel interface.
- **Firecracker**: AWS's microVM for serverless (Lambda, Fargate). Uses KVM for hardware-level isolation with minimal overhead. Similar lifecycle management but at the VM level.

The key production lesson: **signal forwarding and zombie reaping are the hardest parts to get right**. When a container's PID 1 process exits, the runtime must reap it as a zombie. When the runtime receives SIGTERM, it must forward it to the container process. If the container spawns children, the runtime must reap those too. Production runtimes use a dedicated reaper goroutine.

## Read the Source

- [OCI Runtime Spec](https://github.com/opencontainers/runtime-spec) — The standard container runtime interface. Defines the config.json format, lifecycle hooks, and runtime behavior.
- [runc source](https://github.com/opencontainers/runc) — The reference implementation. `libcontainer/` contains the namespace, cgroup, and seccomp setup code.
- [containerd architecture](https://github.com/containerd/containerd) — Higher-level container management. The `runtime/v2/` package shows how containerd delegates to runc.

## Ship It

- `code/main.go`: runtime config parser, cgroup setup, namespace-based process isolation.
- `code/main.c`: low-level C implementation using `clone()` for namespace creation.
- `outputs/README.md`: runtime safety checklist covering namespace setup, cgroup limits, signal handling, and cleanup.

## Exercises

1. **Easy** — Add cgroup quota config fields. Extend the ResourceLimits struct with `cpu_quota_us` (CPU time quota in microseconds per period) and `io_weight` (block I/O weight). Write these to the appropriate cgroup files.
2. **Medium** — Add namespace flag validation. Before cloning, validate that the requested namespaces are available on the current kernel (check `/proc/self/ns/`). Reject unsupported flags with a clear error message. Add a `--privileged` flag that skips namespace creation.
3. **Hard** — Add signal forwarding and shutdown policy. When the runtime receives SIGTERM, forward it to the container process. If the container doesn't exit within a timeout (e.g., 10 seconds), send SIGKILL. Implement a graceful shutdown sequence: SIGTERM, wait, SIGKILL.

## Key Terms

| Term | What people say | What it actually means |
|---|---|---|
| Namespace | "isolation domain" | A kernel feature that gives a process its own view of a system resource (PIDs, mount points, network interfaces). Created via `clone()` flags or `unshare()`. |
| cgroup | "resource limit" | Control Group: a kernel mechanism for accounting and limiting resource usage (CPU, memory, PIDs, I/O). Processes are added to cgroup control files. |
| OCI spec | "runtime contract" | Open Container Initiative runtime specification. Defines the config.json format, lifecycle hooks, and expected runtime behavior. The standard interface between container managers (Docker, containerd) and runtimes (runc). |
| Reaper | "zombie cleanup" | The parent process that calls `waitpid()` to collect child exit status. Without a reaper, exited children become zombies that consume process table entries. |
| Seccomp | "syscall filter" | Secure Computing Mode: a Linux kernel feature that restricts which syscalls a process can make. Used in containers to reduce the kernel attack surface. |

## Further Reading

- [OCI Runtime Spec](https://github.com/opencontainers/runtime-spec) — The standard container runtime interface.
- [runc](https://github.com/opencontainers/runc) — Reference OCI runtime implementation.
- [Containers from Scratch](https://www.youtube.com/watch?v=8fi7uSYlOdc) — Liz Rice's talk building a container runtime from scratch in Go.
