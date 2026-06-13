// Reactive bearer-token store for the operator SPA.
//
// The token is obtained out-of-band from the central
// authentication-service (passwordless magic-link -> access token) and
// kept under a single shared localStorage key so any Main X operator SPA
// on the same origin family reads the same session:
//
//     localStorage["mxi_access_token"]
//
// The ApiClient pulls the current token from here on every request and
// attaches `Authorization: Bearer <token>` when present (see client.ts).
// Enforcement is off by default service-side, so an absent token simply
// means no header is sent. Guards `localStorage` so this is safe under
// SSR / `vite preview`. See `agents/share/jwt-enforcement.md`.

/** The shared localStorage key, agreed family-wide. */
export const TOKEN_KEY = "mxi_access_token";

function readStored(): string | null {
    if (typeof localStorage === "undefined") return null;
    const value = localStorage.getItem(TOKEN_KEY);
    return value && value.length > 0 ? value : null;
}

class AuthStore {
    #token = $state<string | null>(readStored());

    /** The current access token, or `null` when the SPA is signed out. */
    get token(): string | null {
        return this.#token;
    }

    /** Store a token (reactive + persisted to localStorage). */
    setToken(token: string): void {
        const trimmed = token.trim();
        this.#token = trimmed.length > 0 ? trimmed : null;
        if (typeof localStorage !== "undefined") {
            if (this.#token) localStorage.setItem(TOKEN_KEY, this.#token);
            else localStorage.removeItem(TOKEN_KEY);
        }
    }

    /** Clear the token (reactive + removed from localStorage). */
    clearToken(): void {
        this.#token = null;
        if (typeof localStorage !== "undefined") {
            localStorage.removeItem(TOKEN_KEY);
        }
    }
}

/** The process-wide token store. */
export const auth = new AuthStore();
