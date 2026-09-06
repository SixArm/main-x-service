// Central handler for an error caught from an ApiClient call (T-29).
//
// Every route today catches its repository calls with the same
// `err instanceof Error ? err.message : String(err)` fallback, which
// surfaces a `401`/`403` as a raw, untranslated error banner rather than
// reacting to it — a session that expires mid-visit (or a
// `THING_REQUIRE_AUTH`-gated `403`) looks the same as any other failure.
//
// `describeApiError` is the one place that distinction is made: it
// redirects on `401` and translates `403`, and otherwise falls back to
// the exact message every route already displayed, so wiring it in is a
// drop-in replacement with no other behaviour change.
import { goto } from "$app/navigation";
import { ApiError } from "./client.js";
import { t } from "$lib/i18n.svelte.js";

/**
 * Turn a caught error into the message a route's error banner should
 * show, reacting to the two auth-specific statuses along the way.
 *
 * - `401` (session missing/expired) — navigates to `/signin` and
 *   returns a translated "session expired" message for the brief
 *   window before the redirect completes.
 * - `403` (valid session, ABAC denied) — returns a translated
 *   access-denied message instead of the server's raw error body.
 * - anything else — the error's own message, exactly as every route's
 *   inline `err instanceof Error ? err.message : String(err)` already
 *   produced.
 *
 * @param err - The value caught from a `ThingRepository`/`ApiClient` call.
 */
export function describeApiError(err: unknown): string {
  if (err instanceof ApiError && err.isUnauthorized) {
    void goto("/signin");
    return t("auth.sessionExpired");
  }
  if (err instanceof ApiError && err.isForbidden) {
    return t("auth.accessDenied");
  }
  return err instanceof Error ? err.message : String(err);
}
