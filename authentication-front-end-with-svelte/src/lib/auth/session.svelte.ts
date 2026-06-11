// Client-side session: the access token + cached profile, persisted to
// localStorage and exposed as Svelte 5 runes so components react to
// sign-in / sign-out. The token is the federation's bearer credential;
// other Main X services accept it directly (RS256 + JWKS).

import { browser } from "$app/environment";
import type { CurrentUser, LoginResponse } from "$lib/api/types";

const TOKEN_KEY = "mxi.auth.token";
const USER_KEY = "mxi.auth.user";

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

let token = $state<string | null>(browser ? localStorage.getItem(TOKEN_KEY) : null);
let user = $state<CurrentUser | null>(readUser());

export const session = {
    get token(): string | null {
        return token;
    },
    get user(): CurrentUser | null {
        return user;
    },
    get isAuthenticated(): boolean {
        return token !== null;
    },

    /// Store the result of a successful magic-link verification.
    start(login: LoginResponse): void {
        token = login.token;
        user = { pid: login.pid, name: login.name, email: login.email };
        if (browser) {
            localStorage.setItem(TOKEN_KEY, token);
            localStorage.setItem(USER_KEY, JSON.stringify(user));
        }
    },

    /// Refresh the cached profile (e.g. after GET /me).
    setUser(next: CurrentUser): void {
        user = next;
        if (browser) localStorage.setItem(USER_KEY, JSON.stringify(next));
    },

    /// Drop all client-side session state.
    clear(): void {
        token = null;
        user = null;
        if (browser) {
            localStorage.removeItem(TOKEN_KEY);
            localStorage.removeItem(USER_KEY);
        }
    },
};
