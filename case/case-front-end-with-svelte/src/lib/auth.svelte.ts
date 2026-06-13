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

const STORAGE_KEY = "mxi_access_token";

function hasStorage(): boolean {
  return typeof localStorage !== "undefined";
}

function read(): string | null {
  if (!hasStorage()) return null;
  try {
    return localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

// Reactive backing state, hydrated once from localStorage.
let current = $state<string | null>(read());

/** The current access token, or `null` when the SPA is unauthenticated. */
export function token(): string | null {
  return current;
}

/** Store a token and persist it under the shared localStorage key. */
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
