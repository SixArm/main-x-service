// Sign-in action (BFF, per-app magic-link login, CMS-T25): the server
// asks the authentication service for a magic link that returns to
// THIS app's own `/verify` (via `return_url` = this origin, honoured
// by the auth service's allow-list). No credential is held
// client-side.

import type { Actions } from "./$types";
import { requestMagicLink } from "$lib/server/auth";

export const actions: Actions = {
  default: async ({ request, fetch, url }) => {
    const form = await request.formData();
    const email = String(form.get("email") ?? "").trim();
    if (!email) {
      return { sent: false, error: "email-required" };
    }
    const ok = await requestMagicLink(fetch, email, url.origin);
    return ok ? { sent: true, error: null } : { sent: false, error: "failed" };
  },
};
