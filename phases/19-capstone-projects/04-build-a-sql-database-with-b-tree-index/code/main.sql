CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE INDEX idx_users_name ON users(name);

INSERT INTO users(id, name) VALUES (10, 'alice'), (20, 'bob'), (15, 'carol');

SELECT id, name FROM users WHERE id = 15;
SELECT id, name FROM users WHERE id BETWEEN 10 AND 20 ORDER BY id;
