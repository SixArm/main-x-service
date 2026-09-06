// Sign-up action (BFF): the server posts the account creation + magic-link
// request to the authentication service. The form carries an email, an
// optional name, and the UI locale; no credential is held client-side.

import type { Actions } from "./$types";
import { signup } from "$lib/server/auth";

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
    const outcome = await signup(fetch, email, name, locale);
    if (outcome === "sent") return { sent: true, error: null };
    return {
      sent: false,
      error: outcome === "rateLimited" ? "rate-limited" : "failed",
    };
  },
};
