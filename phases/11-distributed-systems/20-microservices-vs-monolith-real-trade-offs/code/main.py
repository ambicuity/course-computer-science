"""
Microservices vs Monolith — Real Trade-offs
Service mesh simulation: monolith, microservices, circuit breaker, strangler fig.
"""

import random
import time
import statistics
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Optional


# ── Monolith Simulation ─────────────────────────────────────────────────────

class MonolithSimulation:
    def __init__(self):
        self.db = {}
        self.modules = {
            "auth": lambda req: {"user_id": req.get("user_id", "u1"), "authenticated": True},
            "billing": lambda req: {"charged": True, "amount": req.get("amount", 10.0)},
            "inventory": lambda req: {"in_stock": True, "sku": req.get("sku", "SKU-1")},
            "search": lambda req: {"results": ["item1", "item2"], "query": req.get("query", "laptop")},
            "notifications": lambda req: {"sent": True, "channel": req.get("channel", "email")},
        }

    def handle_request(self, request: dict) -> dict:
        start = time.perf_counter()
        user = self.modules["auth"](request)
        billing = self.modules["billing"](request)
        inventory = self.modules["inventory"](request)
        search = self.modules["search"](request)
        notif = self.modules["notifications"](request)
        elapsed_ms = (time.perf_counter() - start) * 1000
        return {
            "result": {
                "auth": user,
                "billing": billing,
                "inventory": inventory,
                "search": search,
                "notifications": notif,
            },
            "latency_ms": round(elapsed_ms, 4),
            "hops": 1,
        }


# ── Circuit Breaker ─────────────────────────────────────────────────────────

class CircuitState(Enum):
    CLOSED = auto()
    OPEN = auto()
    HALF_OPEN = auto()


class CircuitBreaker:
    def __init__(self, failure_threshold: float = 0.5, cooldown_s: float = 1.0,
                 half_open_max_calls: int = 1):
        self.state = CircuitState.CLOSED
        self.failure_threshold = failure_threshold
        self.cooldown_s = cooldown_s
        self.half_open_max_calls = half_open_max_calls
        self.successes = 0
        self.failures = 0
        self.last_failure_time: Optional[float] = None
        self.half_open_calls = 0

    def can_execute(self) -> bool:
        if self.state == CircuitState.CLOSED:
            return True
        if self.state == CircuitState.OPEN:
            if time.time() - self.last_failure_time >= self.cooldown_s:
                self.state = CircuitState.HALF_OPEN
                self.half_open_calls = 0
                return True
            return False
        if self.state == CircuitState.HALF_OPEN:
            if self.half_open_calls < self.half_open_max_calls:
                self.half_open_calls += 1
                return True
            return False
        return False

    def record_success(self):
        self.successes += 1
        if self.state == CircuitState.HALF_OPEN:
            self.state = CircuitState.CLOSED
            self.successes = 0
            self.failures = 0

    def record_failure(self):
        self.failures += 1
        self.last_failure_time = time.time()
        if self.state == CircuitState.HALF_OPEN:
            self.state = CircuitState.OPEN
        elif self.state == CircuitState.CLOSED:
            total = self.successes + self.failures
            if total >= 5 and (self.failures / total) >= self.failure_threshold:
                self.state = CircuitState.OPEN

    @property
    def state_name(self) -> str:
        return self.state.name


# ─- Service Discovery ───────────────────────────────────────────────────────

class ServiceRegistry:
    def __init__(self):
        self._services: dict[str, list[str]] = {}

    def register(self, name: str, address: str):
        self._services.setdefault(name, []).append(address)

    def deregister(self, name: str, address: str):
        if name in self._services:
            self._services[name] = [a for a in self._services[name] if a != address]

    def discover(self, name: str) -> Optional[str]:
        instances = self._services.get(name)
        if not instances:
            return None
        return random.choice(instances)


# ─- Microservice ─────────────────────────────────────────────────────────────

@dataclass
class Microservice:
    name: str
    address: str
    latency_ms: float = 2.0
    failure_rate: float = 0.0
    db: dict = field(default_factory=dict)

    def handle(self, request: dict) -> dict:
        time.sleep(self.latency_ms / 1000.0)
        if random.random() < self.failure_rate:
            raise RuntimeError(f"{self.name}: service unavailable")
        return {"service": self.name, "status": "ok", "request": request}


# ─- Microservice Simulation ─────────────────────────────────────────────────

