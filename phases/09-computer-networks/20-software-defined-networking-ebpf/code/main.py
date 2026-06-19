"""
Software-Defined Networking & eBPF
Phase 09 — Computer Networks

An SDN controller simulation with OpenFlow-like flow tables,
a learning switch, and a userspace eBPF/XDP packet counter.
"""


class FlowRule:
    """An OpenFlow-style flow table entry with match fields and actions."""

    def __init__(self, priority=0, match=None, actions=None, idle_timeout=0):
        self.priority = priority
        self.match = match or {}
        self.actions = actions or []
        self.idle_timeout = idle_timeout
        self.packet_count = 0
        self.byte_count = 0

    def matches(self, packet):
        """Return True if this flow rule matches the given packet."""
        for key, value in self.match.items():
            if packet.get(key) != value:
                return False
        return True

    def apply_actions(self, packet):
        """Execute actions for this rule; update counters."""
        self.packet_count += 1
        self.byte_count += packet.get("length", 0)
        return self.actions


class FlowTable:
    """A switch's flow table with priority-ordered rules."""

    def __init__(self):
        self.rules = []

    def add_rule(self, rule):
        self.rules.append(rule)
        self.rules.sort(key=lambda r: r.priority, reverse=True)

    def remove_rule(self, match):
        self.rules = [r for r in self.rules if r.match != match]

    def lookup(self, packet):
        """Return actions for highest-priority matching rule, or None."""
        for rule in self.rules:
            if rule.matches(packet):
                return rule.apply_actions(packet)
        return None


class OpenFlowSwitch:
    """An OpenFlow-capable switch with flow tables and controller connection."""

    def __init__(self, switch_id, controller=None):
        self.switch_id = switch_id
        self.flow_table = FlowTable()
        self.controller = controller
        self.ports = {}

    def add_port(self, port_id, name=""):
        self.ports[port_id] = {"name": name, "stats": {"rx_packets": 0, "tx_packets": 0}}

    def receive_packet(self, packet):
        """Process an incoming packet: flow lookup -> execute or PACKET_IN."""
        actions = self.flow_table.lookup(packet)
        if actions:
            return self._execute_actions(packet, actions)
        if self.controller:
            self.controller.handle_packet_in(self, packet)
            return ["PACKET_IN"]
        return []

    def _execute_actions(self, packet, actions):
        outputs = []
        for action in actions:
            if action == "flood":
                outputs = list(self.ports.keys())
            elif action == "drop":
                return ["drop"]
            elif action.startswith("output:"):
                port = int(action.split(":")[1])
                if port in self.ports:
                    outputs.append(port)
        for port in outputs:
            if port in self.ports:
                self.ports[port]["stats"]["tx_packets"] += 1
        return outputs


class SDNController:
    """An SDN controller managing switches and installing flow rules."""

    def __init__(self):
        self.switches = {}
        self._mac_table = {}

    def add_switch(self, sw):
        sw.controller = self
        self.switches[sw.switch_id] = sw

    def handle_packet_in(self, switch, packet):
        """Handle PACKET_IN: learn MAC, install flow rule for known dst, or flood."""
        src_mac = packet.get("src_mac")
        in_port = packet.get("in_port")
        self._mac_table[src_mac] = (switch.switch_id, in_port)

        dst_mac = packet.get("dst_mac")
        entry = self._mac_table.get(dst_mac)
        if entry is not None:
            _, out_port = entry
            rule = FlowRule(
                priority=100,
                match={"dst_mac": dst_mac, "in_port": in_port},
                actions=[f"output:{out_port}"],
            )
            switch.flow_table.add_rule(rule)
        else:
            rule = FlowRule(
                priority=10,
                match={"dst_mac": dst_mac},
                actions=["flood"],
            )
            switch.flow_table.add_rule(rule)

    def install_flow(self, switch_id, match, actions, priority=100):
        """Install a flow rule via the OpenFlow protocol simulation."""
        if switch_id in self.switches:
            rule = FlowRule(priority=priority, match=match, actions=actions)
            self.switches[switch_id].flow_table.add_rule(rule)

    def get_stats(self):
        """Return per-switch statistics."""
        stats = {}
        for sid, sw in self.switches.items():
            tx = sum(p["stats"]["tx_packets"] for p in sw.ports.values())
            stats[sid] = {
                "ports": {pid: p["stats"] for pid, p in sw.ports.items()},
                "flow_rules": len(sw.flow_table.rules),
                "packets_forwarded": tx,
            }
        return stats


