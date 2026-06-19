// BFF reverse proxy for entity-API calls.
//
// The browser calls this same-origin proxy (sending the httpOnly session
// cookie automatically, no token in JS). The server exchanges the session
// for a short-lived PASETO and forwards the request to the portfolio service
// with `Authorization: Bearer <paseto>`. The browser thus never holds a
// token, and pages keep using the existing client unchanged (its base URL
// points here).

import type { RequestHandler } from "./$types";
import { PORTFOLIO_API_URL } from "$lib/server/config";
import { exchangeToken } from "$lib/server/auth";

const proxy: RequestHandler = async ({ request, params, url, locals, fetch }) => {
    const target = `${PORTFOLIO_API_URL}/${params.path}${url.search}`;

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
