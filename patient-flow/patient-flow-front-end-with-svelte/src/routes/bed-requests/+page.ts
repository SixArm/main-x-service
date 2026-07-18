// The bed-request demand board.

import type { PageLoad } from "./$types";
import { getBedRequests, getWards } from "$lib/api/flow";

export const load: PageLoad = async ({ fetch }) => {
  const [requests, wards] = await Promise.all([
    getBedRequests("open", fetch),
    getWards(fetch),
  ]);
  return { requests, wards };
};
