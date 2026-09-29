import { error, redirect } from "@sveltejs/kit";
import { p as public_env } from "../../../../chunks/shared-server.js";
import { o as oidcLoginUrl } from "../../../../chunks/auth.js";
const GET = ({ url }) => {
  if (public_env.PUBLIC_OIDC_SIGNIN_ENABLED !== "true") {
    error(404, "SSO sign-in is not enabled for this deployment");
  }
  redirect(303, oidcLoginUrl(url.origin));
};
export {
  GET
};
