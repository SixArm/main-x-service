// Home/account actions. Sign-out is a server action (BFF): revoke the
// session server-side, then clear the httpOnly cookie and redirect.

import type { Actions, PageServerLoad } from "./$types";
import { redirect } from "@sveltejs/kit";
import { signout } from "$lib/server/auth";
import { CSRF_COOKIE, SESSION_COOKIE } from "$lib/server/session";

// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
export const load: PageServerLoad = () => {
  return { title: "Main X Auth" };
};

export const actions: Actions = {
  signout: async ({ locals, fetch, cookies }) => {
    if (locals.sessionId) {
      await signout(fetch, locals.sessionId, locals.csrfToken);
    }
    cookies.delete(SESSION_COOKIE, { path: "/" });
    cookies.delete(CSRF_COOKIE, { path: "/" });
    redirect(303, "/signin");
  },
};
