#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List


@dataclass(frozen=True)
class User:
    user_id: str
    preferred_channel: str


class PreferenceStub:
    def __init__(self, mapping: Dict[str, str]) -> None:
        self.mapping = mapping

    def get_channel(self, user_id: str) -> str:
        return self.mapping.get(user_id, "email")


class AuditFake:
    def __init__(self) -> None:
        self.rows: List[str] = []

    def save(self, record: str) -> None:
        self.rows.append(record)


class SenderSpy:
    def __init__(self) -> None:
        self.sent_payloads: List[str] = []

    def send(self, channel: str, payload: str) -> None:
        self.sent_payloads.append(f"{channel}:{payload}")


class StrictSenderMock:
    def __init__(self, expected_correlation: str) -> None:
        self.expected_correlation = expected_correlation
        self.calls = 0

    def send(self, channel: str, payload: str) -> None:
        if self.expected_correlation not in payload:
            raise AssertionError("missing required correlation ID")
        self.calls += 1


class NotificationService:
    def __init__(self, pref_repo: PreferenceStub, sender, audit_repo: AuditFake) -> None:
        self.pref_repo = pref_repo
        self.sender = sender
        self.audit_repo = audit_repo

    def notify(self, user: User, message: str, correlation_id: str) -> None:
        channel = self.pref_repo.get_channel(user.user_id)
        payload = f"cid={correlation_id};msg={message}"
        self.sender.send(channel, payload)
        self.audit_repo.save(f"{user.user_id}|{channel}|{payload}")


def demo_spy_and_fake() -> None:
    prefs = PreferenceStub({"u-1": "sms"})
    sender = SenderSpy()
    audit = AuditFake()
    svc = NotificationService(prefs, sender, audit)

    svc.notify(User("u-1", "sms"), "build green", "abc-123")

    assert len(sender.sent_payloads) == 1
    assert sender.sent_payloads[0].startswith("sms:")
    assert len(audit.rows) == 1


def demo_strict_mock_contract() -> None:
    prefs = PreferenceStub({"u-2": "email"})
    sender = StrictSenderMock(expected_correlation="cid=req-77")
    audit = AuditFake()
    svc = NotificationService(prefs, sender, audit)

    svc.notify(User("u-2", "email"), "invoice sent", "req-77")
    assert sender.calls == 1


def main() -> None:
    demo_spy_and_fake()
    demo_strict_mock_contract()
    print("test doubles demo passed")


if __name__ == "__main__":
    main()
