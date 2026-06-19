"""
Two-Phase Commit (2PC) — coordinator, participants, crash simulation, and recovery.
"""

import logging
import time
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Optional

logging.basicConfig(level=logging.INFO, format="%(name)s: %(message)s")
log = logging.getLogger("2pc")


class ParticipantState(Enum):
    INIT = auto()
    PREPARED = auto()
    COMMITTED = auto()
    ABORTED = auto()


class Vote(Enum):
    YES = auto()
    NO = auto()


class Decision(Enum):
    COMMIT = auto()
    ABORT = auto()


@dataclass
class ParticipantLog:
    vote: Optional[Vote] = None
    decision: Optional[Decision] = None


class Participant:
    def __init__(self, name: str, will_vote: Vote = Vote.YES):
        self.name = name
        self.state = ParticipantState.INIT
        self.will_vote = will_vote
        self.log = ParticipantLog()
        self.simulate_crash_after_prepare = False

    def receive_prepare(self) -> Vote:
        vote = self.will_vote
        self.log.vote = vote
        if vote == Vote.YES:
            self.state = ParticipantState.PREPARED
            log.info("  %s votes YES (PREPARED)", self.name)
        else:
            self.state = ParticipantState.ABORTED
            log.info("  %s votes NO (ABORTED)", self.name)
        if self.simulate_crash_after_prepare:
            raise RuntimeError(f"CRASH: {self.name} crashed after voting!")
        return vote

    def receive_decision(self, decision: Decision):
        self.log.decision = decision
        if decision == Decision.COMMIT:
            self.state = ParticipantState.COMMITTED
            log.info("  %s COMMIT", self.name)
        else:
            self.state = ParticipantState.ABORTED
            log.info("  %s ABORT (rollback)", self.name)

    def recover(self):
        if self.log.vote == Vote.YES and self.log.decision is None:
            log.info("  %s recovering: voted YES but no decision — BLOCKED", self.name)
            return
        if self.log.decision is not None:
            decision = self.log.decision
            self.state = (
                ParticipantState.COMMITTED
                if decision == Decision.COMMIT
                else ParticipantState.ABORTED
            )
            log.info("  %s recovering: decision is %s, state=%s", self.name, decision.name, self.state.name)


@dataclass
class CoordinatorLog:
    decision: Optional[Decision] = None


class Coordinator:
    def __init__(self, name: str = "coordinator"):
        self.name = name
        self.log = CoordinatorLog()
        self.simulate_crash_after_phase1 = False
        self.simulate_crash_during_phase1 = False

    def phase1_prepare(self, participants: list[Participant]) -> Decision:
        log.info("Phase 1: PREPARE — coordinator asking participants to vote")
        all_yes = True
        for p in participants:
            try:
                vote = p.receive_prepare()
                if vote == Vote.NO:
                    all_yes = False
                    log.info("  %s voted NO → will abort", p.name)
            except RuntimeError:
                log.info("  %s CRASHED during prepare!", p.name)
                all_yes = False

        if self.simulate_crash_during_phase1:
            raise RuntimeError(f"CRASH: {self.name} crashed during Phase 1 before deciding!")

        decision = Decision.COMMIT if all_yes else Decision.ABORT
        self.log.decision = decision
        log.info(
            "Coordinator decision: %s",
            "COMMIT" if decision == Decision.COMMIT else "ABORT",
        )

        if self.simulate_crash_after_phase1:
            raise RuntimeError(f"CRASH: {self.name} crashed after logging decision but before Phase 2!")

        return decision

    def phase2_commit(self, participants: list[Participant], decision: Decision):
        log.info(
            "Phase 2: %s — coordinator sending decision to participants",
            decision.name,
        )
        for p in participants:
            if p.state not in (ParticipantState.PREPARED, ParticipantState.ABORTED):
                if decision == Decision.ABORT and p.state == ParticipantState.INIT:
                    p.receive_decision(decision)
                    continue
            p.receive_decision(decision)


