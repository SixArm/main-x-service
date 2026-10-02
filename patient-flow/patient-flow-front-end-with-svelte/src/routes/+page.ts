// Home: the ward list with live counts (loaded via the proxy) for a
// signed-in operator; nothing is fetched for an anonymous visitor, who
// sees the splash instead.

import type { PageLoad } from "./$types";
import { getAtAGlance } from "#lib/api/flow.js";

export const load: PageLoad = async ({ fetch, parent }) => {
  const { signedIn } = await parent();
  const glance = signedIn ? await getAtAGlance(fetch) : null;
  return { signedIn, glance, title: "Patient Flow" };
};
