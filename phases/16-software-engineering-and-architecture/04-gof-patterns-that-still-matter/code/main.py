"""
GoF Patterns That Still Matter — Python Implementations
Phase 16 — Software Engineering & Architecture

Seven patterns, each self-contained and runnable.
Run: python main.py
"""

from __future__ import annotations

import functools
import time
from dataclasses import dataclass, field
from typing import Protocol, Generator


# ── 1. Observer ──────────────────────────────────────────────────────────────

class EventEmitter:
    """Observer pattern with unsubscribe support."""

    def __init__(self) -> None:
        self._listeners: dict[str, list[callable]] = {}

    def on(self, event: str, callback: callable) -> callable:
        """Subscribe and return an unsubscribe function."""
        self._listeners.setdefault(event, []).append(callback)

        def unsubscribe() -> None:
            listeners = self._listeners.get(event, [])
            if callback in listeners:
                listeners.remove(callback)

        return unsubscribe

    def emit(self, event: str, *args, **kwargs) -> None:
        """Notify all listeners of an event."""
        for callback in list(self._listeners.get(event, [])):
            callback(*args, **kwargs)


def demo_observer() -> None:
    print("=== Observer ===")
    bus = EventEmitter()
    unsub_a = bus.on("price", lambda p: print(f"  Dashboard A: {p}"))
    bus.on("price", lambda p: print(f"  Dashboard B: {p}"))
    print("  First emit:")
    bus.emit("price", 142.50)
    unsub_a()
    print("  After unsubscribing A:")
    bus.emit("price", 143.00)


# ── 2. Strategy ──────────────────────────────────────────────────────────────

class PaymentStrategy(Protocol):
    def pay(self, amount: float) -> str: ...


@dataclass
class CreditCardPayment:
    card_number: str

    def pay(self, amount: float) -> str:
        return f"  Charged ${amount:.2f} to card ending {self.card_number[-4:]}"


@dataclass
class CryptoPayment:
    wallet: str

    def pay(self, amount: float) -> str:
        return f"  Transferred ${amount:.2f} from wallet {self.wallet[:8]}"


@dataclass
class PayPalPayment:
    email: str

    def pay(self, amount: float) -> str:
        return f"  Sent ${amount:.2f} via PayPal to {self.email}"


class PaymentProcessor:
    """Context that uses a strategy swappable at runtime."""

    def __init__(self, strategy: PaymentStrategy) -> None:
        self._strategy = strategy

    def set_strategy(self, strategy: PaymentStrategy) -> None:
        self._strategy = strategy

    def process(self, amount: float) -> str:
        return self._strategy.pay(amount)


def demo_strategy() -> None:
    print("\n=== Strategy ===")
    processor = PaymentProcessor(CreditCardPayment("4242424242424242"))
    print(processor.process(99.99))
    processor.set_strategy(CryptoPayment("0xABCDEF1234567890"))
    print(processor.process(49.99))
    processor.set_strategy(PayPalPayment("user@example.com"))
    print(processor.process(24.99))


# ── 3. Command (with Undo) ──────────────────────────────────────────────────

class Command:
    """Base command with execute and undo."""

    def execute(self) -> None:
        raise UnsupportedOperation

    def undo(self) -> None:
        raise UnsupportedOperation


class AddTextCommand(Command):
    def __init__(self, editor: "TextEditor", text: str, position: int) -> None:
        self._editor = editor
        self._text = text
        self._position = position

    def execute(self) -> None:
        content = self._editor.content
        self._editor.content = content[: self._position] + self._text + content[self._position :]

    def undo(self) -> None:
        content = self._editor.content
        end = self._position + len(self._text)
        self._editor.content = content[: self._position] + content[end:]


class DeleteTextCommand(Command):
    def __init__(self, editor: "TextEditor", length: int, position: int) -> None:
        self._editor = editor
        self._length = length
        self._position = position
        self._deleted = ""

    def execute(self) -> None:
        content = self._editor.content
        self._deleted = content[self._position : self._position + self._length]
        self._editor.content = content[: self._position] + content[self._position + self._length :]

    def undo(self) -> None:
        content = self._editor.content
        self._editor.content = content[: self._position] + self._deleted + content[self._position :]


class TextEditor:
    """Editor with undo/redo stack."""

    def __init__(self) -> None:
        self.content = ""
        self._history: list[Command] = []
        self._redo_stack: list[Command] = []

    def execute(self, command: Command) -> None:
        command.execute()
        self._history.append(command)
        self._redo_stack.clear()

    def undo(self) -> bool:
        if not self._history:
            return False
        cmd = self._history.pop()
        cmd.undo()
        self._redo_stack.append(cmd)
        return True

    def redo(self) -> bool:
        if not self._redo_stack:
            return False
        cmd = self._redo_stack.pop()
        cmd.execute()
        self._history.append(cmd)
        return True


def demo_command() -> None:
    print("\n=== Command ===")
    editor = TextEditor()
    editor.execute(AddTextCommand(editor, "Hello", 0))
    print(f"  After 'Hello': '{editor.content}'")
    editor.execute(AddTextCommand(editor, " World", 5))
    print(f"  After ' World': '{editor.content}'")
    editor.undo()
    print(f"  After undo: '{editor.content}'")
    editor.redo()
    print(f"  After redo: '{editor.content}'")
    editor.execute(DeleteTextCommand(editor, 5, 5))
    print(f"  After delete: '{editor.content}'")
    editor.undo()
    print(f"  After undo delete: '{editor.content}'")


# ── 4. Iterator ──────────────────────────────────────────────────────────────

