// BFF entry point: read the httpOnly session cookie on every request
// and expose the opaque session id to server endpoints via `locals`.
// The browser never reads the cookie; only the SvelteKit server does,
// and only the server talks to the services (CMS-T25, per
// `agents/share/authentication-sessions.md`).

import type { Handle } from "@sveltejs/kit";
import { SESSION_COOKIE } from "$lib/server/session";

export const handle: Handle = async ({ event, resolve }) => {
  event.locals.sessionId = event.cookies.get(SESSION_COOKIE) ?? null;
  return resolve(event);
};
