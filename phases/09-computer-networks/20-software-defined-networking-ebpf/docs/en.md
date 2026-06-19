# Software-Defined Networking & eBPF

> Separate the control plane from the data plane, then program the kernel.

**Type:** Learn
**Languages:** C, Python
**Prerequisites:** Phase 09 lessons 01–19
**Time:** ~75 minutes

## Learning Objectives

- Understand the separation of control plane and data plane in SDN.
- Implement a simulated OpenFlow-like SDN controller with flow tables.
- Understand eBPF architecture (maps, helpers, verifier) and its role in kernel programmability.
- Compare SDN/eBPF with traditional networking in terms of flexibility and performance.

## The Problem

Configuring 1000 switches independently via CLI for a routing change takes hours and risks misconfiguration. Before eBPF, adding custom packet processing to Linux meant kernel modules (unsafe) or iptables (inflexible). SDN centralizes the control plane; eBPF provides safe kernel programmability. Together they shift networking from static to programmable.

## The Concept

### Control Plane vs Data Plane

In a traditional switch, the control plane (routing) and data plane (forwarding) run on the same device. SDN splits them: a centralized controller installs flow rules into switches via OpenFlow. Each flow entry has match fields (ports, MACs, IPs), priority, counters, actions (output, drop, flood), and timeouts.

### eBPF Architecture

eBPF (extended Berkeley Packet Filter) is a sandboxed bytecode runtime in the Linux kernel. Programs are written in restricted C, compiled to BPF bytecode, verified for safety (no unbounded loops, valid memory, type-safe helpers, max 4096 instructions), and JIT-compiled:

```
C Code -> LLVM/clang -> BPF Verifier -> JIT Compiler -> Kernel Hook
```

Key hooks:
- **XDP (eXpress Data Path)**: runs in the NIC driver before SKB allocation — fastest hook for DDoS mitigation, load balancing, packet filtering.
- **TC BPF**: runs in the traffic control layer for classification, shaping, and mangling.
- **BPF maps**: kernel-userspace shared data structures (hash, array, ring buffer, stack trace).

## Build It

### Step 1: SDN Controller Simulation in Python

The core abstractions: `FlowRule` (match + actions + counters), `FlowTable` (priority-ordered rules), `OpenFlowSwitch` (ports + flow lookup), and `SDNController` (MAC learning + rule installation):

```python
class FlowRule:
    def __init__(self, priority=0, match=None, actions=None):
        self.priority = priority
        self.match = match or {}
        self.actions = actions or []
        self.packet_count = 0

    def matches(self, packet):
        return all(packet.get(k) == v for k, v in self.match.items())

class FlowTable:
    def __init__(self):
        self.rules = []
    def add_rule(self, rule):
        self.rules.append(rule)
        self.rules.sort(key=lambda r: r.priority, reverse=True)
    def lookup(self, packet):
        for rule in self.rules:
            if rule.matches(packet):
                return rule.apply_actions(packet)
        return None

class OpenFlowSwitch:
    def receive_packet(self, packet):
        actions = self.flow_table.lookup(packet)
        if actions:
            return self._execute_actions(packet, actions)
        if self.controller:
            self.controller.handle_packet_in(self, packet)
            return ["PACKET_IN"]
        return []

class SDNController:
    def handle_packet_in(self, switch, packet):
        self._mac_table[packet["src_mac"]] = (switch.switch_id, packet["in_port"])
        dst_entry = self._mac_table.get(packet["dst_mac"])
        if dst_entry:
            rule = FlowRule(100, {"dst_mac": packet["dst_mac"]},
                            [f"output:{dst_entry[1]}"])
            switch.flow_table.add_rule(rule)
```

A 3-switch topology is created; the first packet causes a table-miss, the controller learns the MAC and installs rules so subsequent packets take the fast path. Run: `python3 code/main.py`.

### Step 2: eBPF Packet Counter in C

Simulates an XDP program parsing Ethernet/IP/transport headers and counting packets per protocol:

```c
static uint64_t proto_counts[4] = {0};
int xdp_program(const uint8_t *data, size_t len) {
    struct eth_hdr *eth = (struct eth_hdr *)data;
    if (ntoh16(eth->eth_type) != 0x0800) return XDP_PASS;
    struct ipv4_hdr *ip = (struct ipv4_hdr *)(data + 14);
    int idx;
    switch (ip->protocol) {
    case 6:  idx = 0; break;
    case 17: idx = 1; break;
    case 1:  idx = 2; break;
    default: idx = 3;
    }
    __sync_fetch_and_add(&proto_counts[idx], 1);
    return XDP_PASS;
}
```

