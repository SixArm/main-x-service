import { api, ApiError } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { redirect } from '@sveltejs/kit';

export async function load({ url, fetch }) {
    const token = url.searchParams.get('token');
    if (!token) {
        return { error: 'This sign-in link is missing its token.' };
    }
    try {
        const user = await api.auth.verify(token, { fetch });
        cache.setUser(user);
    } catch (e) {
        const message =
            e instanceof ApiError ? e.message : (e as Error).message ?? 'Sign-in failed.';
        return { error: message };
    }
    // Success — hand off to the dashboard. The layout's `me()` check will
    // now see the session cookie and let the protected routes through.
    throw redirect(307, '/');
}
