// Sign-in action (BFF): the server posts the magic-link request to the
// authentication service. No credential is held client-side; the form
// only carries an email + the UI locale (email language).

import type { Actions } from "./$types";
import { requestMagicLink } from "$lib/server/auth";

export const actions: Actions = {
    default: async ({ request, fetch }) => {
        const form = await request.formData();
        const email = String(form.get("email") ?? "").trim();
        const localeRaw = form.get("locale");
        const locale = localeRaw ? String(localeRaw) : undefined;
        if (!email) {
            return { sent: false, error: "email-required" };
        }
        const ok = await requestMagicLink(fetch, email, locale);
        return ok ? { sent: true, error: null } : { sent: false, error: "failed" };
    },
};
