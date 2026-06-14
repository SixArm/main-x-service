// Client-side session: the access token + cached profile, persisted to
// localStorage and exposed as Svelte 5 runes so components react to
// sign-in / sign-out. The token is the federation's bearer credential;
// other Main X services accept it directly (RS256 + JWKS).

import { browser } from "$app/environment";
import type { CurrentUser, LoginResponse } from "$lib/api/types";

// localStorage keys for this app's own bookkeeping copy of the session.
const TOKEN_KEY = "mxi.auth.token";
const USER_KEY = "mxi.auth.user";

/**
 * Shared federation localStorage key for the access token.
 *
 * The issued access token is mirrored here (in addition to the
 * `mxi.auth.*` bookkeeping above) so a SAME-ORIGIN sibling operator SPA
 * can read the bearer credential directly, with no cross-origin handoff.
 * This is the key documented in the family JWT-enforcement contract; do
 * not rename without updating every consumer.
 */
/// Shared federation key. The issued access token is mirrored here (in
/// addition to the `mxi.auth.*` bookkeeping above) so a same-origin
/// sibling operator SPA can read the bearer credential with no handoff.
/// This is the key documented in the family JWT-enforcement contract.
export const FEDERATION_TOKEN_KEY = "mxi_access_token";

// Rehydrate the cached profile from localStorage. Returns null off the
// browser (SSR/tests) or when the stored JSON is missing/corrupt.
function readUser(): CurrentUser | null {
    if (!browser) return null;
    const raw = localStorage.getItem(USER_KEY);
    if (!raw) return null;
    try {
        return JSON.parse(raw) as CurrentUser;
    } catch {
        return null;
    }
}

// Reactive session state seeded from localStorage on first load so a
// page refresh keeps the user signed in. `$state` makes components that
// read `session.*` re-render on sign-in / sign-out.
let token = $state<string | null>(browser ? localStorage.getItem(TOKEN_KEY) : null);
let user = $state<CurrentUser | null>(readUser());

/**
 * Client-side session store: the access token + cached profile, persisted
 * to `localStorage` and exposed as Svelte 5 runes.
 *
 * The token is the federation's bearer credential (RS256 + JWKS), accepted
 * directly by other Main X services. Reading any getter inside a component
 * subscribes that component to session changes.
 */
export const session = {
    /** Current access token, or `null` when signed out. */
    get token(): string | null {
        return token;
    },
    /** Cached current-user profile, or `null` when unknown/signed out. */
    get user(): CurrentUser | null {
        return user;
    },
    /** Whether a token is currently held (signed-in). */
    get isAuthenticated(): boolean {
        return token !== null;
    },

    /**
     * Store the result of a successful magic-link verification.
     *
     * Persists the token under both the app's own key and the shared
     * {@link FEDERATION_TOKEN_KEY} so same-origin siblings can read it.
     *
     * @param login - The verify-step {@link LoginResponse} (token + profile).
     */
    /// Store the result of a successful magic-link verification.
    start(login: LoginResponse): void {
        token = login.token;
        user = { pid: login.pid, name: login.name, email: login.email };
        if (browser) {
            // Mirror to the federation key as well as our own bookkeeping.
            localStorage.setItem(TOKEN_KEY, token);
            localStorage.setItem(USER_KEY, JSON.stringify(user));
            localStorage.setItem(FEDERATION_TOKEN_KEY, token);
        }
    },

    /**
     * Refresh the cached profile without changing the token (e.g. after
     * `GET /me`).
     *
     * @param next - The freshly fetched {@link CurrentUser}.
     */
    /// Refresh the cached profile (e.g. after GET /me).
    setUser(next: CurrentUser): void {
        user = next;
        if (browser) localStorage.setItem(USER_KEY, JSON.stringify(next));
    },

    /**
     * Drop all client-side session state (sign out).
     *
     * Clears in-memory state and removes every persisted key, including
     * the shared {@link FEDERATION_TOKEN_KEY}, so siblings see the sign-out.
     */
    /// Drop all client-side session state.
    clear(): void {
        token = null;
        user = null;
        if (browser) {
            localStorage.removeItem(TOKEN_KEY);
            localStorage.removeItem(USER_KEY);
            localStorage.removeItem(FEDERATION_TOKEN_KEY);
        }
    },
};
