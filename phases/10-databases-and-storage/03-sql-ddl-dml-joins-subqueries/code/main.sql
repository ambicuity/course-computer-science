CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL
);

CREATE TABLE orders (
  id INTEGER PRIMARY KEY,
  user_id INTEGER NOT NULL,
  amount_cents INTEGER NOT NULL,
  FOREIGN KEY(user_id) REFERENCES users(id)
);

INSERT INTO users(id, name) VALUES (1, 'Ada'), (2, 'Linus');
INSERT INTO orders(id, user_id, amount_cents) VALUES
  (10, 1, 2500),
  (11, 1, 1800),
  (12, 2, 4200);

SELECT u.name, SUM(o.amount_cents) AS total
FROM users u
JOIN orders o ON o.user_id = u.id
GROUP BY u.name
ORDER BY total DESC;