class BPFPacketCounter:
    """Userspace simulation of an eBPF packet counter with BPF_MAP_TYPE_ARRAY."""

    def __init__(self):
        self.protocol_counts = {"TCP": 0, "UDP": 0, "ICMP": 0, "OTHER": 0}
        self.flow_stats = {}

    def parse_packet(self, raw_data):
        """Parse Ethernet/IP/transport headers from raw bytes (simulated eBPF parser)."""
        if len(raw_data) < 14:
            return None

        eth_type = int.from_bytes(raw_data[12:14], "big")
        if eth_type != 0x0800:
            return {"eth_type": eth_type, "protocol": "OTHER"}

        if len(raw_data) < 34:
            return None

        version_ihl = raw_data[14]
        ihl = (version_ihl & 0x0F) * 4
        if ihl < 20 or len(raw_data) < 14 + ihl:
            return None

        protocol = raw_data[23]
        src_ip = ".".join(str(b) for b in raw_data[26:30])
        dst_ip = ".".join(str(b) for b in raw_data[30:34])
        total_length = int.from_bytes(raw_data[16:18], "big")

        proto_map = {6: "TCP", 17: "UDP", 1: "ICMP"}
        proto_name = proto_map.get(protocol, "OTHER")

        result = {
            "eth_type": eth_type,
            "src_ip": src_ip,
            "dst_ip": dst_ip,
            "protocol": proto_name,
            "ip_header_len": ihl,
            "total_length": total_length,
        }

        if protocol in (6, 17) and len(raw_data) >= 14 + ihl + 4:
            offset = 14 + ihl
            result["src_port"] = int.from_bytes(raw_data[offset : offset + 2], "big")
            result["dst_port"] = int.from_bytes(raw_data[offset + 2 : offset + 4], "big")

        return result

    def process(self, parsed):
        """Update BPF map counters (simulated XDP action)."""
        if parsed is None:
            return "XDP_DROP"

        proto = parsed.get("protocol", "OTHER")
        self.protocol_counts[proto] = self.protocol_counts.get(proto, 0) + 1

        flow_key = (
            parsed.get("src_ip"),
            parsed.get("dst_ip"),
            proto,
            parsed.get("src_port", 0),
            parsed.get("dst_port", 0),
        )
        self.flow_stats[flow_key] = (
            self.flow_stats.get(flow_key, 0) + parsed.get("total_length", 0)
        )

        return "XDP_PASS"

    def get_protocol_stats(self):
        return dict(self.protocol_counts)

    def get_top_flows(self, n=5):
        sorted_flows = sorted(self.flow_stats.items(), key=lambda x: x[1], reverse=True)
        return sorted_flows[:n]


def _build_test_packets():
    """Build a list of raw byte-array test packets."""
    pkts = []

    def make_pkt(proto, src_ip, dst_ip, src_port=0, dst_port=0, extra_len=0):
        p = bytearray(64)
        p[12:14] = (0x0800).to_bytes(2, "big")
        p[14] = 0x45
        length = 40 + extra_len
        p[16:18] = length.to_bytes(2, "big")
        p[23] = proto
        for i, b in enumerate(src_ip.split(".")):
            p[26 + i] = int(b)
        for i, b in enumerate(dst_ip.split(".")):
            p[30 + i] = int(b)
        if src_port:
            p[34:36] = src_port.to_bytes(2, "big")
        if dst_port:
            p[36:38] = dst_port.to_bytes(2, "big")
        return bytes(p)

    # 20 TCP packets from various hosts to 10.0.0.1:5000
    for i in range(20):
        pkts.append(make_pkt(6, f"192.168.1.{i % 5 + 1}", "10.0.0.1", 80, 5000))

    # 10 UDP DNS packets
    for _ in range(10):
        pkts.append(make_pkt(17, "192.168.1.1", "8.8.8.8", 53, 12345))

    # 5 ICMP pings
    for _ in range(5):
        pkts.append(make_pkt(1, "192.168.1.1", "8.8.8.8"))

    # 3 IPv6 packets (non-IPv4, should be OTHER)
    for _ in range(3):
        p = bytearray(64)
        p[12:14] = (0x86DD).to_bytes(2, "big")
        pkts.append(bytes(p))

    return pkts


