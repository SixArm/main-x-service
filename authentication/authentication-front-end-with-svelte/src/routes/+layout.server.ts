// BFF layout load: resolve the signed-in user from the httpOnly session
// cookie (server reads the cookie via `locals`, exchanges it for a PASETO,
// and calls /me) so the whole app can show signed-in state without the
// browser ever holding a token.

import type { LayoutServerLoad } from "./$types";
import { currentUser } from "$lib/server/auth";

export const load: LayoutServerLoad = async ({ locals, fetch }) => {
  const user = locals.sessionId
    ? await currentUser(fetch, locals.sessionId, locals.csrfToken)
    : null;
  return { user };
};
