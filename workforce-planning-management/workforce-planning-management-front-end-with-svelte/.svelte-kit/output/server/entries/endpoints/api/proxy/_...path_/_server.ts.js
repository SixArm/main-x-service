import { W as WPM_API_URL, e as exchangeToken } from "../../../../../chunks/auth.js";
const proxy = async ({
  request,
  params,
  url,
  locals,
  fetch
}) => {
  const target = `${WPM_API_URL}/${params.path}${url.search}`;
  const headers = new Headers(request.headers);
  headers.delete("cookie");
  headers.delete("host");
  headers.delete("connection");
  headers.delete("content-length");
  headers.set("accepts-version", "1.0");
  if (locals.sessionId) {
    const token = await exchangeToken(fetch, locals.sessionId);
    if (token) headers.set("authorization", `Bearer ${token}`);
  }
  const init = { method: request.method, headers };
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
    headers: responseHeaders
  });
};
const GET = proxy;
const POST = proxy;
const PUT = proxy;
const PATCH = proxy;
const DELETE = proxy;
export {
  DELETE,
  GET,
  PATCH,
  POST,
  PUT
};
