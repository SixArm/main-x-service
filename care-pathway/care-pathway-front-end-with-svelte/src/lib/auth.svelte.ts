// Reactive access-token store for the care-pathway operator SPA.
//
// The token is obtained out-of-band from the central
// authentication-service (passwordless magic-link -> access token) and
// stored under the family-shared localStorage key so any operator SPA on
// the same origin family can read it. The ApiClient reads this store on
// each request and attaches `Authorization: Bearer <token>` when present.
//
// The token arrives via the cross-origin SSO handoff: the operator
// clicks "Sign in", the authentication front-end verifies the
// magic-link, and (when this origin is on its allowlist) redirects back
// to `…#access_token=<jwt>`. `captureFromLocation()` reads that fragment
// on app load, stores the token, and strips the fragment. The layout
// also keeps a manual paste field as a dev convenience. See the family
// contract `agents/share/jwt-enforcement.md`.

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

/// Parse an access token out of a URL fragment of the form
/// `#…access_token=<jwt>…` (the cross-origin SSO handoff; see
/// `agents/share/jwt-enforcement.md`). The fragment is treated as
/// `application/x-www-form-urlencoded`, so the token is URL-decoded.
///
/// Pure: pass any hash string (with or without a leading `#`). Returns
/// the token, or `null` when the fragment carries no non-empty
/// `access_token`.
export function captureTokenFromHash(hash: string): string | null {
    if (!hash) {
        return null;
    }
    const raw = hash.startsWith("#") ? hash.slice(1) : hash;
    if (raw.length === 0) {
        return null;
    }
    const value = new URLSearchParams(raw).get("access_token");
    if (!value) {
        return null;
    }
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
}

/// In the browser, read `window.location.hash`; if it carries an access
/// token, store it and strip the fragment from the URL via
/// `history.replaceState` (so the bearer credential is not left in the
/// address bar / history). Returns the captured token, or `null` when
/// there was nothing to capture. No-op (returns `null`) under SSR /
/// `vite preview`, where `window` is absent.
export function captureFromLocation(): string | null {
    if (typeof window === "undefined") {
        return null;
    }
    const captured = captureTokenFromHash(window.location.hash);
    if (!captured) {
        return null;
    }
    setToken(captured);
    const clean = window.location.pathname + window.location.search;
    window.history.replaceState(null, "", clean);
    return captured;
}
