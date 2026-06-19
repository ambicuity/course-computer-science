CREATE TABLE events (
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL,
  ts INTEGER NOT NULL,
  kind TEXT NOT NULL
);

CREATE INDEX idx_events_user_ts ON events(user_id, ts);

EXPLAIN QUERY PLAN
SELECT *
FROM events
WHERE user_id = 42 AND ts >= 1700000000
ORDER BY ts
LIMIT 50;