class MicroserviceSimulation:
    def __init__(self, network_latency_ms: float = 2.0, gateway_latency_ms: float = 0.5):
        self.registry = ServiceRegistry()
        self.circuit_breakers: dict[str, CircuitBreaker] = {}
        self.services: dict[str, list[Microservice]] = {}
        self.network_latency_ms = network_latency_ms
        self.gateway_latency_ms = gateway_latency_ms

    def add_service(self, name: str, latency_ms: float = 2.0,
                    failure_rate: float = 0.0, replicas: int = 1):
        instances = []
        for i in range(replicas):
            addr = f"{name}-{i}.svc.cluster.local"
            svc = Microservice(name=name, address=addr,
                               latency_ms=latency_ms, failure_rate=failure_rate)
            self.registry.register(name, addr)
            instances.append(svc)
        self.services[name] = instances
        self.circuit_breakers[name] = CircuitBreaker()

    def call_service(self, name: str, request: dict,
                     retries: int = 2, backoff_ms: float = 50.0) -> dict:
        cb = self.circuit_breakers[name]
        for attempt in range(retries + 1):
            if not cb.can_execute():
                return {"service": name, "status": "circuit_open", "request": request}
            addr = self.registry.discover(name)
            if addr is None:
                cb.record_failure()
                return {"service": name, "status": "not_found", "request": request}
            instances = self.services[name]
            svc = random.choice(instances)
            start = time.perf_counter()
            try:
                result = svc.handle(request)
                elapsed = (time.perf_counter() - start) * 1000
                cb.record_success()
                result["latency_ms"] = round(elapsed, 4)
                return result
            except RuntimeError:
                elapsed = (time.perf_counter() - start) * 1000
                cb.record_failure()
                if attempt < retries:
                    time.sleep((backoff_ms * (2 ** attempt)) / 1000.0)
                    continue
                return {"service": name, "status": "failed", "latency_ms": round(elapsed, 4),
                        "request": request}
        return {"service": name, "status": "exhausted_retries", "request": request}

    def handle_request(self, request: dict) -> dict:
        start = time.perf_counter()
        time.sleep(self.gateway_latency_ms / 1000.0)

        auth = self.call_service("auth", request)
        billing = self.call_service("billing", request)
        inventory = self.call_service("inventory", request)
        search = self.call_service("search", request)
        notif = self.call_service("notifications", request)

        elapsed = (time.perf_counter() - start) * 1000
        return {
            "result": {
                "auth": auth,
                "billing": billing,
                "inventory": inventory,
                "search": search,
                "notifications": notif,
            },
            "latency_ms": round(elapsed, 4),
            "hops": 5,
        }


# ── Strangler Fig ────────────────────────────────────────────────────────────

class StranglerFig:
    def __init__(self, monolith: MonolithSimulation,
                 microservices: MicroserviceSimulation):
        self.monolith = monolith
        self.microservices = microservices
        self.routes: dict[str, str] = {}

    def migrate_endpoint(self, endpoint_name: str):
        self.routes[endpoint_name] = "microservice"
        return self

    def revert_endpoint(self, endpoint_name: str):
        self.routes[endpoint_name] = "monolith"
        return self

    def handle_request(self, request: dict) -> dict:
        start = time.perf_counter()
        results = {}
        for endpoint in ["auth", "billing", "inventory", "search", "notifications"]:
            target = self.routes.get(endpoint, "monolith")
            if target == "microservice":
                svc_result = self.microservices.call_service(endpoint, request)
                results[endpoint] = svc_result
            else:
                handler = self.monolith.modules[endpoint]
                results[endpoint] = handler(request)
        elapsed = (time.perf_counter() - start) * 1000
        return {"result": results, "latency_ms": round(elapsed, 4)}


# ── Demos ────────────────────────────────────────────────────────────────────

