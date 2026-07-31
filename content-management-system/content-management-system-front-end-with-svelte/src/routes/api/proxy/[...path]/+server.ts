// BFF reverse proxy for content-management-system API calls.
//
// The browser calls this same-origin proxy; the server forwards to the
// CMS service. No token ever lives in browser JS: when a session cookie
// is present (CMS-T25 magic-link flow), the server exchanges it for a
// short-lived PASETO and injects the bearer here. With
// `CMS_REQUIRE_AUTH` off (the shipped default) the unauthenticated
// forward is sufficient and the exchange is skipped.
//
// ## Two paths this proxy deliberately will not forward
//
// `POST …/variants/{locale}/preview` returns a **raw preview token**,
// and `/api/preview-tokens/…` manages them. Forwarding either would put
// a credential that renders unpublished content into browser
// JavaScript — the thing the family's session design exists to prevent,
// and what `../spec/auth.md` rules out for this app specifically. The
// authoring UI asks `/preview/{pid}/{locale}` instead, where the server
// mints the token, spends it, and returns only the rendered result.
//
// It is a refusal with a reason rather than a silent drop: a `403`
// saying where to go instead is what stops a future contributor
// debugging the service for a decision that was made here.

import type { RequestHandler } from "./$types";
import { CMS_API_URL } from "$lib/server/config";
import { exchangeToken } from "$lib/server/auth";
// The predicate lives in `$lib` because a SvelteKit endpoint may only
// export HTTP verbs and a fixed set of config names.
import { isPreviewTokenPath } from "$lib/proxy-paths";

const REFUSAL = {
  error: "not_proxied",
  description:
    "preview tokens are held by this app's server, never by the browser: " +
    "request /preview/{entry_pid}/{locale} instead",
};

const proxy: RequestHandler = async ({
  request,
  params,
  url,
  locals,
  fetch,
}) => {
  if (isPreviewTokenPath(params.path)) {
    return new Response(JSON.stringify(REFUSAL), {
      status: 403,
      headers: { "content-type": "application/json" },
    });
  }

  const target = `${CMS_API_URL}/${params.path}${url.search}`;

  // Copy request headers, but drop hop-by-hop / origin-specific ones and
  // never forward the browser's cookie to the entity service.
  const headers = new Headers(request.headers);
  headers.delete("cookie");
  headers.delete("host");
  headers.delete("connection");
  headers.delete("content-length");
  headers.set("accepts-version", "1.0");

  // Session → short-lived PASETO, server-side only (CMS-T25).
  if (locals.sessionId) {
    const token = await exchangeToken(fetch, locals.sessionId);
    if (token) headers.set("authorization", `Bearer ${token}`);
  }

  const init: RequestInit = { method: request.method, headers };
  if (request.method !== "GET" && request.method !== "HEAD") {
    init.body = await request.arrayBuffer();
  }

  const upstream = await fetch(target, init);

  // `etag` carries through because the insights and delivery views are
  // conditional; `cache-control` and `x-robots-tag` because a response
  // the service marked private must not become cacheable merely by
  // passing through here.
  const responseHeaders = new Headers();
  for (const name of [
    "content-type",
    "etag",
    "cache-control",
    "x-robots-tag",
  ]) {
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
