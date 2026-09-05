// Root sign-in gate (PF-T22): every page except the public sign-in/
// verify routes and the wall-mounted kiosk display requires a session.
// Before this, an anonymous visitor reached every clinical/PII route —
// the whiteboard, stay detail, bed-request board, locate — and only
// discovered they were signed out once an API call silently failed
// through the BFF proxy, rather than being redirected up front. Today's
// only real gate is `PATIENT_FLOW_REQUIRE_AUTH` on the API, which
// defaults off (`agents/share/security.md` §4), so this front-end gate
// was the only thing standing between an anonymous visitor and live
// patient data.
//
// **Kiosk exemption.** `/wards/[pid]/kiosk` (the whiteboard's
// wall-touchscreen render, `+layout.svelte`'s `kiosk` chrome-less mode)
// has no interactive session to sign in with — it is a public display,
// optionally rendered with `?masked=1` to suppress patient-identifying
// fields client-side. Gating it here would break the kiosk entirely, so
// any path ending `/kiosk` is exempt regardless of the `masked` query
// param, matching how `+layout.svelte` already recognises the same
// path shape for its own chrome-less rendering.
//
// Gated at the root (not per-mutation-page, unlike person-front-end's
// narrower guard): this app's pages mix read content with embedded
// actions (bed-request decisions, ward-board moves, …) rather than
// separating reads and writes onto dedicated routes, so there is no
// small "mutation-only" subset to gate instead.
//
// `locals.sessionId` is presence-only (set from the httpOnly cookie in
// `hooks.server.ts`, never re-validated here) — a UX convenience in
// front of the backend's real ABAC enforcement, not a substitute for
// it, matching the family's `requireSignedIn` convention (repo
// `tasks.md` WEB-1 / PRO-H10).

import { redirect } from "@sveltejs/kit";
import type { LayoutServerLoad } from "./$types";

/** Routes reachable with no session. */
const PUBLIC_PATHS = ["/signin", "/verify"];

export const load: LayoutServerLoad = ({ locals, url }) => {
  const isPublic =
    PUBLIC_PATHS.some((path) => url.pathname.startsWith(path)) ||
    url.pathname.endsWith("/kiosk");
  if (!isPublic && locals.sessionId === null) {
    redirect(303, "/signin");
  }
  return { signedIn: locals.sessionId !== null };
};
