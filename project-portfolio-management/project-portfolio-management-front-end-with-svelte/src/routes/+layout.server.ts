// BFF layout load: expose whether a session is present (resolved
// server-side from the httpOnly cookie) so the chrome can show signed-in
// state without the browser ever holding a token.

import type { LayoutServerLoad } from "./$types";

export const load: LayoutServerLoad = ({ locals }) => {
  return { signedIn: locals.sessionId !== null };
};
