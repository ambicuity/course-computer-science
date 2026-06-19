import express, { Request, Response } from "express";
import Database from "better-sqlite3";
import crypto from "node:crypto";

const app = express();
const PORT = 4001;

app.use(express.urlencoded({ extended: true }));
app.use(express.json());

const db = new Database(":memory:");
db.exec(`
  CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password TEXT NOT NULL
  );
  INSERT INTO users (username, password) VALUES ('admin', 'supersecret');
  INSERT INTO users (username, password) VALUES ('alice', 'password123');
`);
const getStmt = db.prepare("SELECT * FROM users WHERE username = ? AND password = ?");

const comments: string[] = [
  "First post!",
  "Great lesson on web security.",
];

const sessions = new Map<string, { username: string; csrfToken: string }>();

function randomToken(): string {
  return crypto.randomBytes(32).toString("hex");
}

function getSession(req: Request): { username: string; csrfToken: string } | null {
  const sid = req.headers.cookie?.match(/session=([^;]+)/)?.[1];
  if (!sid) return null;
  return sessions.get(sid) ?? null;
}

function withSession(req: Request, res: Response): { username: string; csrfToken: string } | null {
  const session = getSession(req);
  if (!session) {
    res.status(401).send("Unauthorized");
    return null;
  }
  return session;
}

// ── VULNERABLE ENDPOINTS ──────────────────────────────────────────

app.get("/search-vuln", (req: Request, res: Response) => {
  const q = (req.query.q as string) ?? "";
  res.send(`<!DOCTYPE html><html><body><h1>Results for ${q}</h1></body></html>`);
});

app.post("/login-vuln", (req: Request, res: Response) => {
  const { username, password } = req.body;
  const sql = `SELECT * FROM users WHERE username = '${username}' AND password = '${password}'`;
  const row = db.prepare(sql).get() as Record<string, unknown> | undefined;
  if (row) {
    const sid = randomToken();
    sessions.set(sid, { username: row.username as string, csrfToken: randomToken() });
    res.setHeader("Set-Cookie", `session=${sid}; Path=/; HttpOnly`);
    res.send(`Welcome, ${row.username as string}`);
  } else {
    res.status(401).send("Invalid credentials");
  }
});

app.get("/comments-vuln", (_req: Request, res: Response) => {
  const items = comments.map((c, i) => `<li>${c}</li>`).join("\n");
  res.send(`<!DOCTYPE html><html><body><ul>${items}</ul></body></html>`);
});

app.post("/comment-vuln", (req: Request, res: Response) => {
  const text = req.body.text ?? "";
  comments.push(text);
  res.redirect("/comments-vuln");
});

app.post("/change-password-vuln", (req: Request, res: Response) => {
  const session = withSession(req, res);
  if (!session) return;
  const { newPassword } = req.body;
  db.prepare("UPDATE users SET password = ? WHERE username = ?").run(newPassword, session.username);
  res.send("Password changed");
});

app.post("/fetch-url-vuln", async (req: Request, res: Response) => {
  const url = req.body.url as string;
  try {
    const resp = await fetch(url);
    const text = await resp.text();
    res.send(text);
  } catch {
    res.status(500).send("Fetch failed");
  }
});

app.post("/deserialize-vuln", (req: Request, res: Response) => {
  const data = req.body.data as string;
  try {
    const obj = JSON.parse(data);
    res.json({ deserialized: obj });
  } catch {
    res.status(400).send("Invalid JSON");
  }
});

// ── DEFENDED ENDPOINTS ────────────────────────────────────────────

function escapeHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");
}

app.get("/search", (req: Request, res: Response) => {
  const q = escapeHtml((req.query.q as string) ?? "");
  res.setHeader("Content-Security-Policy", "default-src 'self'; script-src 'self'");
  res.send(`<!DOCTYPE html><html><body><h1>Results for ${q}</h1></body></html>`);
});

app.post("/login", (req: Request, res: Response) => {
  const { username, password } = req.body;
  const row = getStmt.get(username, password) as Record<string, unknown> | undefined;
  if (row) {
    const sid = randomToken();
    sessions.set(sid, { username: row.username as string, csrfToken: randomToken() });
    res.setHeader("Set-Cookie", `session=${sid}; Path=/; HttpOnly; SameSite=Strict`);
    res.send(`Welcome, ${escapeHtml(row.username as string)}`);
  } else {
    res.status(401).send("Invalid credentials");
  }
});

app.get("/comments", (_req: Request, res: Response) => {
  const items = comments.map((c, i) => `<li>${escapeHtml(c)}</li>`).join("\n");
  res.setHeader("Content-Security-Policy", "default-src 'self'; script-src 'self'");
  res.send(`<!DOCTYPE html><html><body><ul>${items}</ul></body></html>`);
});

app.post("/comment", (req: Request, res: Response) => {
  const text = req.body.text ?? "";
  comments.push(text);
  res.redirect("/comments");
});

app.post("/change-password", (req: Request, res: Response) => {
  const session = withSession(req, res);
  if (!session) return;
  const csrfHeader = req.headers["x-csrf-token"] as string;
  if (!csrfHeader || csrfHeader !== session.csrfToken) {
    res.status(403).send("CSRF token mismatch");
    return;
  }
  const { newPassword } = req.body;
  db.prepare("UPDATE users SET password = ? WHERE username = ?").run(newPassword, session.username);
  res.send("Password changed");
});

app.get("/csrf-token", (req: Request, res: Response) => {
  const session = withSession(req, res);
  if (!session) return;
  res.json({ token: session.csrfToken });
});

const ALLOWED_HOSTS = new Set(["api.example.com", "safe.internal"]);
function isAllowedUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    if (parsed.protocol !== "https:" && parsed.protocol !== "http:") return false;
    if (!ALLOWED_HOSTS.has(parsed.hostname)) return false;
    if (parsed.hostname === "169.254.169.254") return false;
    const ipPattern = /^(10\.|172\.(1[6-9]|2\d|3[01])\.|192\.168\.|127\.)/;
    if (ipPattern.test(parsed.hostname)) return false;
    return true;
  } catch {
    return false;
  }
}

app.post("/fetch-url", async (req: Request, res: Response) => {
  const url = req.body.url as string;
  if (!isAllowedUrl(url)) {
    res.status(403).send("URL not allowed");
    return;
  }
  try {
    const resp = await fetch(url);
    const text = await resp.text();
    res.send(text);
  } catch {
    res.status(500).send("Fetch failed");
  }
});

app.post("/deserialize", (req: Request, res: Response) => {
  const data = req.body.data as string;
  const signature = req.headers["x-signature"] as string;
  if (!signature || signature !== "valid-signature-for-trusted-data") {
    res.status(403).send("Untrusted data rejected");
    return;
  }
  try {
    const obj = JSON.parse(data);
    res.json({ deserialized: obj });
  } catch {
    res.status(400).send("Invalid JSON");
  }
});

app.listen(PORT, () => {
  console.log(`Web Security Lab running on http://localhost:${PORT}`);
  console.log(`  XSS vuln:    /search-vuln?q=<script>alert(1)</script>`);
  console.log(`  SQLi vuln:   POST /login-vuln  username=admin' OR '1'='1`);
  console.log(`  CSRF vuln:   POST /change-password-vuln`);
  console.log(`  SSRF vuln:   POST /fetch-url-vuln  url=http://169.254.169.254/`);
  console.log(`  Defended:    /search, /login, /change-password, /fetch-url, /deserialize`);
});
