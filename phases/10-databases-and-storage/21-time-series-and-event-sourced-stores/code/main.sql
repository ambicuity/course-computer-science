-- SQL for Event Sourcing in PostgreSQL
-- Phase 10 — Databases & Storage Systems
--
-- Patterns demonstrated:
--   1. Append-only event table with optimistic locking
--   2. Event stream reads by aggregate_id
--   3. State rebuild by folding over events (PL/pgSQL)
--   4. Snapshot table for faster rebuilds
--   5. CQRS materialized view


-- ── 1. Event table ────────────────────────────────────────────────────
--
-- Every state change is an immutable, versioned event in this table.
-- The UNIQUE (aggregate_id, version) constraint provides optimistic
-- locking: if two concurrent writers try to insert the same version,
-- one will get a unique violation and must retry.

CREATE TABLE IF NOT EXISTS events (
    id            BIGSERIAL    PRIMARY KEY,
    aggregate_id  TEXT         NOT NULL,
    version       INTEGER      NOT NULL,
    event_type    TEXT         NOT NULL,
    event_data    JSONB        NOT NULL DEFAULT '{}',
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT now(),
    UNIQUE (aggregate_id, version)
);

CREATE INDEX IF NOT EXISTS idx_events_aggregate
    ON events (aggregate_id, version);


-- ── 2. Append event (optimistic locking) ──────────────────────────────
--
-- Application code flow:
--   1. BEGIN;
--   2. SELECT COALESCE(MAX(version), 0) INTO cur_version
--        FROM events WHERE aggregate_id = 'acct:42';
--   3. INSERT INTO events (aggregate_id, version, event_type, event_data)
--        VALUES ('acct:42', cur_version + 1, 'Deposited', '{"amount": 100}');
--   4. COMMIT;
--
-- The UNIQUE constraint on (aggregate_id, version) ensures no two
-- transactions append the same version. If two concurrent transactions
-- both read cur_version=3 and try to insert version=4, exactly one
-- commits; the other gets a unique-violation error and must retry.
--
-- Example insert (run manually):
-- INSERT INTO events (aggregate_id, version, event_type, event_data)
-- VALUES ('acct:42', 1, 'AccountOpened', '{"owner": "Alice"}');
-- INSERT INTO events (aggregate_id, version, event_type, event_data)
-- VALUES ('acct:42', 2, 'Deposited', '{"amount": 100}');
-- INSERT INTO events (aggregate_id, version, event_type, event_data)
-- VALUES ('acct:42', 3, 'Withdrew', '{"amount": 50}');
-- INSERT INTO events (aggregate_id, version, event_type, event_data)
-- VALUES ('acct:42', 4, 'Deposited', '{"amount": 100}');


-- ── 3. Read events by aggregate_id (ordered) ──────────────────────────

SELECT aggregate_id, version, event_type, event_data, created_at
FROM events
WHERE aggregate_id = 'acct:42'
ORDER BY version;


-- ── 4. Rebuild state ──────────────────────────────────────────────────
--
-- Current state = fold(events, initial_state, apply_fn).
-- The PL/pgSQL function below replays all events for an aggregate_id
-- through a hardcoded state machine and returns the final JSONB state.

CREATE OR REPLACE FUNCTION rebuild_state(p_aggregate_id TEXT)
RETURNS JSONB
LANGUAGE plpgsql AS $$
DECLARE
    state JSONB := '{}';
    rec RECORD;
BEGIN
    FOR rec IN
        SELECT event_type, event_data
        FROM events
        WHERE aggregate_id = p_aggregate_id
        ORDER BY version
    LOOP
        IF rec.event_type = 'AccountOpened' THEN
            state := jsonb_set(state, '{owner}', rec.event_data->'owner');
            state := jsonb_set(state, '{balance}', '0');
        ELSIF rec.event_type = 'Deposited' THEN
            state := jsonb_set(
                state,
                '{balance}',
                to_jsonb(
                    COALESCE((state->>'balance')::numeric, 0)
                    + (rec.event_data->>'amount')::numeric
                )
            );
        ELSIF rec.event_type = 'Withdrew' THEN
            state := jsonb_set(
                state,
                '{balance}',
                to_jsonb(
                    COALESCE((state->>'balance')::numeric, 0)
                    - (rec.event_data->>'amount')::numeric
                )
            );
        END IF;
    END LOOP;
    RETURN state;
END;
$$;

