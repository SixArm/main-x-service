// IdP-initiated sign-in entry point (EV-2,
// `agents/share/authentication-sessions.md` §7a): a plain browser
// navigation that 303-redirects to the auth service's
// `/api/auth/oidc/login`, carrying this app's origin as `return_url`.
// Opt-in via `PUBLIC_OIDC_SIGNIN_ENABLED` (see `.env.example`).

import type { RequestHandler } from "./$types";
import { error, redirect } from "@sveltejs/kit";
import { PUBLIC_OIDC_SIGNIN_ENABLED } from "$app/env/public";
import { oidcLoginUrl } from "#lib/server/auth.js";

export const GET: RequestHandler = ({ url }) => {
  if (PUBLIC_OIDC_SIGNIN_ENABLED !== "true") {
    error(404, "SSO sign-in is not enabled for this deployment");
  }
  redirect(303, oidcLoginUrl(url.origin), { external: true });
};
