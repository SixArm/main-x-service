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

// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
export const load: PageServerLoad = async ({ url, fetch, cookies }) => {
  const title = "Sign-in link";
  const token = url.searchParams.get("token");
  if (!token) {
    return { error: "missingToken" as const, title };
  }
  const upstream = await verifyMagicLink(fetch, token);
  if (!upstream.ok) {
    return { error: "invalidToken" as const, title };
  }
  const sid = sessionIdFromResponse(upstream);
  if (!sid) {
    return { error: "noSession" as const, title };
  }
  cookies.set(SESSION_COOKIE, sid, SESSION_COOKIE_OPTIONS);
  redirect(303, "/");
};