-- SELECT rebuild_state('acct:42');
-- Expected: {"owner": "Alice", "balance": 150}


-- ── 5. Snapshot table ─────────────────────────────────────────────────
--
-- For aggregates with many events (millions), replaying from version 1
-- every time is slow. Snapshots store the state at a specific version.
-- On read: load snapshot, then replay events WHERE version > snapshot.version.

CREATE TABLE IF NOT EXISTS snapshots (
    aggregate_id TEXT         PRIMARY KEY,
    state        JSONB        NOT NULL,
    version      INTEGER      NOT NULL,
    updated_at   TIMESTAMPTZ  NOT NULL DEFAULT now()
);

-- After N events (e.g., every 100), the application stores a snapshot:
-- INSERT INTO snapshots (aggregate_id, state, version)
-- VALUES ('acct:42', rebuild_state('acct:42'), 100)
-- ON CONFLICT (aggregate_id) DO UPDATE
--   SET state = EXCLUDED.state,
--       version = EXCLUDED.version,
--       updated_at = now();

-- Optimized read with snapshot:
-- SELECT state FROM snapshots WHERE aggregate_id = 'acct:42';  -- ver=100
-- SELECT event_type, event_data FROM events
-- WHERE aggregate_id = 'acct:42' AND version > 100
-- ORDER BY version;
-- Then fold only the delta events into the snapshot state.


-- ── 6. CQRS materialized view ─────────────────────────────────────────
--
-- The read model is separated from the write model. This materialized
-- view pre-computes account summaries from the event stream. It is
-- refreshed periodically (REFRESH MATERIALIZED VIEW) or incrementally
-- via a trigger.

DROP MATERIALIZED VIEW IF EXISTS account_summary CASCADE;

CREATE MATERIALIZED VIEW account_summary AS
SELECT
    e.aggregate_id,
    COALESCE(
        (SELECT state->>'owner' FROM snapshots s WHERE s.aggregate_id = e.aggregate_id),
        (SELECT event_data->>'owner' FROM events e2
         WHERE e2.aggregate_id = e.aggregate_id
           AND e2.event_type = 'AccountOpened'
         LIMIT 1),
        'unknown'
    ) AS owner,
    COALESCE(
        (SELECT (s.state->>'balance')::numeric FROM snapshots s
         WHERE s.aggregate_id = e.aggregate_id),
        0
    ) AS balance,
    COUNT(*) AS total_events,
    MAX(e.version) AS current_version
FROM events e
GROUP BY e.aggregate_id;

-- REFRESH MATERIALIZED VIEW account_summary;
-- SELECT * FROM account_summary ORDER BY aggregate_id;


-- ── 7. Temporal query: state at any past version ──────────────────────
--
-- What was the balance at version 3 (before the second deposit)?
-- Replay only events WHERE version <= 3.

-- SELECT rebuild_state_upto('acct:42', 3);
-- The same PL/pgSQL function but with an additional LIMIT clause:
-- FOR rec IN ... WHERE aggregate_id = p_aggregate_id AND version <= p_version ...

CREATE OR REPLACE FUNCTION rebuild_state_at(
    p_aggregate_id TEXT,
    p_version      INTEGER
)
RETURNS JSONB
LANGUAGE plpgsql AS $$
DECLARE
    state JSONB := '{}';
    rec RECORD;
BEGIN
    FOR rec IN
        SELECT event_type, event_data
        FROM events
        WHERE aggregate_id = p_aggregate_id
          AND version <= p_version
        ORDER BY version
    LOOP
        IF rec.event_type = 'AccountOpened' THEN
            state := jsonb_set(state, '{owner}', rec.event_data->'owner');
            state := jsonb_set(state, '{balance}', '0');
        ELSIF rec.event_type = 'Deposited' THEN
            state := jsonb_set(
                state,
                '{balance}',
                to_jsonb(
                    COALESCE((state->>'balance')::numeric, 0)
                    + (rec.event_data->>'amount')::numeric
                )
            );
        ELSIF rec.event_type = 'Withdrew' THEN
            state := jsonb_set(
                state,
                '{balance}',
                to_jsonb(
                    COALESCE((state->>'balance')::numeric, 0)
                    - (rec.event_data->>'amount')::numeric
                )
            );
        END IF;
    END LOOP;
    RETURN state;
END;
$$;

-- SELECT rebuild_state_at('acct:42', 3);
-- Expected: {"owner": "Alice", "balance": 50}
