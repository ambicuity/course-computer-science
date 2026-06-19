# Linux for Builders — proc, sys, cgroups, namespaces

> The Linux kernel publishes its state as a filesystem. `cat`ting the right path is a syscall.

**Type:** Learn
**Languages:** Shell, C
**Prerequisites:** Phase 00, Lessons 01–02
**Time:** ~75 minutes

## Learning Objectives

- Read live kernel state from `/proc` and `/sys` using ordinary file I/O.
- Use `cgroups v2` to constrain a process's CPU, memory, and IO.
- Use namespaces (`unshare`, `nsenter`, `clone()` flags) to isolate a process's view of the system.
- Combine the three to understand what a "container" actually is: a process whose namespaces and cgroups have been set up.

## The Problem

When you `ps aux | grep nginx`, what does `ps` actually do? It opens files under `/proc/<pid>/`. When `htop` shows CPU usage per core, it's reading `/proc/stat`. When Docker says "this container can use at most 2 CPUs," it's writing to a file under `/sys/fs/cgroup/...`. When Kubernetes "isolates" pods, it's creating namespaces with `clone(CLONE_NEWPID | CLONE_NEWNET | ...)`.

If you only know the tools (`ps`, `htop`, `docker`, `kubectl`), you can't debug what's underneath. If you know the kernel interfaces — `/proc`, `/sys`, cgroups, namespaces — you can build the tools, read what they read, and diagnose what's happening when "the container is being killed by OOM but I don't know why."

This lesson is the tour of those four kernel interfaces. macOS users: most of this is Linux-specific. Use a Linux VM or container — Phase 11 has a devcontainer ready.

## The Concept

### `/proc` — process state as files

`/proc` is a pseudo-filesystem: nothing on disk, but reads materialize live kernel state into bytes.

```
/proc/cpuinfo                     CPU model, features
/proc/meminfo                     memory totals + breakdowns
/proc/loadavg                     1/5/15-minute load averages
/proc/stat                        CPU time accounting (user/sys/idle since boot)
/proc/diskstats                   disk I/O counters
/proc/uptime                      seconds since boot, idle time
/proc/version                     kernel version

/proc/<pid>/cmdline               argv concatenated, null-separated
/proc/<pid>/status                human-readable: state, uid, mem, threads
/proc/<pid>/stat                  one line of every counter (parseable)
/proc/<pid>/exe                   symlink to the binary
/proc/<pid>/cwd                   symlink to working dir
/proc/<pid>/fd/                   directory of open file descriptors
/proc/<pid>/maps                  memory map (libraries, heap, stack)
/proc/<pid>/limits                ulimits for this process
/proc/<pid>/environ               env vars (null-separated)
```

`ps aux` is essentially a `for pid in /proc/[0-9]*; do read /proc/$pid/status ...; done` rewritten in C.

### `/sys` — kernel and driver knobs

`/sys/` exposes hardware and driver state, plus some kernel tuning knobs:

```
/sys/block/sda/                   one dir per block device
/sys/class/net/eth0/              one dir per network interface
/sys/class/thermal/               temperature sensors
/sys/devices/system/cpu/cpu0/cpufreq/    per-core frequency control
/sys/kernel/mm/transparent_hugepage/     THP setting
/sys/fs/cgroup/                   cgroup v2 hierarchy
```

Writing to most `/sys` files changes kernel state (requires root). E.g.:

```sh
echo never > /sys/kernel/mm/transparent_hugepage/enabled   # disable THP
```

### cgroups — limit resources per process group

A **cgroup** (control group) is a set of processes that share a resource budget: CPU, memory, IO, PIDs. On modern kernels you use **cgroup v2** (unified hierarchy mounted at `/sys/fs/cgroup`).

