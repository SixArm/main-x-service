import { b as private_env } from "./shared-server.js";
import { S as SESSION_COOKIE } from "./session.js";
const WPM_API_URL = private_env.WPM_API_URL ?? "http://localhost:5150";
const AUTH_API_URL = private_env.AUTH_API_URL ?? "http://localhost:5150";
function verifyMagicLink(fetchFn, token) {
  return fetchFn(
    `${AUTH_API_URL}/api/auth/magic-link/${encodeURIComponent(token)}`
  );
}
async function requestMagicLink(fetchFn, email, returnUrl, locale) {
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/magic-link`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ email, locale, return_url: returnUrl })
  });
  return res.ok;
}
async function exchangeToken(fetchFn, sid) {
  const res = await fetchFn(`${AUTH_API_URL}/api/auth/token`, {
    method: "POST",
    headers: { cookie: `${SESSION_COOKIE}=${sid}` }
  });
  if (!res.ok) return null;
  const body = await res.json();
  return body.token ?? null;
}
async function signout(fetchFn, sid) {
  const token = await exchangeToken(fetchFn, sid);
  if (!token) return;
  await fetchFn(`${AUTH_API_URL}/api/auth/signout`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}` }
  });
}
function oidcLoginUrl(originForReturn) {
  const params = new URLSearchParams({ return_url: originForReturn });
  return `${AUTH_API_URL}/api/auth/oidc/login?${params.toString()}`;
}
export {
  WPM_API_URL as W,
  exchangeToken as e,
  oidcLoginUrl as o,
  requestMagicLink as r,
  signout as s,
  verifyMagicLink as v
};
