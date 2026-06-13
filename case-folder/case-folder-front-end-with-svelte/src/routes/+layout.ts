// Client-side only. The Loco JSON API lives behind the dev-server proxy
// (`/api` → :5150) so the browser sees one origin and the HttpOnly
// session cookie is first-party. A production deployment behind a single
// reverse proxy could re-enable SSR.

import { api, ApiError } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { redirect } from '@sveltejs/kit';

export const ssr = false;

// Routes reachable while signed out.
const PUBLIC_PREFIXES = ['/login', '/auth/callback'];

function isPublic(pathname: string): boolean {
    return PUBLIC_PREFIXES.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

export async function load({ url, fetch }) {
    const publicRoute = isPublic(url.pathname);

    let user = null;
    let signedOut = false;
    try {
        user = await api.auth.me({ fetch });
    } catch (e) {
        if (e instanceof ApiError && e.status === 401) {
            // Definitively signed out.
            signedOut = true;
        }
        // Any other failure (e.g. the API is unreachable) is NOT an auth
        // decision. We must not redirect or throw here: this is the root
        // layout, so a throw would bypass +error.svelte. Fall through and
        // let the page loader surface the error via error(503).
    }

    if (user) {
        cache.setUser(user);
        // Already signed in — keep the login page out of reach.
        if (publicRoute) throw redirect(307, '/');
        return {};
    }

    cache.clearUser();
    if (signedOut && !publicRoute) throw redirect(307, '/login');
    return {};
}
