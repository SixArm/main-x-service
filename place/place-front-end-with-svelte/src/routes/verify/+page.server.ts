// BFF verify flow (server-side; the credential never reaches the browser).
//
// The emailed magic link lands at `/verify?token=…`. The SvelteKit server
// exchanges the single-use token at the authentication service and
// re-hosts the resulting opaque session id as an httpOnly cookie on THIS
// origin, then redirects home. No access token is ever exposed to client
// JS, localStorage, or a URL fragment.

import type { PageServerLoad } from "./$types";
import { redirect } from "@sveltejs/kit";
import {
  SESSION_COOKIE,
  SESSION_COOKIE_OPTIONS,
  sessionIdFromResponse,
} from "$lib/server/session";
import { verifyMagicLink } from "$lib/server/auth";

export const load: PageServerLoad = async ({ url, fetch, cookies }) => {
  const token = url.searchParams.get("token");
  if (!token) {
    return { error: "missingToken" as const };
  }
  // A network-level failure (the authentication service unreachable,
  // timed out, DNS failure, connection reset, …) is a different failure
  // mode from a token the service actively rejected: `fetch` throws
  // rather than resolving with a non-ok `Response`. Uncaught, that
  // propagates out of `load` and SvelteKit renders its generic 500
  // error page instead of this route's own friendly UI — confirmed by
  // running this exact scenario (T-26). Caught here so the visitor sees
  // an honest "try again" message instead, distinct from "this link is
  // invalid or has expired" (which would misattribute the problem to
  // the token when the real cause is the service being unreachable).
  let upstream: Response;
  try {
    upstream = await verifyMagicLink(fetch, token);
  } catch {
    return { error: "serviceUnavailable" as const };
  }
  if (!upstream.ok) {
    return { error: "invalidToken" as const };
  }
  const sid = sessionIdFromResponse(upstream);
  if (!sid) {
    return { error: "noSession" as const };
  }
  cookies.set(SESSION_COOKIE, sid, SESSION_COOKIE_OPTIONS);
  redirect(303, "/");
};
