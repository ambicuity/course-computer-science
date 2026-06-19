%%%-------------------------------------------------------------------
%%% The Actor Model — Ping-Pong, Stateful Counter, Supervisor Demo
%%% Phase 13, Lesson 15
%%%
%%% Compile:  erlc main.erl
%%% Run:      erl -noshell -s main start
%%%-------------------------------------------------------------------
-module(main).
-export([start/0]).

%% ===================================================================
%% Section 1 — Ping-Pong Actors
%% ===================================================================

%% ping(N, PongPid)
%%   Sends N ping messages, expecting a pong reply for each.
%%   Sends 'finished' when done.
ping(0, _PongPid) ->
    io:format("[ping]  all done~n");
ping(N, PongPid) when N > 0 ->
    PongPid ! {ping, self()},
    receive
        pong ->
            io:format("[ping]  received pong (~p left)~n", [N - 1]),
            ping(N - 1, PongPid)
    after 2000 ->
        io:format("[ping]  TIMEOUT waiting for pong~n")
    end.

%% pong()
%%   Infinite loop: waits for {ping, From} or 'finished'.
pong() ->
    receive
        {ping, PingPid} ->
            io:format("[pong]  received ping, sending pong~n"),
            PingPid ! pong,
            pong();
        finished ->
            io:format("[pong]  received finished, exiting~n")
    end.

%% ===================================================================
%% Section 2 — Stateful Counter Actor
%% ===================================================================

%% counter(Count)
%%   Maintains an integer count. Understands:
%%     {increment, N}   — add N to count
%%     {decrement, N}   — subtract N from count
%%     {multiply,  N}   — multiply count by N
%%     {divide,    N}   — divide count by N (crash if N == 0)
%%     {get, From}      — reply with {count, Count}
%%     {set,    N}      — set count to N
%%     {batch,  Ops}    — apply a list of operations atomically
%%     status           — print current count
%%     stop             — exit and show final count
counter(Count) ->
    receive
        {increment, N} when is_integer(N) ->
            counter(Count + N);
        {decrement, N} when is_integer(N) ->
            counter(Count - N);
        {multiply, N} when is_integer(N) ->
            counter(Count * N);
        {divide, N} when N =:= 0 ->
            io:format("[counter] division by zero — crashing!~n"),
            exit(division_by_zero);
        {divide, N} when is_integer(N) ->
            counter(Count div N);
        {get, From} ->
            From ! {count, Count},
            counter(Count);
        {set, N} when is_integer(N) ->
            counter(N);
        {batch, Ops} ->
            NewCount = apply_batch(Count, Ops),
            counter(NewCount);
        status ->
            io:format("[counter] current count = ~p~n", [Count]),
            counter(Count);
        stop ->
            io:format("[counter] final count = ~p~n", [Count]);
        Other ->
            io:format("[counter] unknown message: ~p~n", [Other]),
            counter(Count)
    end.

apply_batch(Count, []) -> Count;
apply_batch(Count, [{increment, N} | Rest]) -> apply_batch(Count + N, Rest);
apply_batch(Count, [{decrement, N} | Rest]) -> apply_batch(Count - N, Rest);
apply_batch(Count, [{multiply,  N} | Rest]) -> apply_batch(Count * N, Rest);
apply_batch(Count, [{divide,    N} | Rest]) -> apply_batch(Count div N, Rest);
apply_batch(Count, [_ | Rest]) -> apply_batch(Count, Rest).

%% ===================================================================
%% Section 3 — Supervisor & Let-It-Crash Demo
%% ===================================================================

%% supervised_counter(Initial, MaxRestarts)
%%   Like counter/1 but intentionally crashes on division-by-zero.
%%   The supervisor monitors via link and restarts.
supervised_counter(Count, _MaxRestarts) ->
    receive
        {increment, N} -> supervised_counter(Count + N,  _MaxRestarts);
        {decrement, N} -> supervised_counter(Count - N,  _MaxRestarts);
        {get, From}    -> From ! {count, Count}, supervised_counter(Count, _MaxRestarts);
        {divide, 0}    -> exit(division_by_zero);
        {divide, N}    -> supervised_counter(Count div N, _MaxRestarts);
        {crash, Reason}-> exit(Reason);
        stop           -> io:format("[supervised] final count = ~p~n", [Count]);
        status         -> io:format("[supervised] count = ~p~n", [Count]),
                          supervised_counter(Count, _MaxRestarts)
    end.

%% supervisor(ChildInit)
%%   Monitors a child. On EXIT, logs and restarts.
%%   A real OTP supervisor would use restart intensity + backoff.
supervisor(ChildInit) ->
    process_flag(trap_exit, true),
    Child = spawn_link(ChildInit),
    io:format("[supervisor] started child ~p~n", [Child]),
    supervisor_loop(Child, ChildInit).

supervisor_loop(Child, ChildInit) ->
    receive
        {'EXIT', Child, Reason} ->
            io:format("[supervisor] child ~p crashed: ~p~n", [Child, Reason]),
            io:format("[supervisor] restarting...~n"),
            NewChild = spawn_link(ChildInit),
            io:format("[supervisor] restarted as ~p~n", [NewChild]),
            supervisor_loop(NewChild, ChildInit);
        stop ->
            io:format("[supervisor] stopping child ~p~n", [Child]),
            exit(Child, shutdown);
        Msg ->
            io:format("[supervisor] forwarding: ~p~n", [Msg]),
            Child ! Msg,
            supervisor_loop(Child, ChildInit)
    end.

