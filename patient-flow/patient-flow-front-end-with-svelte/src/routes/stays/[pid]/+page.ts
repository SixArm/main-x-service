// Stay detail: the MDT view (journey, Red2Green run, flags, audit
// anchor). A sensitive read — the service audits it.

import type { PageLoad } from "./$types";
import { getStay } from "$lib/api/flow";

export const load: PageLoad = async ({ fetch, params }) => {
  const detail = await getStay(params.pid, fetch);
  return { detail };
};
