# Lesson 19: Containers — Namespaces, cgroups, seccomp

## Why This Matters

Containers are how modern software is packaged and deployed. Every cloud service, CI pipeline, and microservice you'll encounter runs inside a container. But containers aren't a kernel feature — they're a **composition of three Linux primitives**: namespaces (isolation), cgroups (resource limits), and seccomp (syscall filtering). Understanding these primitives means you understand Docker, Kubernetes, LXC, and every container runtime from the inside out.

## What Is a Container?

A container is an **isolated user-space instance**. It looks like its own Linux system — it has its own PID tree, filesystem root, network stack, hostname, and user IDs — but it shares the same kernel as the host.

```
  Host Linux Kernel
  ┌─────────────────────────────────────────────────┐
  │                                                 │
  │  ┌──────────────┐    ┌──────────────┐           │
  │  │ Container A  │    │ Container B  │           │
  │  │              │    │              │           │
  │  │ PID 1: nginx │    │ PID 1: redis │           │
  │  │ / = /var/... │    │ / = /var/... │           │
  │  │ eth0: 10.0.x │    │ eth0: 10.0.y │           │
  │  │ hostname: a  │    │ hostname: b  │           │
  │  └──────────────┘    └──────────────┘           │
  │                                                 │
  │  namespaces × cgroups × seccomp = container     │
  └─────────────────────────────────────────────────┘
```

Containers are **not** VMs. They share the kernel; VMs emulate entire machines. Containers start in milliseconds, use less memory, and pack more densely — but offer weaker isolation.

## The Three Primitives

### 1. Namespaces — "What You Can See"

Namespaces partition kernel resources so that one set of processes sees one view, and another set sees a different view. Each namespace type isolates a different resource:

| Namespace | Flag | What It Isolates |
|-----------|------|-----------------|
| PID | `CLONE_NEWPID` | Process IDs — container sees its own PID 1 |
| Mount | `CLONE_NEWNS` | Filesystem mount points — own root, `/proc`, `/sys` |
| Network | `CLONE_NEWNET` | Network stack — own interfaces, routes, iptables |
| UTS | `CLONE_NEWUTS` | Hostname and domain name |
| IPC | `CLONE_NEWIPC` | IPC mechanisms — shared memory, semaphores, message queues |
| User | `CLONE_NEWUSER` | UID/GID mappings — root inside container, unprivileged outside |
| Cgroup | `CLONE_NEWCGROUP` | cgroup root directory view |

**PID namespace example:** The first process created in a new PID namespace becomes PID 1 (init). It can't see processes outside its namespace. When PID 1 exits, all processes in that namespace are killed.

**Mount namespace example:** Each container gets its own mount table. You can `chroot` or pivot to a different root filesystem without affecting the host.

**Network namespace example:** Each container gets a virtual Ethernet pair (`veth`). One end is in the container's namespace, the other is on the host (usually attached to a bridge like `docker0`).

### 2. cgroups — "How Much You Can Use"

Control groups (cgroups) limit and account for resource usage. Without cgroups, a container could consume all CPU, memory, or disk I/O on the host.

**CPU control:**
- `cpu.shares` — relative weight (default 1024). A container with 2048 shares gets twice the CPU of one with 1024.
- `cpu.cfs_quota_us` / `cpu.cfs_period_us` — hard cap. `quota=50000, period=100000` = max 50% of one CPU.

**Memory control:**
- `memory.limit_in_bytes` — hard limit. Exceeding it triggers OOM killer.
- `memory.soft_limit_in_bytes` — soft limit. Under memory pressure, the kernel reclaims from containers exceeding this first.
- `memory.oom_control` — configure OOM behavior (kill process or pause).

**I/O control (blkio):**
- `blkio.weight` — relative I/O weight (10–1000).
- `blkio.throttle.read_bps_device` — bytes/sec limit per device.

```
  /sys/fs/cgroup/
  ├── cpu/
  │   └── my_container/
  │       ├── cpu.shares          (1024)
  │       ├── cpu.cfs_quota_us    (50000)
  │       └── cpu.cfs_period_us   (100000)
  ├── memory/
  │   └── my_container/
  │       ├── memory.limit_in_bytes  (536870912 = 512 MB)
  │       └── memory.usage_in_bytes
  └── blkio/
      └── my_container/
          └── blkio.weight        (500)
```

**How to use cgroups directly (no Docker needed):**

```bash
# Create a cgroup
sudo mkdir /sys/fs/cgroup/memory/my_container

# Set memory limit to 256MB
echo 268435456 | sudo tee /sys/fs/cgroup/memory/my_container/memory.limit_in_bytes

# Add a process to the cgroup
echo $$ | sudo tee /sys/fs/cgroup/memory/my_container/tasks

# CPU: limit to 25% of one core
sudo mkdir /sys/fs/cgroup/cpu/my_container
echo 50000 | sudo tee /sys/fs/cgroup/cpu/my_container/cpu.cfs_quota_us
echo 100000 | sudo tee /sys/fs/cgroup/cpu/my_container/cpu.cfs_period_us
```

### 3. seccomp — "What You Can Do"

Secure Computing (seccomp) restricts which **system calls** a process can make. Even if a container is compromised, seccomp limits the damage by blocking dangerous syscalls.

