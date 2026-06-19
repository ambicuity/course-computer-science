# GoF Patterns That Still Matter

> The Gang of Four catalogued 23 design patterns in 1994. Half of them are obsolete or actively harmful today. This lesson covers the 7 that remain essential and explains why the rest can be replaced.

**Type:** Learn
**Languages:** TypeScript, Python
**Prerequisites:** Phase 16 lessons 01–03
**Time:** ~75 minutes

## Learning Objectives

- Identify which GoF patterns remain relevant in modern codebases and why.
- Implement Observer, Strategy, Command, Iterator, Factory Method, Decorator, and Adapter from scratch.
- Recognize when a "classic" pattern is better replaced by a language feature or simpler construct.
- Apply these patterns with dependency injection and type safety rather than 1990s-style class hierarchies.

## The Problem

This lesson sits in **Phase 16 — Software Engineering & Architecture**. Without understanding which patterns actually matter, you'll either over-engineer (applying Singleton everywhere) or under-engineer (writing brittle `if/else` chains where Strategy or Command would clarify intent). The patterns that survive do so because they solve problems that language features alone don't: decoupling producers from consumers, encapsulating variation, and modeling operations as data.

The capstone for this phase is refactoring a real-world OSS repo with ADRs. You need to know which patterns to reach for and, just as importantly, which to avoid.

## Mental Model

Think of patterns as **communication devices**, not construction kits. A pattern's value is proportional to how often real code needs the abstraction it provides. The 7 patterns below solve problems that appear in every non-trivial codebase:

| Pattern | Core Problem | Modern Incarnation |
|---------|-------------|-------------------|
| Observer | "I need to notify N things when something happens, without knowing who they are." | Event emitters, RxJS, signals |
| Strategy | "I need to swap an algorithm at runtime without touching the caller." | DI-injected functions, policy objects |
| Command | "I need to treat an operation as data — store it, queue it, undo it." | CQRS commands, job queues, undo stacks |
| Iterator | "I need to walk a collection without exposing its internals." | Python generators, JS `Symbol.iterator` |
| Factory Method | "I need to create objects without knowing their concrete class." | DI containers, test doubles, generic type params |
| Decorator | "I need to add behavior to an object without modifying its code." | Middleware, Python `@decorator`, TS decorators |
| Adapter | "I need two incompatible interfaces to work together." | API wrappers, legacy integration layers |

## Build It

