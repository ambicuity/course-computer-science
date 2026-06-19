# Reusable Artifact: SDN Controller + eBPF Packet Counter

## What this is

Two standalone, dependency-free implementations that demonstrate the core
abstractions of Software-Defined Networking and eBPF:

### 1. SDN Controller Simulation (`../code/main.py`)

An OpenFlow-like controller with:
- Priority-ordered flow tables (match + action + counters)
- Switches that send PACKET_IN on table-miss
- MAC learning via controller-installed flow rules
- Static flow rule installation via API (`install_flow()`)
- Per-switch and per-rule statistics

### 2. XDP Packet Counter (`../code/main.c`)

A userspace simulation of an eBPF/XDP program that:
- Parses Ethernet, IPv4, TCP, UDP, and ICMP headers from raw bytes
- Counts packets per protocol using a simulated `BPF_MAP_TYPE_ARRAY`
- Tracks per-flow byte counts (simulating `BPF_MAP_TYPE_HASH`)

## How to run

```bash
# Python SDN simulation
python3 ../code/main.py

# C eBPF simulation
gcc ../code/main.c -o /tmp/sdn_ebpf && /tmp/sdn_ebpf
```

## Where this is reused

- The flow-table match-action pattern appears in Phase 10 database query
  planning and in Phase 16 firewall ACL compilation.
- The eBPF map counter pattern (shared kernel/userspace data structures)
  reappears in Phase 17 distributed systems tracing.
