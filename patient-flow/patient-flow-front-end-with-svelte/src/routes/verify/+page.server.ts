// BFF verify flow (server-side; the credential never reaches the
// browser, PF-T18). The emailed magic link lands at `/verify?token=…`;
// the SvelteKit server exchanges the single-use token at the
// authentication service and re-hosts the resulting opaque session id
// as an httpOnly cookie on THIS origin, then redirects home. No access
// token is ever exposed to client JS, localStorage, or a URL fragment.

import type { PageServerLoad } from "./$types";
import { redirect } from "@sveltejs/kit";
import {
  SESSION_COOKIE,
  SESSION_COOKIE_OPTIONS,
  sessionIdFromResponse,
} from "$lib/server/session";
import { verifyMagicLink } from "$lib/server/auth";

// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
const TITLE = "Sign-in link — Patient Flow";

export const load: PageServerLoad = async ({ url, fetch, cookies }) => {
  const token = url.searchParams.get("token");
  if (!token) {
    return { error: "missingToken" as const, title: TITLE };
  }
  const upstream = await verifyMagicLink(fetch, token);
  if (!upstream.ok) {
    return { error: "invalidToken" as const, title: TITLE };
  }
  const sid = sessionIdFromResponse(upstream);
  if (!sid) {
    return { error: "noSession" as const, title: TITLE };
  }
  cookies.set(SESSION_COOKIE, sid, SESSION_COOKIE_OPTIONS);
  redirect(303, "/");
};