%% ===================================================================
%% Section 4 — Registry Actor (name → pid lookup)
%% ===================================================================

%% registry()
%%   A simple name registry. Supports register, whereis, unregister.
registry(Map) ->
    receive
        {register, Name, Pid, From} ->
            case maps:find(Name, Map) of
                {ok, _} ->
                    From ! {error, already_registered},
                    registry(Map);
                error ->
                    NewMap = maps:put(Name, Pid, Map),
                    From ! ok,
                    registry(NewMap)
            end;
        {whereis, Name, From} ->
            case maps:find(Name, Map) of
                {ok, Pid} -> From ! {registered, Pid};
                error     -> From ! not_found
            end,
            registry(Map);
        {unregister, Name} ->
            registry(maps:remove(Name, Map));
        list ->
            io:format("[registry] entries:~n"),
            maps:fold(fun(K, V, _) -> io:format("  ~s => ~p~n", [K, V]) end, ok, Map),
            registry(Map);
        stop ->
            io:format("[registry] stopping~n")
    end.

%% ===================================================================
%% Section 5 — Benchmark with Multiple Actors
%% ===================================================================

%% bench_send(N, Pid)
%%   Sends N messages to Pid in rapid succession.
bench_send(0, _Pid) -> done;
bench_send(N, Pid) ->
    Pid ! {increment, 1},
    Pid ! {increment, 1},
    bench_send(N - 1, Pid).

%% bench_receiver(Count)
%%   Counts messages received, prints final tally.
bench_receiver(N) ->
    receive
        {increment, _} -> bench_receiver(N + 1)
    after 100 ->
        io:format("[bench] actor received ~p messages~n", [N])
    end.

%% ===================================================================
%% Main
%% ===================================================================

start() ->
    io:format("~n===== Actor Model Demo (Erlang) =====~n~n"),

    %% --- Step 1: Ping-Pong ---
    io:format("----- 1. Ping-Pong Actors -----~n"),
    PongPid = spawn(fun pong/0),
    spawn(fun() -> ping(4, PongPid) end),
    timer:sleep(200),
    PongPid ! finished,
    timer:sleep(50),
    io:format("~n"),

    %% --- Step 2: Stateful Counter ---
    io:format("----- 2. Stateful Counter Actor -----~n"),
    CounterPid = spawn(fun() -> counter(0) end),
    CounterPid ! {increment, 10},
    CounterPid ! {decrement, 3},
    CounterPid ! {multiply, 2},
    CounterPid ! status,
    CounterPid ! {get, self()},
    receive {count, C1} -> io:format("[main]    counter = ~p~n", [C1]) end,
    %% Batch ops
    CounterPid ! {batch, [{increment, 100}, {divide, 7}, {decrement, 1}]},
    CounterPid ! {get, self()},
    receive {count, C2} -> io:format("[main]    counter after batch = ~p~n", [C2]) end,
    CounterPid ! stop,
    timer:sleep(100),
    io:format("~n"),

    %% --- Step 3: Let It Crash (Supervisor) ---
    io:format("----- 3. Let-It-Crash Supervisor Demo -----~n"),
    SupPid = spawn(fun() ->
        supervisor(fun() -> supervised_counter(0, 3) end)
    end),
    SupPid ! {increment, 42},
    io:format("[main]    sending divide-by-zero to cause crash...~n"),
    SupPid ! {divide, 0},
    timer:sleep(50),
    io:format("[main]    sending increment to restarted actor...~n"),
    SupPid ! {increment, 100},
    SupPid ! status,
    timer:sleep(100),
    SupPid ! stop,
    timer:sleep(50),
    io:format("~n"),

    %% --- Step 4: Registry ---
    io:format("----- 4. Name Registry Actor -----~n"),
    RegPid = spawn(fun() -> registry(maps:new()) end),
    %% Register the counter
    NewCounter = spawn(fun() -> counter(0) end),
    RegPid ! {register, "counter-alpha", NewCounter, self()},
    receive ok -> ok end,
    RegPid ! {register, "counter-beta", NewCounter, self()},
    receive {error, already_registered} ->
        io:format("[main]    duplicate registration rejected~n")
    end,
    RegPid ! list,
    RegPid ! {whereis, "counter-alpha", self()},
    receive
        {registered, P} -> io:format("[main]    found counter-alpha at ~p~n", [P])
    end,
    RegPid ! stop,
    timer:sleep(100),
    io:format("~n"),

    %% --- Step 5: Message Passing Benchmark ---
    io:format("----- 5. Message Passing Benchmark -----~n"),
    BenchPid = spawn(fun() -> bench_receiver(0) end),
    SendStart = erlang:system_time(microsecond),
    bench_send(5000, BenchPid),
    SendEnd = erlang:system_time(microsecond),
    io:format("[main]    sent 10000 messages in ~p us~n", [SendEnd - SendStart]),
    timer:sleep(200),
    io:format("~n"),

    io:format("===== Demo Complete =====~n"),
    halt().
