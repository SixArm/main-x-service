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

/**
 * The family-shared `localStorage` key under which the SSO access token is
 * persisted. Kept identical across every operator front-end so a token set
 * in one SPA is visible to the others on the same origin family.
 */
/// The family-shared localStorage key. Keep this identical across every
/// operator front-end so a token set in one SPA is visible to the others.
export const TOKEN_STORAGE_KEY = "mxi_access_token";

/**
 * Whether `localStorage` is available in the current environment.
 *
 * @returns `true` in the browser; `false` under SSR / `vite preview` /
 *   unit tests where the global may be absent.
 */
/// Guard `localStorage` for SSR / `vite preview` / test environments
/// where the global may be absent.
function hasStorage(): boolean {
    return typeof localStorage !== "undefined";
}

/**
 * Read the persisted token from `localStorage`, swallowing any access
 * error (private mode, disabled storage) and treating it as "no token".
 *
 * @returns The stored token, or `null` when absent or unreadable.
 */
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
// Reading `token()` (which returns `current`) inside a rune subscribes to
// this signal, so any UI that shows session state updates on set/clear.
let current = $state<string | null>(readStored());

/**
 * Set (or replace) the access token, updating the reactive store and
 * persisting it to `localStorage` for cross-SPA / reload survival.
 *
 * @param token - The bearer access token to store.
 */
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

/**
 * Sign out: clear the access token from both the reactive store and
 * `localStorage`. Reactive consumers re-run and reflect the signed-out
 * state.
 */
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

/**
 * The current access token.
 *
 * Reactive: reading this inside a rune / `$effect` / `$derived` subscribes
 * to the backing signal, so the caller re-runs when the token changes
 * (sign in / sign out). The `ApiClient` reads it on every request to
 * attach the `Authorization: Bearer` header.
 *
 * @returns The token string, or `null` when the operator is signed out.
 */
/// The current access token, or `null` when the operator is signed out.
/// Reactive: reading this inside a rune/effect re-runs on change.
export function token(): string | null {
    return current;
}

/**
 * Parse an access token out of a URL fragment of the form
 * `#…access_token=<jwt>…` — the payload of the cross-origin SSO handoff
 * (see `agents/share/jwt-enforcement.md`). The fragment is treated as
 * `application/x-www-form-urlencoded`, so the token is URL-decoded.
 *
 * Pure and DOM-free, which makes it directly unit-testable: pass any hash
 * string with or without a leading `#`.
 *
 * @param hash - The URL fragment (e.g. `window.location.hash`), with or
 *   without the leading `#`.
 * @returns The decoded, trimmed token, or `null` when the fragment is
 *   empty or carries no non-blank `access_token`.
 */
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
    // Drop the leading '#', then bail if nothing remains.
    const raw = hash.startsWith("#") ? hash.slice(1) : hash;
    if (raw.length === 0) {
        return null;
    }
    // URLSearchParams treats the fragment as form-encoded and URL-decodes
    // the value for us.
    const value = new URLSearchParams(raw).get("access_token");
    if (!value) {
        return null;
    }
    // A whitespace-only token (e.g. `access_token=%20%20`) counts as none.
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
}

/**
 * Complete the SSO handoff on app load: read `window.location.hash`, and
 * if it carries an access token, persist it via {@link setToken} and strip
 * the fragment from the address bar via `history.replaceState` so the
 * bearer credential is not left in the URL / browser history.
 *
 * Called once from the root layout's `onMount` before any route issues an
 * API request.
 *
 * @returns The captured token, or `null` when there was nothing to capture
 *   (no token in the fragment, or running under SSR / `vite preview` where
 *   `window` is absent).
 */
/// In the browser, read `window.location.hash`; if it carries an access
/// token, store it and strip the fragment from the URL via
/// `history.replaceState` (so the bearer credential is not left in the
/// address bar / history). Returns the captured token, or `null` when
/// there was nothing to capture. No-op (returns `null`) under SSR /
/// `vite preview`, where `window` is absent.
export function captureFromLocation(): string | null {
    // No DOM under SSR / preview: nothing to capture.
    if (typeof window === "undefined") {
        return null;
    }
    const captured = captureTokenFromHash(window.location.hash);
    if (!captured) {
        return null;
    }
    setToken(captured);
    // Rebuild the URL without the fragment and rewrite history in place so
    // the token never lingers in the address bar.
    const clean = window.location.pathname + window.location.search;
    window.history.replaceState(null, "", clean);
    return captured;
}
