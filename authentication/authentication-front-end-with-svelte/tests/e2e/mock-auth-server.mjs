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

// Fixture data. Must match the constants tests/e2e/smoke.spec.ts and
// tests/e2e/admin-attributes.spec.ts assert against — kept in sync by
// hand since this file runs as plain Node and the specs run under
// Playwright's TypeScript loader, so a shared module isn't worth the
// cross-loader friction for a handful of constants.
const PID = "11111111-1111-4111-8111-111111111111";
const EMAIL = "alice@example.com";
const NAME = "Alice";
const VALID_MAGIC_TOKEN = "magic-123";
// An email that simulates the auth service's rate limit (5 requests /
// 5 min per email) being exceeded — must match smoke.spec.ts's copy of
// the same constant (AFE-4).
const RATE_LIMITED_EMAIL = "toomany@example.com";
// A magic-link token that simulates the auth service being unreachable
// (see the `req.socket.destroy()` handling below) — must match
// smoke.spec.ts's copy of the same constant.
const NETWORK_ERROR_MAGIC_TOKEN = "magic-network-error";

// A second identity carrying `access=admin` — the operator who manages
// other users' ABAC attributes. Distinct from `PID`/`EMAIL` above (an
// ordinary signed-in user, no admin rights) so the 403 path is real: it
// is this crate's own ABAC engine that would deny a non-admin caller,
// not merely an untested branch.
const ADMIN_PID = "22222222-2222-4222-8222-222222222222";
const ADMIN_EMAIL = "admin@example.com";
const ADMIN_NAME = "Ada";
const ADMIN_MAGIC_TOKEN = "magic-admin-456";

// The user whose attributes the admin views/edits — distinct from both
// login identities above, matching the real UI's "manage someone else's
// attributes" flow (an admin never edits their own attributes here).
const TARGET_PID = "33333333-3333-4333-8333-333333333333";
const TARGET_EMAIL = "target@example.com";

const SESSION_COOKIE = "__Host-mxi_session";
const CSRF_COOKIE = "__Host-mxi_csrf";

/** sid -> { csrf, pid, email, name, isAdmin } for verified magic-link
 *  sessions. */
const sessions = new Map();
/** short-lived bearer -> sid, minted by POST /api/auth/token. */
const bearers = new Map();
/** pid -> ABAC attribute map, seeded with one target user so `GET` has
 *  something real to return; `PUT` replaces it in place. */
const userAttributes = new Map([[TARGET_PID, { access: ["write"] }]]);

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
    const body = await readBody(req);
    const { email } = JSON.parse(body || "{}");
    if (email === RATE_LIMITED_EMAIL) {
      return sendJson(res, 429, { error: "rate_limited" });
    }
    return sendJson(res, 200, undefined);
  }

  if (pathname === "/api/auth/magic-link" && method === "POST") {
    const body = await readBody(req);
    const { email } = JSON.parse(body || "{}");
    if (email === RATE_LIMITED_EMAIL) {
      return sendJson(res, 429, { error: "rate_limited" });
    }
    return sendJson(res, 200, undefined);
  }

  if (pathname.startsWith("/api/auth/magic-link/") && method === "GET") {
    const token = decodeURIComponent(
      pathname.slice("/api/auth/magic-link/".length),
    );
    if (token === NETWORK_ERROR_MAGIC_TOKEN) {
      // Simulate the auth service being unreachable: reset the
      // connection instead of answering, so the BFF's `fetch` rejects
      // rather than resolving with a non-ok `Response` — the scenario
      // `+page.server.ts`'s try/catch around `verifyMagicLink` exists
      // to handle (smoke.spec.ts pins the friendly message this produces).
      req.socket.destroy();
      return;
    }
    const identity =
      token === VALID_MAGIC_TOKEN
        ? { pid: PID, name: NAME, email: EMAIL, isAdmin: false }
        : token === ADMIN_MAGIC_TOKEN
          ? { pid: ADMIN_PID, name: ADMIN_NAME, email: ADMIN_EMAIL, isAdmin: true }
          : null;
    if (!identity) {
      return sendJson(res, 401, { error: "invalid_token" });
    }
    // Consuming the magic link establishes a session, exactly as the real
    // service does: the caller (src/lib/server/session.ts) reads the sid
    // and CSRF token back out of these Set-Cookie lines by string parsing,
    // not through a browser cookie jar (this is a server-to-server call).
    const sid = randomUUID();
    const csrf = randomUUID();
    sessions.set(sid, { csrf, ...identity });
    res.setHeader("set-cookie", [
      `${SESSION_COOKIE}=${sid}; Path=/; HttpOnly; Secure; SameSite=Lax`,
      `${CSRF_COOKIE}=${csrf}; Path=/; HttpOnly; Secure; SameSite=Lax`,
    ]);
    return sendJson(res, 200, {
      token: "upstream-access-token",
      pid: identity.pid,
      name: identity.name,
      email: identity.email,
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
    const session = sid ? sessions.get(sid) : undefined;
    if (!session) {
      return sendJson(res, 401, { error: "invalid_token" });
    }
    return sendJson(res, 200, {
      pid: session.pid,
      name: session.name,
      email: session.email,
    });
  }

  if (
    pathname.startsWith("/api/auth/admin/users/") &&
    pathname.endsWith("/attributes") &&
    (method === "GET" || method === "PUT")
  ) {
    const targetPid = decodeURIComponent(
      pathname.slice(
        "/api/auth/admin/users/".length,
        pathname.length - "/attributes".length,
      ),
    );
    const bearer = bearerFromAuthHeader(req.headers.authorization);
    const sid = bearer ? bearers.get(bearer) : undefined;
    const session = sid ? sessions.get(sid) : undefined;
    if (!session) {
      return sendJson(res, 401, { error: "unauthorized" });
    }
    if (!session.isAdmin) {
      return sendJson(res, 403, {
        error: "forbidden",
        description: "caller does not carry access=admin",
      });
    }
    const targetEmail = targetPid === TARGET_PID ? TARGET_EMAIL : "unknown@example.com";
    if (method === "PUT") {
      const body = await readBody(req);
      let attributes;
      try {
        attributes = JSON.parse(body).attributes;
      } catch {
        return sendJson(res, 400, { error: "invalid_body" });
      }
      userAttributes.set(targetPid, attributes ?? {});
    }
    return sendJson(res, 200, {
      pid: targetPid,
      email: targetEmail,
      attributes: userAttributes.get(targetPid) ?? {},
    });
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