class TwoPhaseCommit:
    def __init__(self, coordinator: Coordinator, participants: list[Participant]):
        self.coordinator = coordinator
        self.participants = participants

    def run(self) -> Decision:
        try:
            decision = self.coordinator.phase1_prepare(self.participants)
            self.coordinator.phase2_commit(self.participants, decision)
            return decision
        except RuntimeError:
            log.info("Coordinator crashed — participants are BLOCKED waiting for decision")
            return Decision.ABORT

    def recover(self) -> Optional[Decision]:
        if self.coordinator.log.decision is not None:
            decision = self.coordinator.log.decision
            log.info("Recovery: coordinator has logged decision: %s", decision.name)
            for p in self.participants:
                if p.state == ParticipantState.PREPARED:
                    p.receive_decision(decision)
            return decision
        log.info("Recovery: coordinator has no logged decision — cannot recover yet")
        return None


def demo_all_vote_yes():
    log.info("=" * 60)
    log.info("DEMO 1: All participants vote YES → COMMIT")
    log.info("=" * 60)
    p1 = Participant("inventory")
    p2 = Participant("billing")
    p3 = Participant("shipping")
    coord = Coordinator()
    txn = TwoPhaseCommit(coord, [p1, p2, p3])
    result = txn.run()
    log.info("Result: %s\n", result.name)


def demo_one_votes_no():
    log.info("=" * 60)
    log.info("DEMO 2: One participant votes NO → ABORT")
    log.info("=" * 60)
    p1 = Participant("inventory", Vote.YES)
    p2 = Participant("billing", Vote.NO)
    p3 = Participant("shipping", Vote.YES)
    coord = Coordinator()
    txn = TwoPhaseCommit(coord, [p1, p2, p3])
    result = txn.run()
    log.info("Result: %s\n", result.name)


def demo_coordinator_crash_and_recovery():
    log.info("=" * 60)
    log.info("DEMO 3: Coordinator crashes between Phase 1 and Phase 2, then recovers")
    log.info("=" * 60)
    p1 = Participant("inventory")
    p2 = Participant("billing")
    p3 = Participant("shipping")
    coord = Coordinator()
    coord.simulate_crash_after_phase1 = True
    txn = TwoPhaseCommit(coord, [p1, p2, p3])
    result = txn.run()
    log.info("After crash, result: %s", result.name)
    log.info("Participant states: %s, %s, %s", p1.state.name, p2.state.name, p3.state.name)
    log.info("")
    log.info("Simulating coordinator recovery...")
    coord.simulate_crash_after_phase1 = False
    recovered = txn.recover()
    if recovered:
        log.info("Recovery successful, decision: %s", recovered.name)
    log.info("Participant states after recovery: %s, %s, %s", p1.state.name, p2.state.name, p3.state.name)
    log.info("")


def demo_participant_blocked():
    log.info("=" * 60)
    log.info("DEMO 4: Coordinator crash BEFORE logging decision — participants BLOCKED")
    log.info("=" * 60)
    p1 = Participant("inventory")
    p2 = Participant("billing")
    p3 = Participant("shipping")
    coord = Coordinator()
    coord.simulate_crash_during_phase1 = True
    txn = TwoPhaseCommit(coord, [p1, p2, p3])
    result = txn.run()
    log.info("After crash: result=%s", result.name)
    log.info("Coordinator log decision: %s", coord.log.decision)
    log.info("Participant states: %s, %s, %s", p1.state.name, p2.state.name, p3.state.name)
    log.info("")
    log.info("Attempting recovery with fresh coordinator (no decision logged)...")
    recovered = txn.recover()
    if recovered is None:
        log.info("Cannot recover — no decision was logged. Participants remain BLOCKED.")
    log.info("Participants remain in states: %s, %s, %s\n", p1.state.name, p2.state.name, p3.state.name)


if __name__ == "__main__":
    demo_all_vote_yes()
    demo_one_votes_no()
    demo_coordinator_crash_and_recovery()
    demo_participant_blocked()