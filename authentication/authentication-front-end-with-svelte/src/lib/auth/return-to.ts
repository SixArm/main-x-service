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

/**
 * Parse the comma-separated `VITE_RETURN_TO_ALLOWLIST` env value into a
 * list of trusted origins.
 *
 * Split on commas, trim each entry, drop blanks. A missing/empty value
 * yields an empty list — meaning same-origin only (no external handoff).
 *
 * @param env - Raw env value (e.g. `"https://a.test, https://b.test"`).
 * @returns The list of trusted origins (possibly empty). Pure.
 */
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

/**
 * Decide whether `returnTo` is a SAFE redirect target for the token handoff.
 *
 * This is the token-exfiltration control. The bearer credential must never
 * be handed to an attacker-controlled site, so the rule is strict and
 * allowlist-based: `returnTo` is allowed only when it parses as an absolute
 * `http(s)` URL AND its ORIGIN is exactly `selfOrigin` or exactly one of
 * `allowlist`. Matching on origin (scheme + host + port) — not a substring
 * or host suffix — blocks look-alike hosts and port/scheme downgrades.
 *
 * Everything else is rejected: unparseable input; non-`http(s)` schemes
 * (`javascript:`, `data:`, …) which could run script or smuggle the token;
 * protocol-relative (`//host`) and other relative URLs; and any origin not
 * explicitly listed. Default-deny.
 *
 * @param returnTo - The candidate redirect target.
 * @param allowlist - Trusted origins from {@link parseAllowlist}.
 * @param selfOrigin - This app's own origin (always allowed).
 * @returns `true` only for an allowed target. Pure.
 */
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
        // Absolute-URL parse: relative/garbage input throws → rejected.
        url = new URL(returnTo);
    } catch {
        return false;
    }
    // Block non-web schemes (javascript:, data:, file:, …) outright.
    if (url.protocol !== "http:" && url.protocol !== "https:") return false;
    // Our own origin is always a valid handoff target.
    if (url.origin === selfOrigin) return true;
    // Otherwise require an exact origin match against the allowlist.
    return allowlist.includes(url.origin);
}

/**
 * The redirect decision made by `/verify` after a successful sign-in.
 *
 * - `{ kind: "external"; url }` — an allowlisted `return_to`: navigate to
 *   it with the token carried in the URL FRAGMENT (`#access_token=…`).
 * - `{ kind: "home" }` — no/disallowed `return_to`: stay on this app (`/`).
 */
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

/**
 * Compute where `/verify` should send the browser after a successful sign-in.
 *
 * The token is placed in the URL FRAGMENT, never the query string: browsers
 * do not transmit the fragment to servers (it stays client-side), so it is
 * kept out of server access logs, the `Referer` header, and proxy logs —
 * unlike a `?access_token=` query param. The destination SPA reads the
 * fragment with JavaScript. When `returnTo` is absent or not allowlisted the
 * token is NEVER appended and the browser stays on this app.
 *
 * @param returnTo - Candidate redirect target (may be null/undefined/empty).
 * @param token - The freshly issued access token to hand off.
 * @param allowlist - Trusted origins from {@link parseAllowlist}.
 * @param selfOrigin - This app's own origin.
 * @returns The {@link NextDestination} decision. Pure — does not navigate.
 */
export function nextDestination(
    returnTo: string | null | undefined,
    token: string,
    allowlist: string[],
    selfOrigin: string,
): NextDestination {
    // Only hand off the token when the target passes the allowlist check.
    if (returnTo && isAllowedReturnTo(returnTo, allowlist, selfOrigin)) {
        return {
            kind: "external",
            // Fragment (#…), not query (?…): never sent to servers/logs.
            url: `${returnTo}#access_token=${encodeURIComponent(token)}`,
        };
    }
    // Default-safe: stay on this app; the token is not exposed.
    return { kind: "home" };
}

/**
 * `sessionStorage` key under which an allowlisted `return_to` is parked
 * across the magic-link email round-trip.
 *
 * The emailed link points only at `/verify?token=…` and does NOT carry the
 * `return_to`, so it must be remembered client-side. `sessionStorage` is
 * per-tab/origin and cleared when the tab closes, which suits a single
 * sign-in flow.
 */
/// sessionStorage key under which an allowlisted `return_to` is parked
/// across the magic-link email round-trip (same browser, different tab).
export const RETURN_TO_STORAGE_KEY = "mxi_return_to";

/**
 * Park an allowlisted `return_to` for the email round-trip.
 *
 * Reads `?return_to=` from `url`. If present AND allowlisted, stores it;
 * if present but NOT allowlisted, it is ignored — never stored, so a
 * malicious origin can never be revived later by {@link readReturnTo}.
 * No-op when the param is absent. Guards `sessionStorage` for SSR /
 * `vite preview`.
 *
 * @param url - The current page URL (signin/signup).
 * @param allowlist - Trusted origins from {@link parseAllowlist}.
 * @param selfOrigin - This app's own origin.
 */
/// Persist an allowlisted `return_to` for the round-trip. Reads `?return_to=`
/// from `url`; if present AND allowlisted, stores it; if present but not
/// allowlisted, it is ignored (never stored). Guards `sessionStorage` for
/// SSR / `vite preview`. No-op when absent.
export function persistReturnTo(
    url: URL,
    allowlist: string[],
    selfOrigin: string,
): void {
    // Guard for SSR / preview where sessionStorage is undefined.
    if (typeof sessionStorage === "undefined") return;
    const returnTo = url.searchParams.get("return_to");
    if (!returnTo) return;
    // Re-validate here so a disallowed value is never persisted at all.
    if (isAllowedReturnTo(returnTo, allowlist, selfOrigin)) {
        sessionStorage.setItem(RETURN_TO_STORAGE_KEY, returnTo);
    }
}

/**
 * Read the parked `return_to`, or `null` when none was stored.
 *
 * @returns The parked target (already validated by {@link persistReturnTo});
 *   callers re-check via {@link nextDestination} before handing off. Guards
 *   `sessionStorage`.
 */
/// Read the parked `return_to` (or `null`). Guards `sessionStorage`.
export function readReturnTo(): string | null {
    if (typeof sessionStorage === "undefined") return null;
    return sessionStorage.getItem(RETURN_TO_STORAGE_KEY);
}

/**
 * Drop the parked `return_to` after the handoff decision is made, so it
 * cannot leak into a later, unrelated sign-in. Guards `sessionStorage`.
 */
/// Drop the parked `return_to`. Guards `sessionStorage`.
export function clearReturnTo(): void {
    if (typeof sessionStorage === "undefined") return;
    sessionStorage.removeItem(RETURN_TO_STORAGE_KEY);
}
