import { redirect } from "@sveltejs/kit";
import { s as signout } from "../../../chunks/auth.js";
import { S as SESSION_COOKIE, a as SESSION_COOKIE_OPTIONS } from "../../../chunks/session.js";
const POST = async ({ locals, cookies, fetch }) => {
  if (locals.sessionId) {
    await signout(fetch, locals.sessionId);
  }
  cookies.delete(SESSION_COOKIE, SESSION_COOKIE_OPTIONS);
  redirect(303, "/");
};
export {
  POST
};
