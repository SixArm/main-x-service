// The IdP-initiated sign-in entry point (EV-2,
// `agents/share/authentication-sessions.md` §7a): a plain browser
// navigation (an <a href>, not a fetch) that this server 303-redirects
// onward to the auth service's `/api/auth/oidc/login`, carrying this
// app's origin as `return_url`. Gated on `PUBLIC_OIDC_SIGNIN_ENABLED`
// (see `.env.example`) so an unconfigured deployment shows no dead link.

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
