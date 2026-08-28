// BFF reverse proxy for entity-API calls.
//
// The browser calls this same-origin proxy (sending the httpOnly session
// cookie automatically, no token in JS). The server exchanges the session
// for a short-lived PASETO and forwards the request to the person service
// with `Authorization: Bearer <paseto>`. The browser thus never holds a
// token, and pages keep using the existing client unchanged (its base URL
// points here).
//
// Mutating requests additionally require CSRF double-submit proof
// (agents/share/authentication-sessions.md §4): the `X-CSRF-Token` header
// must match the `__Host-mxi_csrf` cookie, plus an Origin/Referer
// backstop. A request that fails either check is rejected before it ever
// reaches the person service.

import type { RequestHandler } from "./$types";
import { PERSON_API_URL } from "$lib/server/config";
import { exchangeToken } from "$lib/server/auth";
import { CSRF_COOKIE, verifyCsrf } from "$lib/server/session";

/** Methods that mutate state and therefore require CSRF proof. */
const SAFE_METHODS = new Set(["GET", "HEAD"]);

function csrfRejection(): Response {
  return new Response(JSON.stringify({ error: "csrf" }), {
    status: 403,
    headers: { "content-type": "application/json" },
  });
}

/** The origin of a `Referer` header value, or `null` if absent/unparseable. */
function refererOrigin(referer: string | null): string | null {
  if (!referer) return null;
  try {
    return new URL(referer).origin;
  } catch {
    return null;
  }
}

const proxy: RequestHandler = async ({
  request,
  params,
  url,
  locals,
  fetch,
  cookies,
}) => {
  if (!SAFE_METHODS.has(request.method)) {
    // 1. Double-submit token check.
    const cookieToken = cookies.get(CSRF_COOKIE);
    const headerToken = request.headers.get("x-csrf-token");
    if (!verifyCsrf(cookieToken, headerToken)) {
      return csrfRejection();
    }
    // 2. Origin/Referer backstop. Only rejects when a value is present
    //    and disagrees — some legitimate same-origin requests omit both
    //    headers, and the token check above is the primary defense.
    const origin = request.headers.get("origin");
    const referer = request.headers.get("referer");
    const sourceOrigin = origin ?? refererOrigin(referer);
    if (sourceOrigin && sourceOrigin !== url.origin) {
      return csrfRejection();
    }
  }

  const target = `${PERSON_API_URL}/${params.path}${url.search}`;

  // Copy request headers, but drop hop-by-hop / origin-specific ones and
  // never forward the browser's cookie to the entity service.
  const headers = new Headers(request.headers);
  headers.delete("cookie");
  headers.delete("host");
  headers.delete("connection");
  headers.delete("content-length");

  // Inject the server-exchanged PASETO when a session is present.
  if (locals.sessionId) {
    const token = await exchangeToken(fetch, locals.sessionId);
    if (token) headers.set("authorization", `Bearer ${token}`);
  }

  const init: RequestInit = { method: request.method, headers };
  if (request.method !== "GET" && request.method !== "HEAD") {
    init.body = await request.arrayBuffer();
  }

  const upstream = await fetch(target, init);

  // Relay status + body; copy a safe subset of response headers.
  const responseHeaders = new Headers();
  const contentType = upstream.headers.get("content-type");
  if (contentType) responseHeaders.set("content-type", contentType);
  return new Response(upstream.body, {
    status: upstream.status,
    headers: responseHeaders,
  });
};

export const GET = proxy;
export const POST = proxy;
export const PUT = proxy;
export const PATCH = proxy;
export const DELETE = proxy;
