// BFF reverse proxy for patient-flow API calls.
//
// The browser calls this same-origin proxy; the server forwards to the
// patient-flow service. No token ever lives in browser JS. When the
// family session flow is wired (PF-T18, per
// `agents/share/authentication-sessions.md`), this handler exchanges
// `locals.sessionId` for a short-lived PASETO and injects it as the
// bearer — the seam is marked below. With `PATIENT_FLOW_REQUIRE_AUTH`
// off (the shipped default) the unauthenticated forward is sufficient.

import type { RequestHandler } from "./$types";
import { PATIENT_FLOW_API_URL } from "$lib/server/config";

const proxy: RequestHandler = async ({ request, params, url, fetch }) => {
  const target = `${PATIENT_FLOW_API_URL}/${params.path}${url.search}`;

  // Copy request headers, but drop hop-by-hop / origin-specific ones and
  // never forward the browser's cookie to the entity service.
  const headers = new Headers(request.headers);
  headers.delete("cookie");
  headers.delete("host");
  headers.delete("connection");
  headers.delete("content-length");
  headers.set("accepts-version", "1.0");

  // PF-T18 seam: exchange locals.sessionId → PASETO and set
  // `authorization: Bearer <token>` here once the session flow lands.

  const init: RequestInit = { method: request.method, headers };
  if (request.method !== "GET" && request.method !== "HEAD") {
    init.body = await request.arrayBuffer();
  }

  const upstream = await fetch(target, init);

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
