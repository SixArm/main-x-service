// Sign-out (BFF, PF-T18): best-effort server-side session revocation
// at the authentication service, then clear the httpOnly cookie and
// send the browser home.

import type { RequestHandler } from "./$types";
import { redirect } from "@sveltejs/kit";
import { signout } from "$lib/server/auth";
import { SESSION_COOKIE, SESSION_COOKIE_OPTIONS } from "$lib/server/session";

export const POST: RequestHandler = async ({ locals, cookies, fetch }) => {
  if (locals.sessionId) {
    await signout(fetch, locals.sessionId);
  }
  cookies.delete(SESSION_COOKIE, SESSION_COOKIE_OPTIONS);
  redirect(303, "/");
};
