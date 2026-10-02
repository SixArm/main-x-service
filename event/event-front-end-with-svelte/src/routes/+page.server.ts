// Sign-out action (BFF): revoke the session server-side, then clear the
// httpOnly cookie and redirect home.

import type { Actions, PageServerLoad } from "./$types";
import { redirect } from "@sveltejs/kit";
import { signout } from "#lib/server/auth.js";
import { CSRF_COOKIE, SESSION_COOKIE } from "#lib/server/session.js";

// `page.data.title` convention (see `+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
// The home route renders the dashboard when signed in and the splash
// otherwise, so the title follows the session state.
export const load: PageServerLoad = ({ locals }) => {
  const signedIn = locals.sessionId !== null;
  return {
    title: signedIn ? "Dashboard · Event Service" : "Event · Main X Index",
  };
};

export const actions: Actions = {
  signout: async ({ locals, fetch, cookies }) => {
    if (locals.sessionId) {
      await signout(fetch, locals.sessionId);
    }
    cookies.delete(SESSION_COOKIE, { path: "/" });
    cookies.delete(CSRF_COOKIE, { path: "/" });
    redirect(303, "/");
  },
};
