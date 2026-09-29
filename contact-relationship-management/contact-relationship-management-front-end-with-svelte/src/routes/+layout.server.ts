// Root sign-in gate (CRM-T26): every page except the public
// sign-in/verify routes requires a session. Before this, a visitor
// with no `locals.sessionId` reached every page — the deal board,
// ticket queue, dashboard, all of them — and only discovered they
// were signed out once an API call silently failed through the BFF
// proxy, rather than being redirected up front.
//
// Gated at the root (not per-mutation-page, unlike person-front-end's
// narrower guard): this app's pages mix read content with embedded
// actions (moving a deal card, resolving a ticket, …) rather than
// separating reads and writes onto dedicated routes, so there is no
// small "mutation-only" subset to gate instead.
//
// `locals.sessionId` is presence-only (set from the httpOnly cookie in
// `hooks.server.ts`, never re-validated here) — a UX convenience in
// front of the backend's real ABAC enforcement, not a substitute for
// it, matching the family's `requireSignedIn` convention.

import { redirect } from "@sveltejs/kit";
import type { LayoutServerLoad } from "./$types";

/** Routes reachable with no session (prefix match). */
const PUBLIC_PATHS = ["/signin", "/verify"];

/**
 * The home page is public too (exact match, since "/" is a prefix of
 * everything): a signed-out visitor sees the splash page there, and the
 * dashboard's API calls only fire once signed in.
 */
const PUBLIC_EXACT = ["/"];

export const load: LayoutServerLoad = ({ locals, url }) => {
  const isPublic =
    PUBLIC_EXACT.includes(url.pathname) ||
    PUBLIC_PATHS.some((path) => url.pathname.startsWith(path));
  if (!isPublic && locals.sessionId === null) {
    redirect(303, "/signin");
  }
  return { signedIn: locals.sessionId !== null };
};
