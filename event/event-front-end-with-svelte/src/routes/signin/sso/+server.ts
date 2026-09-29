// The IdP-initiated sign-in entry point (EV-2,
// `agents/share/authentication-sessions.md` §7a §3 rollout step 3):
// a plain browser navigation (an <a href>, not a fetch — see
// `+page.svelte`) that this SvelteKit server 303-redirects onward to
// the auth service's own `/api/auth/oidc/login`, carrying this app's
// origin as `return_url` so the post-federation bridge lands back here.
//
// Gated on `PUBLIC_OIDC_SIGNIN_ENABLED` (this app's own opt-in — see
// `.env.example`) so a deployment that has not configured OIDC on the
// auth service does not show a dead link; the auth service itself
// independently 404s `/api/auth/oidc/login` when unconfigured, so this
// is belt-and-suspenders, not the only guard.

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
