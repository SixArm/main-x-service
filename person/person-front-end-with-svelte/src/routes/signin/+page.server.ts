// Sign-in action (BFF, per-app magic-link login): the server asks the
// authentication service for a magic link that returns to THIS app's own
// `/verify` (via `return_url` = this origin, honoured by the auth
// service's allow-list). No credential is held client-side.

import type { Actions, PageServerLoad } from "./$types";
import { requestMagicLink } from "$lib/server/auth";

// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
export const load: PageServerLoad = () => {
  return { title: "Sign in" };
};

export const actions: Actions = {
  default: async ({ request, fetch, url }) => {
    const form = await request.formData();
    const email = String(form.get("email") ?? "").trim();
    const localeRaw = form.get("locale");
    const locale = localeRaw ? String(localeRaw) : undefined;
    if (!email) {
      return { sent: false, error: "email-required" };
    }
    // Return to this app's own origin so the emailed link lands on our
    // /verify (the auth service honours it only if allow-listed).
    const ok = await requestMagicLink(fetch, email, url.origin, locale);
    return ok ? { sent: true, error: null } : { sent: false, error: "failed" };
  },
};
