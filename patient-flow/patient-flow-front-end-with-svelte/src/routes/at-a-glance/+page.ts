// Hospital at a glance: site tiles + per-ward rows.

import type { PageLoad } from "./$types";
import { getAtAGlance } from "$lib/api/flow";

export const load: PageLoad = async ({ fetch }) => {
  const glance = await getAtAGlance(fetch);
  return { glance };
};
