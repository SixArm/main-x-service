// Server-side auth helpers. This app is otherwise a client-only SPA; the
// one thing that needs a server hop is the opt-in "Sign in with SSO"
// entry point (EV-2, agents/share/authentication-sessions.md §7a), which
// must be a BROWSER navigation to the auth service so the browser itself
// can visit the identity provider and come back.

/** Base URL of the auth service (the Loco app that hosts /api/auth/*). */
const AUTH_API_URL = (
    process.env.AUTH_API_URL ||
    process.env.LOCO_API_PROXY ||
    'http://localhost:5150'
).replace(/\/$/, '');

/**
 * Build the auth service's OIDC login URL, carrying this app's origin as
 * `return_url` so the post-federation bridge lands back here.
 * @param originForReturn - This app's origin, e.g. `https://app.example`.
 * @returns The absolute URL to redirect the browser to.
 */
export function oidcLoginUrl(originForReturn: string): string {
    const params = new URLSearchParams({ return_url: originForReturn });
    return `${AUTH_API_URL}/api/auth/oidc/login?${params.toString()}`;
}
