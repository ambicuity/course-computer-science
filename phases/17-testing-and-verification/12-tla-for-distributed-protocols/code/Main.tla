---- MODULE Main ----
EXTENDS Naturals

CONSTANT Nodes
ASSUME Nodes = {"A", "B"}

VARIABLES role, votes

Roles == {"Follower", "Candidate", "Leader"}

Init ==
  /\ role = [n \in Nodes |-> "Follower"]
  /\ votes = [n \in Nodes |-> {}]

Timeout(n) ==
  /\ n \in Nodes
  /\ role[n] = "Follower"
  /\ role' = [role EXCEPT ![n] = "Candidate"]
  /\ votes' = [votes EXCEPT ![n] = {n}]

GrantVote(voter, cand) ==
  /\ voter \in Nodes /\ cand \in Nodes
  /\ role[cand] = "Candidate"
  /\ votes' = [votes EXCEPT ![cand] = @ \cup {voter}]
  /\ role' = role

BecomeLeader(cand) ==
  /\ cand \in Nodes
  /\ role[cand] = "Candidate"
  /\ Cardinality(votes[cand]) >= 2
  /\ role' = [role EXCEPT ![cand] = "Leader"]
  /\ votes' = votes

Next ==
  \E n \in Nodes: Timeout(n) \/ BecomeLeader(n) \/
    (\E v \in Nodes: GrantVote(v, n))

AtMostOneLeader ==
  Cardinality({n \in Nodes : role[n] = "Leader"}) <= 1

TypeOK ==
  /\ \A n \in Nodes: role[n] \in Roles
  /\ \A n \in Nodes: votes[n] \subseteq Nodes

Spec == Init /\ [][Next]_<<role, votes>>

Inv == TypeOK /\ AtMostOneLeader

====