The C program builds raw test packets, processes them through `xdp_program()`, and prints protocol counts and top flows. Compile & run: `gcc code/main.c -o /tmp/sdn_ebpf && /tmp/sdn_ebpf`.

## Use It

- **Cilium** — eBPF-based CNI for Kubernetes. Uses XDP for service load balancing and TC BPF for network policy. The `bpf/` directory contains production eBPF programs for NAT, load balancing, L3/L4 policy.
- **Calico** — Kubernetes networking with optional eBPF data plane replacing iptables.
- **Cloudflare L4Drop** — XDP-based DDoS mitigation at the NIC driver level, running at 100+ Gbps on commodity servers.
- **Katran** — Facebook's XDP load balancer using BPF maps for backend state and BPF helpers for IPIP/GRE encapsulation.
- **bpftool** — eBPF debugging: `bpftool prog list`, `bpftool map dump`, `bpftool prog trace`.

## Read the Source

- Linux kernel: `kernel/bpf/` — verifier (`verifier.c`), interpreter (`core.c`), JIT compilers.
- Linux kernel: `net/core/filter.c` — eBPF hooks in the networking stack (XDP attachment, sk attach filter).
- Cilium: `github.com/cilium/cilium/bpf/` — production XDP/TC programs for policy, load balancing, encryption.
- OpenFlow Switch Specification 1.5.1 — ONF TS-025, flow table structure, group tables, channel protocol.

## Ship It

The reusable artifact is an SDN controller simulation (Python) and an XDP-like packet counter (C). Both are standalone, dependency-free, and illustrate the core abstractions — flow tables with match-action semantics and BPF-map-based per-packet counters.

- `code/main.py` — SDN learning switch controller + 3-switch topology simulation
- `code/main.c` — Userspace eBPF/XDP protocol counter with flow tracking

See `outputs/README.md` for usage.

## Exercises

1. **Easy** — Add a flow rule to the SDN controller that drops all packets from a specific IP. Use `ctrl.install_flow()` with `actions=["drop"]` and verify matched packets return `["drop"]`.

2. **Medium** — Extend the C packet counter to count packets per TCP destination port instead of per protocol. Output the top 5 ports. Hint: add a BPF map (array indexed by port number).

3. **Hard** — Implement a full learning switch on the SDN controller: learn MAC-to-port mappings from PACKET_IN and install bidirectional flow rules. Extend `handle_packet_in` to install rules for both src and dst directions.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| SDN | A network with a central brain | Control plane on a separate server, data plane on switches; communicate via OpenFlow |
| OpenFlow | A way to program switches | Protocol defining flow table structure and controller-switch messages (PACKET_IN, FLOW_MOD) |
| eBPF | Running code in the kernel safely | Sandboxed bytecode verified by the kernel's BPF verifier, JIT-compiled and attached to hooks |
| XDP | The fastest packet processing in Linux | eBPF hook in the NIC driver, before SKB allocation; can drop/redirect at line rate |
| TC BPF | eBPF for traffic control | eBPF attached to the TC layer for classification, shaping, and mangling |
| Flow table | A routing table on steroids | Ordered (match, action, counters, priority) entries — the core OpenFlow data structure |
| Control plane | The brain of the network | Makes forwarding decisions; runs routing protocols, responds to topology changes |
| Data plane | The muscle of the network | Forwards packets based on flow table entries; must be fast and simple |
| BPF verifier | The eBPF safety net | Static analyzer checking for loops, invalid memory, type safety before loading |
| BPF map | Shared kernel-userspace memory | Hash, array, ring buffer accessible from eBPF programs and userspace via `bpf()` syscall |

## Further Reading

- OpenFlow Switch Specification 1.5.1 (ONF TS-025) — definitive reference for flow tables, groups, meters, and protocol.
- eBPF.io — documentation, tutorials, production users.
- "Systems Performance: Enterprise and the Cloud" 2nd Ed. by Brendan Gregg — eBPF tracing chapter.
- Cilium documentation at cilium.io — eBPF for Kubernetes networking, observability, security.
- "The Design of a Programmable Packet-Processing Pipeline" (P4 paper, SIGCOMM 2014) — match-action pipeline architecture.
