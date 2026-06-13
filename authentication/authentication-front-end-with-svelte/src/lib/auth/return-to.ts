// Cross-origin SSO token-handoff helpers (issuer side).
//
// After a successful magic-link verification the authentication
// front-end may hand the issued access token to an operator SPA on a
// different origin. The bearer credential must never be redirected to an
// untrusted site, so an `return_to` URL is accepted only when its origin
// is on an explicit allowlist (or is our own origin). These helpers are
// PURE and fully unit-tested; the security control lives here.
//
// Protocol: AGENTS/share/jwt-enforcement.md — "Token acquisition handoff
// (cross-origin SSO)".

/// Parse the comma-separated `VITE_RETURN_TO_ALLOWLIST` env value into a
/// list of origins: split on commas, trim each entry, drop blanks. A
/// missing/empty value yields an empty list (same-origin only).
export function parseAllowlist(env: string | undefined): string[] {
    if (!env) return [];
    return env
        .split(",")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
}

/// True iff `returnTo` is a safe redirect target for the token handoff:
/// it must parse as an absolute http(s) URL AND its origin must be
/// exactly `selfOrigin` or exactly one of `allowlist`. Everything else —
/// unparseable input, non-http(s) schemes (`javascript:`, `data:`, …),
/// relative URLs, and origins not listed — is rejected. Pure.
export function isAllowedReturnTo(
    returnTo: string,
    allowlist: string[],
    selfOrigin: string,
): boolean {
    let url: URL;
    try {
        url = new URL(returnTo);
    } catch {
        return false;
    }
    if (url.protocol !== "http:" && url.protocol !== "https:") return false;
    if (url.origin === selfOrigin) return true;
    return allowlist.includes(url.origin);
}

/// Where `/verify` should send the browser after a successful sign-in.
///
/// - `{ kind: "external", url }` — `returnTo` is allowed: redirect to it
///   with the token in the URL fragment (`#access_token=…`), which
///   browsers never send to servers. The caller does the navigation.
/// - `{ kind: "home" }` — no `returnTo`, or it is not allowlisted: stay
///   on this app (go to `/`). The token is NEVER appended in this case.
///
/// Pure, so the redirect *decision* is unit-testable without navigating.
export type NextDestination =
    | { kind: "external"; url: string }
    | { kind: "home" };

export function nextDestination(
    returnTo: string | null | undefined,
    token: string,
    allowlist: string[],
    selfOrigin: string,
): NextDestination {
    if (returnTo && isAllowedReturnTo(returnTo, allowlist, selfOrigin)) {
        return {
            kind: "external",
            url: `${returnTo}#access_token=${encodeURIComponent(token)}`,
        };
    }
    return { kind: "home" };
}

/// sessionStorage key under which an allowlisted `return_to` is parked
/// across the magic-link email round-trip (same browser, different tab).
export const RETURN_TO_STORAGE_KEY = "mxi_return_to";

/// Persist an allowlisted `return_to` for the round-trip. Reads `?return_to=`
/// from `url`; if present AND allowlisted, stores it; if present but not
/// allowlisted, it is ignored (never stored). Guards `sessionStorage` for
/// SSR / `vite preview`. No-op when absent.
export function persistReturnTo(
    url: URL,
    allowlist: string[],
    selfOrigin: string,
): void {
    if (typeof sessionStorage === "undefined") return;
    const returnTo = url.searchParams.get("return_to");
    if (!returnTo) return;
    if (isAllowedReturnTo(returnTo, allowlist, selfOrigin)) {
        sessionStorage.setItem(RETURN_TO_STORAGE_KEY, returnTo);
    }
}

/// Read the parked `return_to` (or `null`). Guards `sessionStorage`.
export function readReturnTo(): string | null {
    if (typeof sessionStorage === "undefined") return null;
    return sessionStorage.getItem(RETURN_TO_STORAGE_KEY);
}

/// Drop the parked `return_to`. Guards `sessionStorage`.
export function clearReturnTo(): void {
    if (typeof sessionStorage === "undefined") return;
    sessionStorage.removeItem(RETURN_TO_STORAGE_KEY);
}
