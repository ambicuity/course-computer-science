---- MODULE Main ----
EXTENDS Naturals

VARIABLE c

Init == c = 0

Inc == c' = c + 1

Read == c' = c

Next == Inc \/ Read

TypeOK == c \in Nat
Monotonic == c >= 0

Spec == Init /\ [][Next]_c
Inv == TypeOK /\ Monotonic

====
