# GoF Patterns That Still Matter — Quick Reference

## The 7 Patterns That Survive

| Pattern | One-Liner | When to Use | Modern Form |
|---------|-----------|-------------|-------------|
| **Observer** | "Notify N things without knowing who they are" | UI events, message buses, reactive streams | `EventEmitter`, RxJS `Observable`, signals |
| **Strategy** | "Swap an algorithm at runtime" | Payment methods, sorting, compression | DI-injected functions, Protocol/interface |
| **Command** | "Treat an operation as data" | Undo/redo, job queues, CQRS | Action objects, Redux actions |
| **Iterator** | "Walk a collection without exposing it" | Custom traversals, lazy sequences, async streams | Generators (`yield`), `Symbol.iterator` |
| **Factory Method** | "Create objects without knowing their class" | DI containers, test doubles, config-driven creation | Protocol/interface + factory class |
| **Decorator** | "Add behavior without modifying code" | Middleware, logging, caching, timing | Python `@decorator`, TS decorators, middleware chains |
| **Adapter** | "Convert one interface to another" | Legacy wrappers, API adapters, third-party integration | Wrapper class implementing target interface |

## Pattern Sketches (Copy-Paste Ready)

### Observer

```python
class EventEmitter:
    def __init__(self): self._listeners = {}
    def on(self, event, cb):
        self._listeners.setdefault(event, []).append(cb)
        return lambda: self._listeners[event].remove(cb)
    def emit(self, event, *a):
        for cb in list(self._listeners.get(event, [])): cb(*a)
```

### Strategy

```python
class PaymentStrategy(Protocol):
    def pay(self, amount: float) -> str: ...

class Processor:
    def __init__(self, strategy: PaymentStrategy): self._s = strategy
    def set_strategy(self, s): self._s = s
    def process(self, amt): return self._s.pay(amt)
```

### Command (with Undo)

```python
class Command:
    def execute(self): ...
    def undo(self): ...

class Editor:
    def __init__(self): self.content, self._hist, self._redo = "", [], []
    def execute(self, cmd): cmd.execute(); self._hist.append(cmd); self._redo.clear()
    def undo(self):
        if self._hist: c = self._hist.pop(); c.undo(); self._redo.append(c)
```

### Iterator

```python
def in_order(node):
    if node: yield from in_order(node.left); yield node.value; yield from in_order(node.right)
```

### Factory Method

```python
class Factory(Protocol):
    def create(self) -> Product: ...

class Service:
    def __init__(self, factory: Factory): self._factory = factory
    def act(self): self._factory.create().do_thing()
```

### Decorator

```python
def log_calls(fn):
    @functools.wraps(fn)
    def wrapper(*a, **kw):
        print(f"Calling {fn.__name__}"); result = fn(*a, **kw); return result
    return wrapper

@log_calls
def my_func(): ...
```

### Adapter

```python
class Adapter(TargetInterface):
    def __init__(self, adaptee: LegacyService): self._adaptee = adaptee
    def target_method(self):
        legacy = self._adaptee.legacy_method()
        return adapt(legacy)
```

## Patterns to SKIP (Use This Instead)

| Pattern | Replace With | Why |
|---------|-------------|-----|
| Singleton | DI | Hidden global state, untestable |
| Builder | Named params / kwargs | `Dataclass(**kwargs)`, object literals |
| Visitor | Pattern matching | `match`/`case`, discriminated unions |
| Flyweight | `functools.lru_cache` | Built-in memoization |
| Bridge | Protocols/interfaces | Language-level structural typing |
| Template Method | Strategy + DI | Composition over inheritance |
| State | FSM libraries | `python-statemachine`, XState |

## Decision Flowchart

```
Need to notify N listeners?          → Observer
Need to swap algorithms?             → Strategy
Need to undo/queue operations?       → Command
Need to traverse without exposing?   → Iterator
Need to create without knowing type? → Factory Method
Need to add behavior dynamically?    → Decorator
Need to bridge two interfaces?       → Adapter
Otherwise?                           → Probably don't need a pattern
```