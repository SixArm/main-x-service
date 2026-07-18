// Server hooks: read the (future) session cookie into locals so the BFF
// proxy can exchange it for a PASETO once the family session flow is
// wired (PF-T18). With auth enforcement off (the shipped default) the
// cookie is simply absent and the proxy forwards unauthenticated.

import type { Handle } from "@sveltejs/kit";

export const handle: Handle = async ({ event, resolve }) => {
  event.locals.sessionId = event.cookies.get("__Host-mxi_session") ?? null;
  return resolve(event);
};