class TreeNode:
    def __init__(self, value: int, left: "TreeNode | None" = None, right: "TreeNode | None" = None):
        self.value = value
        self.left = left
        self.right = right


def in_order(node: TreeNode | None) -> Generator[int, None, None]:
    """Iterator pattern via Python generator — in-order traversal."""
    if node is None:
        return
    yield from in_order(node.left)
    yield node.value
    yield from in_order(node.right)


def pre_order(node: TreeNode | None) -> Generator[int, None, None]:
    if node is None:
        return
    yield node.value
    yield from pre_order(node.left)
    yield from pre_order(node.right)


def demo_iterator() -> None:
    print("\n=== Iterator ===")
    root = TreeNode(4, TreeNode(2, TreeNode(1), TreeNode(3)), TreeNode(6, TreeNode(5), TreeNode(7)))
    print(f"  In-order:  {list(in_order(root))}")
    print(f"  Pre-order: {list(pre_order(root))}")


# ── 5. Factory Method ────────────────────────────────────────────────────────

class Notification(Protocol):
    def send(self, message: str) -> str: ...


class EmailNotification:
    def send(self, message: str) -> str:
        return f"  Email sent: {message}"


class SMSNotification:
    def send(self, message: str) -> str:
        return f"  SMS sent: {message}"


class PushNotification:
    def send(self, message: str) -> str:
        return f"  Push notification: {message}"


class NotificationFactory(Protocol):
    def create(self) -> Notification: ...


class EmailFactory:
    def create(self) -> Notification:
        return EmailNotification()


class SMSFactory:
    def create(self) -> Notification:
        return SMSNotification()


class PushFactory:
    def create(self) -> Notification:
        return PushNotification()


class NotificationService:
    """Uses a factory to create notifications — doesn't know the concrete type."""

    def __init__(self, factory: NotificationFactory) -> None:
        self._factory = factory

    def notify(self, message: str) -> str:
        notification = self._factory.create()
        return notification.send(message)


def demo_factory() -> None:
    print("\n=== Factory Method ===")
    for factory_cls in [EmailFactory, SMSFactory, PushFactory]:
        service = NotificationService(factory_cls())
        print(service.notify("Build succeeded!"))


# ── 6. Decorator ─────────────────────────────────────────────────────────────

def log_calls(fn: callable) -> callable:
    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        print(f"  [LOG] Calling {fn.__name__}")
        result = fn(*args, **kwargs)
        print(f"  [LOG] {fn.__name__} returned: {result}")
        return result

    return wrapper


def time_calls(fn: callable) -> callable:
    @functools.wraps(fn)
    def wrapper(*args, **kwargs):
        start = time.monotonic()
        result = fn(*args, **kwargs)
        elapsed = time.monotonic() - start
        print(f"  [TIME] {fn.__name__} took {elapsed:.4f}s")
        return result

    return wrapper


def cache_calls(fn: callable) -> callable:
    _cache: dict[tuple, object] = {}

    @functools.wraps(fn)
    def wrapper(*args):
        if args not in _cache:
            _cache[args] = fn(*args)
            print(f"  [CACHE] Miss for {fn.__name__}{args}")
        else:
            print(f"  [CACHE] Hit for {fn.__name__}{args}")
        return _cache[args]

    return wrapper


@log_calls
@time_calls
@cache_calls
def expensive_compute(n: int) -> int:
    """Simulate an expensive computation."""
    time.sleep(0.01)
    return n * n


def demo_decorator() -> None:
    print("\n=== Decorator ===")
    print(f"  Result: {expensive_compute(5)}")
    print(f"  Result (cached): {expensive_compute(5)}")
    print(f"  Result (new arg): {expensive_compute(7)}")


# ── 7. Adapter ───────────────────────────────────────────────────────────────

class UserRepository(Protocol):
    def get_user(self, user_id: str) -> dict: ...


@dataclass
class LegacyUser:
    user_id: str
    first_name: str
    last_name: str
    email_address: str


class LegacyAuthService:
    """A legacy system we can't modify."""

    def fetch_user(self, uid: str) -> LegacyUser:
        _db = {
            "1": LegacyUser("1", "Jane", "Doe", "jane@legacy.com"),
            "2": LegacyUser("2", "John", "Smith", "john@legacy.com"),
        }
        return _db[uid]


class LegacyUserAdapter:
    """Adapts LegacyAuthService to UserRepository interface."""

    def __init__(self, legacy_service: LegacyAuthService) -> None:
        self._service = legacy_service

    def get_user(self, user_id: str) -> dict:
        legacy = self._service.fetch_user(user_id)
        return {
            "id": legacy.user_id,
            "name": f"{legacy.first_name} {legacy.last_name}",
            "email": legacy.email_address,
        }


class ModernUserService:
    """The new system our code expects to work with."""

    def __init__(self, repo: UserRepository) -> None:
        self._repo = repo

    def greet(self, user_id: str) -> str:
        user = self._repo.get_user(user_id)
        return f"  Hello, {user['name']} ({user['email']})"


def demo_adapter() -> None:
    print("\n=== Adapter ===")
    legacy = LegacyAuthService()
    adapter = LegacyUserAdapter(legacy)
    service = ModernUserService(adapter)
    print(service.greet("1"))
    print(service.greet("2"))


# ── Main ─────────────────────────────────────────────────────────────────────

def main() -> None:
    demo_observer()
    demo_strategy()
    demo_command()
    demo_iterator()
    demo_factory()
    demo_decorator()
    demo_adapter()


if __name__ == "__main__":
    main()