Each pattern below has a minimal version (the essence) and a realistic version (what you'd ship).

---

### 1. Observer

**Problem:** A stock ticker needs to notify multiple dashboards when a price changes — without the ticker knowing or caring which dashboards exist.

**Minimal version (Python):**

```python
class EventEmitter:
    def __init__(self):
        self._listeners = {}

    def on(self, event, callback):
        self._listeners.setdefault(event, []).append(callback)

    def emit(self, event, *args):
        for cb in self._listeners.get(event, []):
            cb(*args)

ticker = EventEmitter()
ticker.on("price", lambda p: print(f"Dashboard A: {p}"))
ticker.emit("price", 142.50)
```

**Realistic version (TypeScript):**

```typescript
type Listener<T> = (data: T) => void;

class EventBus {
  private listeners = new Map<string, Set<Listener<unknown>>>();

  on<T>(event: string, fn: Listener<T>): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    this.listeners.get(event)!.add(fn as Listener<unknown>);
    return () => this.listeners.get(event)?.delete(fn as Listener<unknown>);
  }

  emit<T>(event: string, data: T): void {
    this.listeners.get(event)?.forEach(fn => fn(data));
  }
}

const bus = new EventBus();
const unsub = bus.on<number>("price", p => console.log(`Dashboard: ${p}`));
bus.emit("price", 142.50);
unsub();
```

**Key insight:** The unsubscribe function (`unsub()`) is what the minimal version lacks — and it's critical in production to avoid memory leaks.

---

### 2. Strategy

**Problem:** A payment processor supports credit cards, PayPal, and crypto. You want to add new methods without modifying the processor.

```python
from dataclasses import dataclass
from typing import Protocol

class PaymentStrategy(Protocol):
    def pay(self, amount: float) -> str: ...

@dataclass
class CreditCardPayment:
    name: str
    def pay(self, amount: float) -> str:
        return f"Charged ${amount:.2f} to card ending {self.name[-4:]}"

@dataclass
class CryptoPayment:
    wallet: str
    def pay(self, amount: float) -> str:
        return f"Transferred ${amount:.2f} from wallet {self.wallet[:8]}"

class PaymentProcessor:
    def __init__(self, strategy: PaymentStrategy):
        self._strategy = strategy

    def set_strategy(self, strategy: PaymentStrategy):
        self._strategy = strategy

    def process(self, amount: float) -> str:
        return self._strategy.pay(amount)

processor = PaymentProcessor(CreditCardPayment("1234567890123456"))
print(processor.process(99.99))  # Charged $99.99 to card ending 3456
processor.set_strategy(CryptoPayment("0xABCDEF1234567890"))
print(processor.process(49.99))  # Transferred $49.99 from wallet 0xABCDEF
```

**Key insight:** In modern Python, `Protocol` defines the interface — no inheritance needed. The strategies are just objects with the right method. TypeScript uses `interface` the same way.

---

### 3. Command (with Undo)

**Problem:** A text editor needs undo/redo. Each operation must be an object that knows how to execute *and* reverse itself.

```python
class Command:
    def execute(self) -> None: ...
    def undo(self) -> None: ...

class AddTextCommand(Command):
    def __init__(self, editor, text, position):
        self.editor = editor
        self.text = text
        self.position = position

    def execute(self):
        self.editor.content = (
            self.editor.content[:self.position] + self.text + self.editor.content[self.position:]
        )

    def undo(self):
        self.editor.content = (
            self.editor.content[:self.position] + self.editor.content[self.position + len(self.text):]
        )

class Editor:
    def __init__(self):
        self.content = ""
        self._history = []
        self._redo_stack = []

    def execute(self, command: Command):
        command.execute()
        self._history.append(command)
        self._redo_stack.clear()

    def undo(self):
        if self._history:
            cmd = self._history.pop()
            cmd.undo()
            self._redo_stack.append(cmd)

    def redo(self):
        if self._redo_stack:
            cmd = self._redo_stack.pop()
            cmd.execute()
            self._history.append(cmd)
```

**Key insight:** Commands are data. You can serialize them, queue them, or replay them — that's why CQRS and event sourcing build on this pattern.

---

### 4. Iterator

**Problem:** Walk a binary tree in-order without exposing the tree's node structure.

```python
class TreeNode:
    def __init__(self, value, left=None, right=None):
        self.value = value
        self.left = left
        self.right = right

def in_order(node):
    if node is None:
        return
    yield from in_order(node.left)
    yield node.value
    yield from in_order(node.right)

root = TreeNode(4, TreeNode(2, TreeNode(1), TreeNode(3)), TreeNode(6, TreeNode(5), TreeNode(7)))
for val in in_order(root):
    print(val)  # 1, 2, 3, 4, 5, 6, 7
```

**Key insight:** Python generators make iterators trivial. In TypeScript, use `Symbol.iterator` or `async function*` for the same effect. The consumer never sees `TreeNode`.

---

### 5. Factory Method

**Problem:** A notification service creates alerts without knowing whether they'll be email, SMS, or push — the decision is made by configuration, not `if/else`.

```typescript
interface Notification {
  send(message: string): string;
}

interface NotificationFactory {
  create(): Notification;
}

class EmailNotification implements Notification {
  send(message: string): string {
    return `Email sent: ${message}`;
  }
}

class SMSNotification implements Notification {
  send(message: string): string {
    return `SMS sent: ${message}`;
  }
}

class EmailFactory implements NotificationFactory {
  create(): Notification { return new EmailNotification(); }
}

class SMSFactory implements NotificationFactory {
  create(): Notification { return new SMSNotification(); }
}

class NotificationService {
  constructor(private factory: NotificationFactory) {}

  notify(message: string): string {
    const notification = this.factory.create();
    return notification.send(message);
  }
}
```

**Key insight:** The factory lets you swap implementations for tests (mock factory) or configuration (env-based factory) without touching `NotificationService`. In practice, DI containers automate this.

---

### 6. Decorator

**Problem:** Add logging, caching, and timing to a data fetcher without modifying its source.

```python
import time
import functools

def log_calls(fn):
    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        print(f"Calling {fn.__name__}")
        result = fn(*args, **kwargs)
        print(f"{fn.__name__} returned {result}")
        return result
    return wrapper

def time_calls(fn):
    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        start = time.monotonic()
        result = fn(*args, **kwargs)
        print(f"{fn.__name__} took {time.monotonic() - start:.4f}s")
        return result
    return wrapper

@log_calls
@time_calls
def fetch_data(url: str) -> str:
    time.sleep(0.1)
    return f"data from {url}"

fetch_data("https://api.example.com")
# Calling fetch_data
# fetch_data took 0.1003s
# fetch_data returned data from https://api.example.com
```

**Key insight:** Decorators compose. You can stack `@log_calls`, `@time_calls`, `@cache` in any order. Each wraps the previous one. This is middleware in a nutshell.

---

### 7. Adapter

**Problem:** Your app uses a modern `UserRepository` interface, but the legacy auth system returns `LegacyUser` objects with a different shape.

```python
from dataclasses import dataclass
from typing import Protocol

class UserRepository(Protocol):
    def get_user(self, user_id: str) -> dict: ...

@dataclass
class LegacyUser:
    user_id: str
    first_name: str
    last_name: str
    email_address: str

class LegacyAuthService:
    def fetch_user(self, uid: str) -> LegacyUser:
        return LegacyUser(uid, "Jane", "Doe", "jane@example.com")

class LegacyUserAdapter:
    """Adapts LegacyAuthService to the UserRepository interface."""

    def __init__(self, legacy_service: LegacyAuthService):
        self._service = legacy_service

    def get_user(self, user_id: str) -> dict:
        legacy = self._service.fetch_user(user_id)
        return {
            "id": legacy.user_id,
            "name": f"{legacy.first_name} {legacy.last_name}",
            "email": legacy.email_address,
        }

def client_code(repo: UserRepository, uid: str):
    user = repo.get_user(uid)
    print(f"User: {user['name']} ({user['email']})")

legacy = LegacyAuthService()
adapter = LegacyUserAdapter(legacy)
client_code(adapter, "123")  # User: Jane Doe (jane@example.com)
```

**Key insight:** The adapter wraps the legacy API and translates it to the interface your code expects. The client never sees `LegacyUser`.

---

## Patterns That DON'T Matter Anymore

The original GoF book included 23 patterns. Here's why most of the remaining ones are better replaced by language features or simpler approaches:

| Pattern | Why It's Obsolete | What to Use Instead |
|---------|-------------------|---------------------|
| **Singleton** | Hidden global state, impossible to test, concurrency issues | Dependency injection — pass the instance where needed |
| **Builder** | Named parameters (Python kwargs, TypeScript inline objects) make it unnecessary | `Dataclass(**kwargs)` or object literals with type checking |
| **Visitor** | Double dispatch is now expressible via pattern matching | `match`/`case` in Python 3.10+, TypeScript discriminated unions |
| **Flyweight** | Language runtimes memoize strings; caching libraries memoize computations | `functools.lru_cache`, memoization utilities |
| **Bridge** | Interfaces (Python Protocols, TypeScript interfaces) separate abstraction from implementation natively | Just define an interface and implement it |
| **Facade** | Still useful but too simple to call a "pattern" — it's just a well-named function | A function or class that composes subsystem calls |
| **Memento** | Serialization libraries and immutability make this trivial | `dataclasses.replace()` or spread operators |
| **Prototype** | `copy.deepcopy()` or structured clone APIs handle this | `copy.deepcopy()`, `structuredClone()` in JS |
| **Chain of Responsibility** | Middleware pipelines are a more composable replacement | Express/Koa middleware, ASGI middleware |
| **Mediator** | Event buses or message brokers do this better at scale | Event bus, message queue, actor model |
| **Template Method** | Composition over inheritance — use Strategy instead | Strategy pattern with DI |
| **State** | Finite state machine libraries are more robust | `python-statemachine`, XState in TS |
| **Abstract Factory** | Factory Method + configuration usually suffices | Factory Method with DI |

**The through-line:** If a "pattern" exists because a language lacked a feature (like named params for Builder, or pattern matching for Visitor), the pattern is the language smell, not the solution.

## Use It

### Production equivalents

| Pattern | Where you'll see it in production code |
|---------|---------------------------------------|
| Observer | Node.js `EventEmitter`, Python `blinker`, RxJS `Observable`, DOM events, Kafka consumers |
| Strategy | Express middleware stacks, Django authentication backends, payment gateway integrations |
| Command | Redux actions, CQRS command objects, undo/redo in editors, Celery task messages |
| Iterator | Python `__iter__`/`__next__`, ES6 `Symbol.iterator`, async iterators, Rust `Iterator` trait |
| Factory Method | Django's `get_queryset()`, SQLAlchemy model factories, DI containers (InversifyJS, inject) |
| Decorator | Python `@decorator` syntax, TypeScript experimental decorators, Express middleware, NestJS guards |
| Adapter | Repository pattern wrappers, API client adapters, ORM dialect adapters, legacy system wrappers |

### Read the Source

- **Observer:** Node.js `events` module — [`lib/events.js`](https://github.com/nodejs/node/blob/main/lib/events.js). Look at `EventEmitter.prototype.on` and `emit`.
- **Command:** Redux — [`packages/redux/src/createStore.ts`](https://github.com/reduxjs/redux/blob/master/packages/redux/src/createStore.ts). Actions are command objects.
- **Iterator:** Python CPython — [`Objects/iterobject.c`](https://github.com/python/cpython/blob/main/Objects/iterobject.c). Reference iterator implementation.
- **Factory Method:** Django — [`django/db/models/manager.py`](https://github.com/django/django/blob/main/django/db/models/manager.py). `Manager._queryset_class` is a factory hook.
- **Decorator:** Python `functools` — [`Lib/functools.py`](https://github.com/python/cpython/blob/main/Lib/functools.py). `wraps` and `update_wrapper`.
- **Adapter:** SQLAlchemy — [`lib/sqlalchemy/engine/adapters.py`](https://github.com/sqlalchemy/sqlalchemy/). DB-API adapters.

## Ship It

The reusable artifact for this lesson is in `outputs/README.md` — a one-page reference card you can print or keep open while coding.

## Exercises

1. **Easy** — Implement the Observer pattern from memory in Python or TypeScript. Add unsubscribe support.
2. **Medium** — Build a Command pattern with undo/redo for a drawing canvas (commands: draw rectangle, draw circle, move shape). Each command must be reversible.
3. **Hard** — Implement a generic `MiddlewareChain<T>` that uses the Decorator pattern to compose processing stages. Then use it to build a request pipeline that logs, authenticates, and rate-limits — with each stage swappable at runtime via Strategy.

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| Observer | "Pub/sub" or "events" | A one-to-many dependency where changes in a subject automatically propagate to registered listeners |
| Strategy | "Policy pattern" | Encapsulating interchangeable algorithms behind a common interface, swappable at runtime |
| Command | "Action object" | Treating an operation as a first-class object — storable, queuable, reversible |
| Iterator | "Traversal" | A protocol for sequential access to a collection's elements without exposing its structure |
| Factory Method | "Virtual constructor" | Deferring object creation to a method that subclasses or configuration can override |
| Decorator | "Wrapper pattern" | Attaching additional responsibilities to an object dynamically, without modifying its class |
| Adapter | "Wrapper" (different meaning) | Converting one interface into another that a client expects, enabling incompatible systems to cooperate |

## Further Reading

- *Design Patterns: Elements of Reusable Object-Oriented Software* (GoF, 1994) — the original, but read with a critical eye
- *Game Programming Patterns* by Robert Nystrom (2014) — modern, practical take on patterns
- Brian Kernighan: "Everyone knows that debugging is twice as hard as writing a program in the first place. So if you're as clever as you can be when you write it, how will you ever debug it?" — patterns should make code *dumber and clearer*, not cleverer
- Python `typing.Protocol` PEP 544 — structural subtyping replaces inheritance hierarchies
- TypeScript Handbook on Generics — type-safe factories without `any`