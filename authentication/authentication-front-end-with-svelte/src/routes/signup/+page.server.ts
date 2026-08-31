// Sign-up action (BFF): the server posts the account creation + magic-link
// request to the authentication service. The form carries an email, an
// optional name, and the UI locale; no credential is held client-side.

import type { Actions, PageServerLoad } from "./$types";
import { signup } from "$lib/server/auth";

// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
export const load: PageServerLoad = () => {
  return { title: "Create account — Main X Auth" };
};

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const form = await request.formData();
    const email = String(form.get("email") ?? "").trim();
    const nameRaw = String(form.get("name") ?? "").trim();
    const name = nameRaw ? nameRaw : undefined;
    const localeRaw = form.get("locale");
    const locale = localeRaw ? String(localeRaw) : undefined;
    if (!email) {
      return { sent: false, error: "email-required" };
    }
    const ok = await signup(fetch, email, name, locale);
    return ok ? { sent: true, error: null } : { sent: false, error: "failed" };
  },
};
