// Which upstream paths the BFF proxy refuses to forward.
//
// A pure predicate in its own module for two reasons: a SvelteKit
// endpoint may only export HTTP verbs and a fixed set of config names,
// so it cannot also export this; and a rule about what must never
// reach the browser deserves a test that does not need a server.

/**
 * Whether `path` is part of the CMS preview-token surface.
 *
 * `POST …/variants/{locale}/preview` returns a raw preview token — a
 * credential that renders unpublished content — and
 * `/api/preview-tokens/…` manages them. Neither may be forwarded to the
 * browser (`../../spec/auth.md`); the app's own `/preview/…` route does
 * the round trip server-side instead.
 *
 * Matching is on the path *segment*, not a substring, so an entry
 * legitimately keyed `preview-of-the-year` still loads. A refusal that
 * catches innocent names would be discovered as a mysterious 403 long
 * after anyone remembered this rule existed.
 */
export function isPreviewTokenPath(path: string): boolean {
  const segments = path.split("/").filter((segment) => segment.length > 0);
  if (segments.length === 0) return false;
  const last = segments[segments.length - 1];
  const first = segments[0];
  const isMintPath = last === "preview" && segments.includes("variants");
  const isTokenPath =
    first === "preview-tokens" || first === "api"
      ? first === "preview-tokens" || segments[1] === "preview-tokens"
      : false;
  return isMintPath || isTokenPath;
}