```
/sys/fs/cgroup/
├── cgroup.controllers        which controllers are available
├── cgroup.procs              PIDs in this cgroup
├── cpu.max                   CPU limit: "QUOTA PERIOD" (microseconds)
├── memory.max                memory ceiling in bytes
├── memory.current            current usage
├── io.max                    per-device IO limit
└── my_app/                   create a new cgroup by mkdir
    ├── cgroup.procs          (write a PID here to move it into this cgroup)
    └── ...
```

To create a cgroup, limit it, and run a process inside:

```sh
sudo mkdir /sys/fs/cgroup/demo
echo "200000 1000000" | sudo tee /sys/fs/cgroup/demo/cpu.max     # 20% of one CPU
echo "100M"           | sudo tee /sys/fs/cgroup/demo/memory.max

echo $$ | sudo tee /sys/fs/cgroup/demo/cgroup.procs              # move this shell
yes > /dev/null &                                                 # busy loop
top                                                              # watch — CPU capped to ~20%
```

Docker, systemd, and Kubernetes all just write to these files. There's nothing magical underneath.

### Namespaces — partition the kernel's view of resources

A **namespace** is a kernel facility that gives a process its own view of one global resource:

| Namespace | What it isolates | `clone` flag |
|-----------|------------------|--------------|
| `pid`   | Process IDs (your PID 1 isn't the system PID 1) | `CLONE_NEWPID` |
| `mnt`   | Mount points | `CLONE_NEWNS` |
| `net`   | Network interfaces, routes, firewall | `CLONE_NEWNET` |
| `uts`   | Hostname, domain name | `CLONE_NEWUTS` |
| `ipc`   | SysV IPC, POSIX message queues | `CLONE_NEWIPC` |
| `user`  | UID/GID mappings | `CLONE_NEWUSER` |
| `cgroup`| Which cgroup tree you see | `CLONE_NEWCGROUP` |
| `time`  | CLOCK_MONOTONIC + CLOCK_BOOTTIME offsets | `CLONE_NEWTIME` |

Use `unshare` from userspace:

```sh
sudo unshare --pid --fork --mount-proc bash    # new PID namespace
ps aux                                          # this shell sees only its own subtree
```

Each `/proc/<pid>/ns/<ns-type>` symlink tells you what namespace a process is in. Two processes with the same `ns:[...]` inode share that namespace.

### "A container" = namespaces + cgroups + filesystem

A container is just:

1. A set of namespaces (typically: `pid`, `mnt`, `net`, `uts`, `ipc`, `user`).
2. A cgroup with resource limits.
3. A root filesystem (chroot-style or via `pivot_root`).
4. Optionally, a seccomp filter (which syscalls are allowed).

Docker, containerd, podman, runc — all of these set these up. Nothing more. Phase 11 shows you the same setup from the Docker side.

## Build It

### Step 1: Read `/proc` from C

The lesson's `main.c` reads `/proc/self/status`, `/proc/uptime`, and `/proc/loadavg` and pretty-prints them. Build and run:

```sh
gcc main.c -o procreader
./procreader
```

Read the source — every "system info" tool you've ever used is doing this.

### Step 2: Walk `/proc/<pid>` for a running process

```sh
# Long-running target
sleep 1000 &
PID=$!
echo "PID = $PID"

cat /proc/$PID/cmdline | tr '\0' ' '; echo
cat /proc/$PID/status | head -10
ls -l /proc/$PID/fd       # which fds is it holding?
cat /proc/$PID/maps | head # which libraries are mapped in?
kill $PID
```

### Step 3: cgroup v2 demo

If you have a Linux box (or VM) with cgroup v2 (`mount | grep cgroup2`):

```sh
sudo mkdir /sys/fs/cgroup/demo
echo "50000 1000000" | sudo tee /sys/fs/cgroup/demo/cpu.max    # 5% of one CPU

# Spawn a CPU loop and move it in
( while :; do :; done ) &
PID=$!
echo $PID | sudo tee /sys/fs/cgroup/demo/cgroup.procs

top -p $PID                                                     # ~5% CPU
kill -9 $PID
sudo rmdir /sys/fs/cgroup/demo                                  # clean up
```

### Step 4: PID namespace demo

```sh
sudo unshare --pid --fork --mount-proc bash
# Now you're in a new PID namespace + mounted /proc points at it
ps aux                # this shell sees only its own subtree; ps PID=1 is bash
echo "I am PID $$"    # very small number
exit
```

### Step 5: User namespace — sudo-less isolation

```sh
unshare --user --map-root-user bash    # no sudo needed
id                                       # uid=0(root) but only inside this ns
cat /proc/self/uid_map                   # shows: 0 1000 1   (root in ns → real uid 1000)
```

You're root inside the namespace, but the kernel maps your namespaced UID 0 back to your real, unprivileged UID outside. This is how rootless containers work.

### Step 6: Network namespace

```sh
sudo ip netns add demo                # create a netns
sudo ip netns exec demo ip addr       # only `lo`, no eth0
sudo ip netns exec demo ping 8.8.8.8  # fails — no route
sudo ip netns del demo
```

A container has one of these, plus a veth pair to the host network namespace.

## Use It

- **Docker** runs a process with `clone(CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWUTS | CLONE_NEWIPC | CLONE_NEWNET | CLONE_NEWUSER, ...)`, then drops it into a cgroup, then `pivot_root`s to the image's filesystem. That's it.
- **systemd** uses cgroups extensively. Every unit (`*.service`) is its own cgroup; `systemctl status` reads `cgroup.procs` to list units' processes.
- **`top`, `htop`, `ps`** all read `/proc/<pid>/stat` and friends.
- **`docker exec`** uses `nsenter` to attach a new process into an existing container's namespaces.

## Read the Source

- `man 5 proc` — exhaustive guide to every file under `/proc`. A reference, not a tutorial.
- `man 7 cgroups` and `man 7 cgroup.v2` — kernel docs for cgroups.
- `man 7 namespaces` — every namespace type, in detail.
- [`runc` source](https://github.com/opencontainers/runc) — the reference container runtime. `libcontainer/` has the namespace + cgroup setup code.

## Ship It

This lesson ships **`outputs/sysinfo.sh`** — a script that prints a one-page summary of the host: CPU, memory, load, top processes, open files, cgroup limits if any. Reusable as a triage tool.

## Exercises

1. **Easy.** Write a one-liner that prints the top 5 processes by RSS (resident memory). Use `/proc/[0-9]*/status` directly, no `ps`.
2. **Medium.** Create a cgroup, set `memory.max=50M`, run a process that allocates a 100MB buffer inside it, and observe the OOM kill in `dmesg`.
3. **Hard.** Write a small program (50 lines of C or Rust) that uses `clone(CLONE_NEWPID | CLONE_NEWUTS | CLONE_NEWNS)` to spawn a child in fresh namespaces, sets the child's hostname to "container", remounts `/proc`, then `execv`s a shell. You've written 20 lines of a container runtime.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| `/proc` | "Linux files" | A pseudo-filesystem that exposes kernel/process state through read/write on virtual files |
| cgroup | "Container resource limits" | A kernel-tracked process group with attached limits on CPU, memory, IO, PIDs |
| Namespace | "Isolation" | A kernel facility that gives a process a private view of one global resource (PIDs, network, mounts, etc.) |
| Pseudo-fs | "Fake filesystem" | A filesystem with no on-disk backing; reads materialize live data, writes change kernel state |

## Further Reading

- *The Linux Programming Interface* (Kerrisk) — chapter 44 (Pseudo-Terminals), 45 (Memory mapping), and the whole "Process credentials" section.
- [LWN — A glimpse at containers](https://lwn.net/Articles/780364/) — readable explanation of how container runtimes use these primitives.
- [Julia Evans's zines](https://wizardzines.com/) — illustrated explanations of `/proc`, cgroups, and namespaces, beginner-friendly.
