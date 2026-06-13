// Read via import.meta.env (not $env/dynamic/public) so this module
// also loads cleanly under vitest. 5150 is the auth service's loco
// default dev port.
const meta = import.meta as ImportMeta & { env?: Record<string, string | undefined> };

export const API_BASE_URL: string =
    meta.env?.PUBLIC_API_BASE_URL ?? "http://localhost:5150";
