// Minimal HTTP stub standing in for the Authentication Service, used only
// by the Playwright e2e suite.
//
// Every auth-service call in this app happens server-side, from the
// SvelteKit BFF's own `fetch` (src/lib/server/auth.ts, admin.ts) — never
// from the browser. `page.route()` only intercepts requests the *browser*
// issues, so it cannot see these calls (spec §11/§13). Rather than stub in
// the browser, the e2e suite points `AUTH_API_URL` at this real (if tiny)
// HTTP server instead, so the BFF's outbound fetch calls actually reach
// something and behave the way the live auth service would for the
// handful of endpoints this app calls.
//
// No dependency beyond Node's built-in `http` — this only needs to run
// under plain `node`, as a second Playwright `webServer` entry.
import { createServer } from "node:http";
import { randomUUID } from "node:crypto";

const PORT = Number(process.env.MOCK_AUTH_PORT ?? 5199);

// Fixture data. Must match the constants tests/e2e/smoke.spec.ts asserts
// against — kept in sync by hand since this file runs as plain Node and
// the spec runs under Playwright's TypeScript loader, so a shared module
// isn't worth the cross-loader friction for four constants.
const PID = "11111111-1111-4111-8111-111111111111";
const EMAIL = "alice@example.com";
const NAME = "Alice";
const VALID_MAGIC_TOKEN = "magic-123";

const SESSION_COOKIE = "__Host-mxi_session";
const CSRF_COOKIE = "__Host-mxi_csrf";

/** sid -> { csrf } for verified magic-link sessions. */
const sessions = new Map();
/** short-lived bearer -> sid, minted by POST /api/auth/token. */
const bearers = new Map();

function sendJson(res, status, body) {
  const text = body === undefined ? "" : JSON.stringify(body);
  res.writeHead(status, {
    "content-type": "application/json",
    "content-length": Buffer.byteLength(text),
  });
  res.end(text);
}

function readBody(req) {
  return new Promise((resolve) => {
    const chunks = [];
    req.on("data", (chunk) => chunks.push(chunk));
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
  });
}

/** Pull one named cookie's value out of a raw `Cookie` request header. */
function cookieValue(cookieHeader, name) {
  if (!cookieHeader) return null;
  const prefix = `${name}=`;
  const segment = cookieHeader
    .split(";")
    .map((s) => s.trim())
    .find((s) => s.startsWith(prefix));
  return segment ? segment.slice(prefix.length) : null;
}

function bearerFromAuthHeader(authHeader) {
  if (!authHeader) return null;
  const match = /^Bearer\s+(.+)$/i.exec(authHeader);
  return match ? match[1] : null;
}

const server = createServer(async (req, res) => {
  const url = new URL(req.url ?? "/", `http://localhost:${PORT}`);
  const { pathname } = url;
  const method = req.method ?? "GET";

  // Playwright's webServer readiness probe.
  if (pathname === "/__mock/health") {
    return sendJson(res, 200, { ok: true });
  }

  if (pathname === "/api/auth/signup" && method === "POST") {
    await readBody(req);
    return sendJson(res, 200, undefined);
  }

  if (pathname === "/api/auth/magic-link" && method === "POST") {
    await readBody(req);
    return sendJson(res, 200, undefined);
  }

  if (pathname.startsWith("/api/auth/magic-link/") && method === "GET") {
    const token = decodeURIComponent(
      pathname.slice("/api/auth/magic-link/".length),
    );
    if (token !== VALID_MAGIC_TOKEN) {
      return sendJson(res, 401, { error: "invalid_token" });
    }
    // Consuming the magic link establishes a session, exactly as the real
    // service does: the caller (src/lib/server/session.ts) reads the sid
    // and CSRF token back out of these Set-Cookie lines by string parsing,
    // not through a browser cookie jar (this is a server-to-server call).
    const sid = randomUUID();
    const csrf = randomUUID();
    sessions.set(sid, { csrf });
    res.setHeader("set-cookie", [
      `${SESSION_COOKIE}=${sid}; Path=/; HttpOnly; Secure; SameSite=Lax`,
      `${CSRF_COOKIE}=${csrf}; Path=/; HttpOnly; Secure; SameSite=Lax`,
    ]);
    return sendJson(res, 200, {
      token: "upstream-access-token",
      pid: PID,
      name: NAME,
      email: EMAIL,
      is_verified: true,
    });
  }

  if (pathname === "/api/auth/token" && method === "POST") {
    const sid = cookieValue(req.headers.cookie, SESSION_COOKIE);
    const session = sid ? sessions.get(sid) : undefined;
    const csrf = req.headers["x-csrf-token"];
    if (!session || session.csrf !== csrf) {
      return sendJson(res, 401, { error: "invalid_session" });
    }
    const bearer = randomUUID();
    bearers.set(bearer, sid);
    return sendJson(res, 200, { token: bearer });
  }

  if (pathname === "/api/auth/me" && method === "GET") {
    const bearer = bearerFromAuthHeader(req.headers.authorization);
    const sid = bearer ? bearers.get(bearer) : undefined;
    if (!sid || !sessions.has(sid)) {
      return sendJson(res, 401, { error: "invalid_token" });
    }
    return sendJson(res, 200, { pid: PID, name: NAME, email: EMAIL });
  }

  if (pathname === "/api/auth/signout" && method === "POST") {
    const bearer = bearerFromAuthHeader(req.headers.authorization);
    const sid = bearer ? bearers.get(bearer) : undefined;
    if (sid) {
      sessions.delete(sid);
      bearers.delete(bearer);
    }
    return sendJson(res, 200, undefined);
  }

  return sendJson(res, 404, { error: "unhandled in mock-auth-server" });
});

server.listen(PORT, () => {
  // eslint-disable-next-line no-console
  console.log(`mock-auth-server listening on http://localhost:${PORT}`);
});