**Seccomp modes:**
- `SECCOMP_MODE_STRICT` — allows only `read()`, `write()`, `exit()`, `sigreturn()`. Too restrictive for most use.
- `SECCOMP_MODE_FILTER` — BPF program inspects each syscall and decides: allow, kill, trap, trace, log.

**Docker's default seccomp profile** blocks ~44 dangerous syscalls including:
- `reboot`, `kexec_load` — can crash the host
- `mount`, `umount2` — can escape filesystem isolation
- `swapon`, `swapoff` — can affect host memory
- `init_module`, `finit_module` — can load kernel modules
- `keyctl` — can manipulate host keyring
- `bpf` — can modify seccomp filters themselves

**Whitelist vs blacklist:** Docker uses a blacklist (block known dangerous syscalls). For high-security environments, a whitelist (allow only needed syscalls) is better.

## chroot — The Predecessor

`chroot` changes the root filesystem for a process. It's the oldest isolation primitive — predating namespaces by decades.

```c
chroot("/path/to/new/root");  /* change root */
chdir("/");                    /* must cd to / after chroot */
```

**Limitations of chroot alone:**
- No PID isolation — processes can still see host PIDs
- No network isolation — shares the host's network stack
- No resource limits — a chrooted process can use all CPU/memory
- Root can escape chroot with `chroot_escape()` techniques

chroot is a building block, not a security boundary. Containers use mount namespaces instead.

## Build It

See `code/main.c` for a minimal container creation program and `code/run.sh` for cgroup/namespace interaction scripts.

The C program demonstrates:
- `create_container()` — uses `clone()` with `CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWUTS` flags
- `setup_cgroups(pid, cpu_quota, mem_limit)` — writes to cgroupfs to limit resources
- `setup_seccomp()` — installs a seccomp-bpf filter blocking dangerous syscalls
- `chroot_demo()` — creates an isolated filesystem root

The shell scripts show how to:
- Create and configure cgroups directly via `/sys/fs/cgroup/`
- Enter existing namespaces with `nsenter`
- Inspect namespace info from `/proc/<pid>/ns/`

## Use It

**Docker** uses all three primitives together:
1. `clone()` with namespace flags to create the container process
2. cgroups to enforce resource limits from the container config
3. seccomp to filter syscalls (default profile or custom)
4. Layered filesystem (overlay2) for the container image
5. Networking via veth pairs + bridge or CNI plugins

**Kubernetes** adds orchestration on top — it schedules containers across nodes, manages networking (CNI), storage (CSI), and uses container runtimes (containerd, CRI-O) that invoke these primitives.

**LXC/LXD** uses the same primitives directly, with a more "VM-like" experience.

## Read the Source

- Linux kernel: `kernel/nsproxy.c` — namespace proxy structure, manages per-process namespace sets
- Linux kernel: `kernel/cgroup/cgroup-v2.c` — cgroup v2 implementation
- Linux kernel: `kernel/seccomp.c` — seccomp filter evaluation
- `man 7 namespaces` — namespace overview and `/proc/pid/ns/` interface
- `man 7 cgroups` — cgroup hierarchy and controllers

## Ship It

The container primitives toolkit in `code/main.c` demonstrates how to build a minimal container from raw Linux syscalls. The shell scripts in `code/run.sh` show how to interact with namespaces and cgroups directly from the command line.

## Exercises

### Level 1 — Recall

List the six namespace types and what each isolates. What is the difference between `cpu.shares` and `cpu.cfs_quota_us`? Why does Docker use seccomp in blacklist mode rather than whitelist?

### Level 2 — Application

Write a C program that creates a child in a new PID namespace using `clone()`. The child should print its PID (which should be 1) and list the processes it can see in `/proc`. Verify that the child cannot see host processes.

### Level 3 — Build

Create a "mini-Docker" in C that:
1. Uses `clone()` with PID, mount, UTS, and network namespaces
2. Sets up cgroup limits (CPU: 25% of one core, memory: 128MB)
3. Installs a seccomp whitelist that allows only `read`, `write`, `open`, `close`, `fstat`, `mmap`, `brk`, `exit_group`, `access`
4. `pivot_root` into a minimal rootfs
5. Executes a shell inside the container

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Container | "Lightweight VM" | Isolated process tree using namespaces + cgroups + seccomp |
| Namespace | "Process isolation" | Kernel mechanism that partitions resource visibility per process group |
| cgroup | "Resource limiter" | Kernel subsystem that accounts for and limits resource usage |
| seccomp | "Syscall filter" | BPF-based system call filtering in the kernel |
| chroot | "Change root" | Changes the apparent root filesystem for a process (not a security boundary) |
| pivot_root | "Switch rootfs" | Atomically swaps the root filesystem (used instead of chroot in containers) |

## Further Reading

- `man 7 namespaces`
- `man 7 cgroups`
- `man 2 clone`
- `man 2 seccomp`
- Liz Rice, *Containers From Scratch* (GopherCon talk)
- Docker documentation: [Seccomp security profiles](https://docs.docker.com/engine/security/seccomp/)
- Linux kernel source: `kernel/nsproxy.c`, `kernel/cgroup/`, `kernel/seccomp.c`
