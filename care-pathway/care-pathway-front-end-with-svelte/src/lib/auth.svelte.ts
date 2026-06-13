// Reactive access-token store for the care-pathway operator SPA.
//
// The token is obtained out-of-band from the central
// authentication-service (passwordless magic-link -> access token) and
// stored under the family-shared localStorage key so any operator SPA on
// the same origin family can read it. The ApiClient reads this store on
// each request and attaches `Authorization: Bearer <token>` when present.
//
// Full magic-link wiring (redirect to the authentication front-end and
// back) is a follow-up; for now the store reads/writes the key directly
// and a minimal session affordance in the layout lets an operator
// paste/clear the token. See the family contract
// `agents/share/jwt-enforcement.md`.

/// The family-shared localStorage key. Keep this identical across every
/// operator front-end so a token set in one SPA is visible to the others.
export const TOKEN_STORAGE_KEY = "mxi_access_token";

/// Guard `localStorage` for SSR / `vite preview` / test environments
/// where the global may be absent.
function hasStorage(): boolean {
    return typeof localStorage !== "undefined";
}

function readStored(): string | null {
    if (!hasStorage()) {
        return null;
    }
    try {
        return localStorage.getItem(TOKEN_STORAGE_KEY);
    } catch {
        return null;
    }
}

// Reactive backing state, hydrated once from localStorage at module load.
let current = $state<string | null>(readStored());

/// Set (or replace) the access token, persisting it to localStorage.
export function setToken(token: string): void {
    current = token;
    if (hasStorage()) {
        try {
            localStorage.setItem(TOKEN_STORAGE_KEY, token);
        } catch {
            // Ignore storage failures (private mode, quota, …): the
            // in-memory token still authenticates this session.
        }
    }
}

/// Clear the access token from the store and localStorage.
export function clearToken(): void {
    current = null;
    if (hasStorage()) {
        try {
            localStorage.removeItem(TOKEN_STORAGE_KEY);
        } catch {
            // Ignore storage failures.
        }
    }
}

/// The current access token, or `null` when the operator is signed out.
/// Reactive: reading this inside a rune/effect re-runs on change.
export function token(): string | null {
    return current;
}
