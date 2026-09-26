// BFF layout load: expose whether a session is present (resolved
// server-side from the httpOnly cookie) so the chrome can show signed-in
// state without the browser ever holding a token. When signed in, also
// resolves the caller's `view` ABAC attribute (T-28f, repo `tasks.md`
// EV-1) for role-tailored nav ordering / landing route — a presentation
// choice only; every route stays reachable by URL regardless.

import type { LayoutServerLoad } from "./$types";
import { currentUser } from "$lib/server/auth";
import { KNOWN_NAV_HREFS, landingRouteForView, viewAttr } from "$lib/nav";

const DEFAULT_LANDING_ROUTE = "/plans";

export const load: LayoutServerLoad = async ({ locals, fetch }) => {
  if (!locals.sessionId) {
    return { signedIn: false, view: null, landingRoute: DEFAULT_LANDING_ROUTE };
  }
  const user = await currentUser(fetch, locals.sessionId);
  const view = viewAttr(user?.attrs);
  return {
    signedIn: true,
    view,
    landingRoute: landingRouteForView(
      KNOWN_NAV_HREFS,
      view,
      DEFAULT_LANDING_ROUTE,
    ),
  };
};
