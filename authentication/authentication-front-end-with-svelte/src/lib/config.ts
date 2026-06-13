// Read via import.meta.env (not $env/dynamic/public) so this module
// also loads cleanly under vitest. 5150 is the auth service's loco
// default dev port.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };

export const API_BASE_URL: string =
    meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:5150";

/// Comma-separated allowlist of operator-app origins that the
/// cross-origin SSO handoff may redirect the access token to (see
/// `$lib/auth/return-to`). Each entry is an exact `scheme://host[:port]`
/// origin. Unset/empty ⇒ same-origin only (no external handoff).
export const RETURN_TO_ALLOWLIST: string =
    meta.env?.VITE_RETURN_TO_ALLOWLIST ?? "";
