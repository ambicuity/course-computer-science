---- MODULE Main ----
EXTENDS Naturals

CONSTANTS Clients
ASSUME Clients = {"A", "B"}

VARIABLE owner, req

Init ==
  /\ owner = "none"
  /\ req = {}

Request(c) ==
  /\ c \in Clients
  /\ req' = req \cup {c}
  /\ owner' = owner

Grant(c) ==
  /\ c \in req
  /\ owner = "none"
  /\ owner' = c
  /\ req' = req \ {c}

Release(c) ==
  /\ owner = c
  /\ owner' = "none"
  /\ req' = req

Next ==
  \E c \in Clients: Request(c) \/ Grant(c) \/ Release(c)

MutualExclusion == owner \in {"none"} \cup Clients

Spec == Init /\ [][Next]_<<owner, req>>

Inv == owner \in {"none"} \cup Clients

====
