:- initialization(main).

parent(alice, bob).
parent(bob, carol).
parent(carol, dave).

ancestor(X, Y) :- parent(X, Y).
ancestor(X, Y) :- parent(X, Z), ancestor(Z, Y).

main :-
  ( ancestor(alice, dave) -> writeln('alice is ancestor of dave') ; writeln('no') ),
  ( ancestor(bob, dave) -> writeln('bob is ancestor of dave') ; writeln('no') ),
  halt.
