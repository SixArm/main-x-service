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
  CSRF_COOKIE,
  CSRF_COOKIE_OPTIONS,
  SESSION_COOKIE,
  SESSION_COOKIE_OPTIONS,
  generateCsrfToken,
  sessionIdFromResponse,
} from "$lib/server/session";
import { verifyMagicLink } from "$lib/server/auth";

// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
const TITLE = "Sign-in link";

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
  cookies.set(CSRF_COOKIE, generateCsrfToken(), CSRF_COOKIE_OPTIONS);
  redirect(303, "/");
};
