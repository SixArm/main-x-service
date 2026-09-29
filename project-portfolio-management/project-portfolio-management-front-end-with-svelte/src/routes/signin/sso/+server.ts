// The IdP-initiated sign-in entry point (EV-2,
// `agents/share/authentication-sessions.md` §7a): a plain browser navigation
// (an <a href>, not a fetch) that this SvelteKit server 303-redirects onward
// to the auth service's `/api/auth/oidc/login`, carrying this app's origin as
// `return_url`. Gated on `PUBLIC_OIDC_SIGNIN_ENABLED` (see `.env.example`).

import type { RequestHandler } from "./$types";
import { error, redirect } from "@sveltejs/kit";
import { env } from "$env/dynamic/public";
import { oidcLoginUrl } from "$lib/server/auth";

export const GET: RequestHandler = ({ url }) => {
  if (env.PUBLIC_OIDC_SIGNIN_ENABLED !== "true") {
    error(404, "SSO sign-in is not enabled for this deployment");
  }
  redirect(303, oidcLoginUrl(url.origin));
};
