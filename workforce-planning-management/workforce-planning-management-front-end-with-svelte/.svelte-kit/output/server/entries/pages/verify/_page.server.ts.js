import { redirect } from "@sveltejs/kit";
import { s as sessionIdFromResponse, S as SESSION_COOKIE, a as SESSION_COOKIE_OPTIONS } from "../../../chunks/session.js";
import { v as verifyMagicLink } from "../../../chunks/auth.js";
const TITLE = "Sign-in link — Workforce Planning Management";
const load = async ({ url, fetch, cookies }) => {
  const token = url.searchParams.get("token");
  if (!token) {
    return { error: "missingToken", title: TITLE };
  }
  let upstream;
  try {
    upstream = await verifyMagicLink(fetch, token);
  } catch {
    return { error: "serviceUnavailable" };
  }
  if (!upstream.ok) {
    return { error: "invalidToken", title: TITLE };
  }
  const sid = sessionIdFromResponse(upstream);
  if (!sid) {
    return { error: "noSession", title: TITLE };
  }
  cookies.set(SESSION_COOKIE, sid, SESSION_COOKIE_OPTIONS);
  redirect(303, "/");
};
export {
  load
};
