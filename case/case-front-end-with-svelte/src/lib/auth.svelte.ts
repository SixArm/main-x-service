// Reactive access-token store for the Case operator SPA.
//
// The token is obtained out-of-band from the central
// authentication-service (passwordless magic-link -> access token) and
// shared across the Main X operator SPAs under one localStorage key, so
// any SPA on the same origin family reads the same session:
//
//   localStorage["mxi_access_token"]
//
// `ApiClient` attaches it as `Authorization: Bearer <token>` on every
// request when present. The store hydrates from localStorage on load and
// writes through on set/clear; localStorage access is guarded so the
// module also loads under SSR / `vite preview` / vitest, where the global
// may be absent.

/** Shared localStorage key used by every Main X operator SPA on this origin family. */
const STORAGE_KEY = "mxi_access_token";

/**
 * Whether `localStorage` exists in this environment. Guards against SSR /
 * `vite preview` / vitest where the global may be absent.
 * @returns `true` when `localStorage` is usable.
 */
function hasStorage(): boolean {
  return typeof localStorage !== "undefined";
}

/**
 * Read the persisted token, swallowing any access error (e.g. disabled
 * storage / private-mode throw).
 * @returns The stored token, or `null` when absent/unavailable.
 */
function read(): string | null {
  if (!hasStorage()) return null;
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

// Reactive backing state, hydrated once from localStorage. Reads of
// `token()` in components/`$derived` track this `$state` cell.
let current = $state<string | null>(read());

/**
 * The current access token, or `null` when the SPA is unauthenticated.
 * Reactive: call it inside `$derived`/markup to track sign-in/out.
 * @returns The active bearer token, or `null`.
 */
export function token(): string | null {
  return current;
}

/**
 * Store a token and persist it under the shared localStorage key.
 * @param value The access token to make active.
 */
export function setToken(value: string): void {
  current = value;
  if (hasStorage()) {
    try {
      localStorage.setItem(STORAGE_KEY, value);
    } catch {
      // Ignore quota / disabled-storage errors; the in-memory token still
      // drives the current session.
    }
  }
}

/** Clear the token from memory and localStorage (sign the SPA out). */
export function clearToken(): void {
  current = null;
  if (hasStorage()) {
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      // Ignore.
    }
  }
}

/**
 * Parse an access token out of a URL fragment of the form
 * `#…access_token=<jwt>…` (the cross-origin SSO handoff; see
 * `agents/share/jwt-enforcement.md`). The fragment is treated as
 * `application/x-www-form-urlencoded`, so the token is URL-decoded.
 *
 * Pure: pass any hash string (with or without a leading `#`). Returns
 * the token, or `null` when the fragment carries no non-empty
 * `access_token`.
 *
 * @param hash A URL fragment (leading `#` optional).
 * @returns The decoded, trimmed token, or `null` if absent/blank.
 */
export function captureTokenFromHash(hash: string): string | null {
  if (!hash) return null;
  // Drop a leading '#', if any, before treating the rest as a query string.
  const raw = hash.startsWith("#") ? hash.slice(1) : hash;
  if (raw.length === 0) return null;
  // URLSearchParams URL-decodes values for us.
  const params = new URLSearchParams(raw);
  const value = params.get("access_token");
  if (!value) return null;
  const trimmed = value.trim();
  // Reject whitespace-only tokens.
  return trimmed.length > 0 ? trimmed : null;
}

/**
 * In the browser, read `window.location.hash`; if it carries an access
 * token, store it and strip the fragment from the URL via
 * `history.replaceState` (so the bearer credential is not left in the
 * address bar / browser history). Returns the captured token, or `null`
 * when there was nothing to capture. No-op (returns `null`) under SSR /
 * `vite preview`, where `window` is absent.
 *
 * @returns The captured token, or `null` when there was none / no `window`.
 */
export function captureFromLocation(): string | null {
  if (typeof window === "undefined") return null;
  const value = captureTokenFromHash(window.location.hash);
  if (!value) return null;
  setToken(value);
  // Rebuild the URL without the fragment so the bearer credential is not
  // left in the address bar / browser history.
  const clean = window.location.pathname + window.location.search;
  window.history.replaceState(null, "", clean);
  return value;
}
