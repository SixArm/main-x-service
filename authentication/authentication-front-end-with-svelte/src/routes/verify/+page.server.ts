// BFF verify flow (server-side; the credential never reaches the browser).
//
// The emailed magic link lands at `/verify?token=…`. The SvelteKit SERVER
// (not the browser) exchanges the single-use token at the authentication
// service, re-hosts the resulting opaque session id as an httpOnly cookie
// on THIS origin, and redirects home. No access token is ever exposed to
// client JS, localStorage, or a URL fragment.

import type { PageServerLoad } from "./$types";
import { redirect } from "@sveltejs/kit";
import {
    SESSION_COOKIE,
    SESSION_COOKIE_OPTIONS,
    sessionIdFromResponse,
} from "$lib/server/session";
import { verifyMagicLink } from "$lib/server/auth";

export const load: PageServerLoad = async ({ url, fetch, cookies }) => {
    const token = url.searchParams.get("token");
    if (!token) {
        return { error: "missingToken" as const };
    }

    // Server-to-server: consume the magic-link token. The auth service sets
    // `__Host-mxi_session` (its own origin) and returns the profile/token in
    // the body — we use neither token nor body in the browser.
    const upstream = await verifyMagicLink(fetch, token);
    if (!upstream.ok) {
        return { error: "invalidToken" as const };
    }

    // Re-host the same opaque session id on THIS origin so the browser
    // sends it back to our BFF (httpOnly — JS can never read it).
    const sid = sessionIdFromResponse(upstream);
    if (!sid) {
        return { error: "noSession" as const };
    }
    cookies.set(SESSION_COOKIE, sid, SESSION_COOKIE_OPTIONS);

    // Signed in — redirect home. No token reaches the browser.
    redirect(303, "/");
};
