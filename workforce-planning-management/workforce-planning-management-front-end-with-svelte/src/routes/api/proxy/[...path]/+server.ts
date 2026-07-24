// BFF reverse proxy for human-capital-management API calls.
//
// The browser calls this same-origin proxy; the server forwards to the
// human-capital-management service. No token ever lives in browser JS: when a
// session cookie is present (PF-T18 magic-link flow), the server
// exchanges it for a short-lived PASETO and injects the bearer here.
// With `HCM_REQUIRE_AUTH` off (the shipped default) the
// unauthenticated forward is sufficient and the exchange is skipped.

import type { RequestHandler } from "./$types";
import { HCM_API_URL } from "$lib/server/config";
import { exchangeToken } from "$lib/server/auth";

const proxy: RequestHandler = async ({
  request,
  params,
  url,
  locals,
  fetch,
}) => {
  const target = `${HCM_API_URL}/${params.path}${url.search}`;

  // Copy request headers, but drop hop-by-hop / origin-specific ones and
  // never forward the browser's cookie to the entity service.
  const headers = new Headers(request.headers);
  headers.delete("cookie");
  headers.delete("host");
  headers.delete("connection");
  headers.delete("content-length");
  headers.set("accepts-version", "1.0");

  // Session → short-lived PASETO, server-side only (PF-T18).
  if (locals.sessionId) {
    const token = await exchangeToken(fetch, locals.sessionId);
    if (token) headers.set("authorization", `Bearer ${token}`);
  }

  const init: RequestInit = { method: request.method, headers };
  if (request.method !== "GET" && request.method !== "HEAD") {
    init.body = await request.arrayBuffer();
  }

  const upstream = await fetch(target, init);

  const responseHeaders = new Headers();
  for (const name of ["content-type", "etag"]) {
    const value = upstream.headers.get(name);
    if (value) responseHeaders.set(name, value);
  }
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