def demo_latency_comparison():
    print("=" * 70)
    print("DEMO 1: Latency Comparison — Monolith vs. Microservices")
    print("=" * 70)

    monolith = MonolithSimulation()
    micro = MicroserviceSimulation(network_latency_ms=2.0, gateway_latency_ms=0.5)
    micro.add_service("auth", latency_ms=0.5, failure_rate=0.0, replicas=2)
    micro.add_service("billing", latency_ms=1.0, failure_rate=0.0, replicas=2)
    micro.add_service("inventory", latency_ms=0.8, failure_rate=0.0, replicas=2)
    micro.add_service("search", latency_ms=1.5, failure_rate=0.0, replicas=3)
    micro.add_service("notifications", latency_ms=0.5, failure_rate=0.0, replicas=1)

    request = {"user_id": "u42", "sku": "SKU-7", "query": "laptop", "amount": 99.99}

    mono_latencies = []
    micro_latencies = []

    for i in range(20):
        m_result = monolith.handle_request(request)
        mono_latencies.append(m_result["latency_ms"])

    for i in range(20):
        ms_result = micro.handle_request(request)
        micro_latencies.append(ms_result["latency_ms"])

    print(f"\nMonolith (20 requests):")
    print(f"  Mean latency: {statistics.mean(mono_latencies):.4f} ms")
    print(f"  Median:       {statistics.median(mono_latencies):.4f} ms")
    print(f"  p99:          {sorted(mono_latencies)[int(len(mono_latencies) * 0.99)]:.4f} ms")
    print(f"  Hops per request: 1 (in-process function calls)")

    print(f"\nMicroservices (20 requests, 5 services × ~2ms network):")
    print(f"  Mean latency: {statistics.mean(micro_latencies):.4f} ms")
    print(f"  Median:       {statistics.median(micro_latencies):.4f} ms")
    print(f"  p99:          {sorted(micro_latencies)[int(len(micro_latencies) * 0.99)]:.4f} ms")
    print(f"  Hops per request: 5 (network calls)")

    ratio = statistics.mean(micro_latencies) / max(statistics.mean(mono_latencies), 0.001)
    print(f"\nMicroservices are ~{ratio:.1f}× slower than monolith for this request")
    print("  (This is the fundamental latency tax of distribution.)")


def demo_circuit_breaker():
    print("\n" + "=" * 70)
    print("DEMO 2: Circuit Breaker in Action")
    print("=" * 70)

    micro = MicroserviceSimulation(network_latency_ms=0.5)
    micro.add_service("billing", latency_ms=0.5, failure_rate=0.7)

    print("\nSending 20 requests to billing service (70% failure rate):")
    print(f"{'Request':<10} {'CB State':<15} {'Result':<20}")
    print("-" * 45)

    cb = micro.circuit_breakers["billing"]
    for i in range(10):
        result = micro.call_service("billing", {"amount": 10.0}, retries=0)
        print(f"{i+1:<10} {cb.state_name:<15} {result.get('status', 'unknown'):<20}")

    print(f"\nFinal circuit breaker state: {cb.state_name}")
    print(f"Total successes: {cb.successes}, Total failures: {cb.failures}")

    if cb.state == CircuitState.OPEN:
        print("\nCircuit is OPEN — fast-failing all requests.")
        print("Waiting for cooldown period...")
        time.sleep(cb.cooldown_s + 0.1)
        can = cb.can_execute()
        print(f"After cooldown: circuit is {cb.state_name} (can_execute={can})")

        billing = micro.services["billing"][0]
        billing.failure_rate = 0.0
        print("  (Billing service is now healthy — failure_rate set to 0%)")

        result = micro.call_service("billing", {"amount": 10.0}, retries=0)
        print(f"  Probe request result: {result.get('status', 'unknown')}")
        print(f"  Circuit is now: {cb.state_name}")

        result = micro.call_service("billing", {"amount": 10.0}, retries=0)
        print(f"  Next request result: {result.get('status', 'unknown')}")
        print(f"  Circuit is now: {cb.state_name}")
        print("\n  Circuit breaker lifecycle:")
        print("  CLOSED → failures exceed threshold → OPEN (fast fail)")
        print("  OPEN → cooldown expires → HALF_OPEN (probe)")
        print("  HALF_OPEN → success → CLOSED (resume normal)")


