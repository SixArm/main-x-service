// Home: the ward list with live counts (server-loaded via the proxy).

import type { PageLoad } from "./$types";
import { getAtAGlance } from "$lib/api/flow";

export const load: PageLoad = async ({ fetch }) => {
  const glance = await getAtAGlance(fetch);
  return { glance };
};
