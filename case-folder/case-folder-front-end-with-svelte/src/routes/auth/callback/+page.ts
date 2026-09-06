// Magic-link callback (`/auth/callback`) load function.
//
// Landing point for the emailed magic link. Reads the one-time `token`
// from the query string and exchanges it for a session (the verify call
// sets the HttpOnly cookie). On success, redirects to the dashboard; on
// any failure, returns an `error` string for the page to display instead
// of throwing, so the user sees a friendly message rather than +error.

import { api, ApiError } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { redirect } from '@sveltejs/kit';

export async function load({ url, fetch }) {
    const token = url.searchParams.get('token');
    if (!token) {
        // Malformed / truncated link — nothing to verify.
        return { error: 'This sign-in link is missing its token.' };
    }
    try {
        const user = await api.auth.verify(token, { fetch });
        cache.setUser(user);
    } catch (e) {
        // Surface a readable reason (expired / already-used token, etc.).
        const message =
            e instanceof ApiError
                ? e.message
                : ((e as Error).message ?? 'Sign-in failed.');
        return { error: message };
    }
    // Success — hand off to the dashboard. The layout's `me()` check will
    // now see the session cookie and let the protected routes through.
    throw redirect(307, '/');
}
