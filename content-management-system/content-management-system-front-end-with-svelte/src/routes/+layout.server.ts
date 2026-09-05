// Root sign-in gate (CMS-T31): every page except the public
// sign-in/verify routes requires a session. Before this, a visitor
// with no `locals.sessionId` reached every authoring/asset/workflow
// view and only discovered they were signed out once an API call
// silently failed through the BFF proxy, rather than being redirected
// up front.
//
// `/preview/[pid]/[locale]` is a `+server.ts` endpoint, not a page —
// this layout's `load` never runs for a direct request to it, so it
// needs no explicit exclusion here (and stays whatever it already was:
// this task does not change preview's own auth posture, deliberately —
// see `agents/share/authentication-sessions.md` on preview tokens).
//
// Gated at the root (not per-mutation-page, unlike person-front-end's
// narrower guard): this app's pages mix read content with embedded
// actions (workflow transitions on `/entries/[pid]`, schedule actions,
// …) rather than separating reads and writes onto dedicated routes, so
// there is no small "mutation-only" subset to gate instead.
//
// `locals.sessionId` is presence-only (set from the httpOnly cookie in
// `hooks.server.ts`, never re-validated here) — a UX convenience in
// front of the backend's real ABAC enforcement, not a substitute for
// it, matching the family's `requireSignedIn` convention.

import { redirect } from "@sveltejs/kit";
import type { LayoutServerLoad } from "./$types";

/** Routes reachable with no session. */
const PUBLIC_PATHS = ["/signin", "/verify"];

export const load: LayoutServerLoad = ({ locals, url }) => {
  const isPublic = PUBLIC_PATHS.some((path) => url.pathname.startsWith(path));
  if (!isPublic && locals.sessionId === null) {
    redirect(303, "/signin");
  }
  return { signedIn: locals.sessionId !== null };
};