def demo_strangler_fig():
    print("\n" + "=" * 70)
    print("DEMO 3: Strangler Fig Pattern — Gradual Migration")
    print("=" * 70)

    monolith = MonolithSimulation()
    micro = MicroserviceSimulation(network_latency_ms=2.0, gateway_latency_ms=0.5)
    micro.add_service("auth", latency_ms=0.5, failure_rate=0.0, replicas=1)
    micro.add_service("billing", latency_ms=1.0, failure_rate=0.0, replicas=1)
    micro.add_service("inventory", latency_ms=0.8, failure_rate=0.0, replicas=1)
    micro.add_service("search", latency_ms=1.5, failure_rate=0.0, replicas=1)
    micro.add_service("notifications", latency_ms=0.5, failure_rate=0.0, replicas=1)

    strangler = StranglerFig(monolith, micro)

    request = {"user_id": "u42", "sku": "SKU-7", "query": "laptop", "amount": 99.99}

    print("\nPhase 1: All routes → Monolith")
    result = strangler.handle_request(request)
    print(f"  Latency: {result['latency_ms']:.4f} ms")
    print(f"  All endpoints served by monolith (in-process)")

    print("\nPhase 2: Migrate auth → Microservice")
    strangler.migrate_endpoint("auth")
    result = strangler.handle_request(request)
    print(f"  Latency: {result['latency_ms']:.4f} ms")
    print(f"  auth → microservice, rest → monolith")

    print("\nPhase 3: Migrate billing → Microservice")
    strangler.migrate_endpoint("billing")
    result = strangler.handle_request(request)
    print(f"  Latency: {result['latency_ms']:.4f} ms")
    print(f"  auth, billing → microservice, rest → monolith")

    print("\nPhase 4: Migrate inventory → Microservice")
    strangler.migrate_endpoint("inventory")
    result = strangler.handle_request(request)
    print(f"  Latency: {result['latency_ms']:.4f} ms")

    print("\nPhase 5: Migrate search → Microservice")
    strangler.migrate_endpoint("search")
    result = strangler.handle_request(request)
    print(f"  Latency: {result['latency_ms']:.4f} ms")

    print("\nPhase 6: Migrate notifications → Microservice (full migration)")
    strangler.migrate_endpoint("notifications")
    result = strangler.handle_request(request)
    print(f"  Latency: {result['latency_ms']:.4f} ms")
    print(f"  All endpoints → microservices (monolith is now gone)")

    print("\nRevert: billing has a bug — redirect back to monolith")
    strangler.revert_endpoint("billing")
    result = strangler.handle_request(request)
    print(f"  Latency: {result['latency_ms']:.4f} ms")
    print(f"  billing → monolith (rollback), rest → microservices")

    print("\n  This is the power of the strangler fig: each endpoint")
    print("  can be migrated and reverted independently, in production.")


def demo_failure_isolation():
    print("\n" + "=" * 70)
    print("DEMO 4: Failure Isolation — Monolith vs. Microservices")
    print("=" * 70)

    monolith = MonolithSimulation()
    micro = MicroserviceSimulation(network_latency_ms=1.0, gateway_latency_ms=0.3)
    micro.add_service("auth", latency_ms=0.5, failure_rate=0.0)
    micro.add_service("billing", latency_ms=0.5, failure_rate=0.0)
    micro.add_service("inventory", latency_ms=0.5, failure_rate=0.0)
    micro.add_service("search", latency_ms=0.5, failure_rate=0.8)
    micro.add_service("notifications", latency_ms=0.5, failure_rate=0.0)

    print("\nMonolith scenario: one module crashes")
    print("  If ANY module throws an exception, the whole process dies.")
    print("  No partial recovery. The entire request fails.")

    print("\nMicroservice scenario: search service has 80% failure rate")
    print(f"{'Req':<5} {'auth':<8} {'billing':<10} {'inventory':<12} "
          f"{'search':<16} {'notif':<10} {'total_ok':<10}")
    print("-" * 71)

    total_ok_counts = []
    for i in range(10):
        result = micro.handle_request({"user_id": "u1", "query": "test"})
        r = result["result"]
        auth_status = r["auth"].get("status", "ok")
        bill_status = r["billing"].get("status", "ok")
        inv_status = r["inventory"].get("status", "ok")
        search_status = r["search"].get("status", "ok")
        notif_status = r["notifications"].get("status", "ok")
        ok_count = sum(1 for s in [auth_status, bill_status, inv_status,
                                     search_status, notif_status] if s == "ok")
        total_ok_counts.append(ok_count)
        print(f"{i+1:<5} {auth_status:<8} {bill_status:<10} {inv_status:<12} "
              f"{search_status:<16} {notif_status:<10} {ok_count}/5")

    avg_ok = sum(total_ok_counts) / len(total_ok_counts)
    print(f"\n  Average services succeeding per request: {avg_ok:.1f}/5")
    print("  Key insight: search failures are ISOLATED.")
    print("  Auth, billing, and inventory succeed even when search fails.")
    print("  In a monolith, ANY module crash kills the ENTIRE request.")


if __name__ == "__main__":
    random.seed(42)
    demo_latency_comparison()
    demo_circuit_breaker()
    demo_strangler_fig()
    demo_failure_isolation()
    print("\n" + "=" * 70)
    print("All demos complete.")
    print("=" * 70)