def simulate_topology():
    """Create a 3-switch topology with learning switch behavior."""
    ctrl = SDNController()

    s1 = OpenFlowSwitch("switch-1")
    s1.add_port(1, "host-a")
    s1.add_port(2, "switch-2")
    s1.add_port(3, "switch-3")

    s2 = OpenFlowSwitch("switch-2")
    s2.add_port(1, "switch-1")
    s2.add_port(2, "host-b")

    s3 = OpenFlowSwitch("switch-3")
    s3.add_port(1, "switch-1")
    s3.add_port(2, "host-c")

    ctrl.add_switch(s1)
    ctrl.add_switch(s2)
    ctrl.add_switch(s3)

    for sw in [s1, s2, s3]:
        sw.flow_table.add_rule(FlowRule(priority=0, match={}, actions=["flood"]))

    print("=== SDN Controller Simulation ===")
    print()
    print("Topology: host-a -- s1 -- s2 -- host-b")
    print("                          \\")
    print("                           s3 -- host-c")
    print()

    # First packet — table miss triggers PACKET_IN
    print("1. Host A sends ARP (first packet — table miss)")
    p1 = {
        "src_mac": "00:11:22:33:44:AA",
        "dst_mac": "FF:FF:FF:FF:FF:FF",
        "in_port": 1,
        "eth_type": 0x0806,
        "length": 64,
    }
    result = s1.receive_packet(p1)
    print(f"   Switch s1 action: {result}")
    print(f"   Controller learns MAC 00:11:22:33:44:AA on s1 port 1")

    # Install static flow rule
    print()
    print("2. Controller installs static flow rule: host A traffic -> port 2")
    ctrl.install_flow(
        "switch-1",
        match={"src_mac": "00:11:22:33:44:AA"},
        actions=["output:2"],
        priority=200,
    )
    print("   Flow rule installed")

    # Second packet hits the flow table directly
    print()
    print("3. Host A sends IPv4 — flow table hit (fast path)")
    p2 = {
        "src_mac": "00:11:22:33:44:AA",
        "dst_mac": "00:11:22:33:44:BB",
        "in_port": 1,
        "eth_type": 0x0800,
        "length": 512,
    }
    result = s1.receive_packet(p2)
    print(f"   Switch s1 action: {result}")

    # Stats
    print()
    print("4. Switch statistics:")
    stats = ctrl.get_stats()
    for sid, data in stats.items():
        print(f"   {sid}: {data['packets_forwarded']} pkts fwd, "
              f"{data['flow_rules']} flow rules")

    print()
    print("5. Flow rule traffic (packet counts):")
    for rule in s1.flow_table.rules:
        print(f"   Pri {rule.priority}: match={rule.match}, "
              f"count={rule.packet_count} pkts")


def simulate_ebpf():
    """Simulate eBPF XDP packet processing with protocol counting."""
    print()
    print("=== eBPF Packet Counter (XDP Simulation) ===")

    counter = BPFPacketCounter()
    test_packets = _build_test_packets()

    print()
    print(f"Processing {len(test_packets)} packets through simulated eBPF/XDP...")

    for pkt in test_packets:
        parsed = counter.parse_packet(pkt)
        counter.process(parsed)

    print()
    print("Protocol distribution (simulated BPF_MAP_TYPE_ARRAY):")
    for proto, count in counter.get_protocol_stats().items():
        bar = "#" * count
        print(f"  {proto:6s}: {count:3d} packets {bar}")

    print()
    print("Top flows by byte volume:")
    for (src, dst, proto, sp, dp), bytes_n in counter.get_top_flows(5):
        print(f"  {src}:{sp} -> {dst}:{dp} ({proto}): {bytes_n} bytes")


def main():
    simulate_topology()
    simulate_ebpf()


if __name__ == "__main__":
    main()
