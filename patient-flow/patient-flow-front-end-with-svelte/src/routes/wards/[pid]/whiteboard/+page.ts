// Ward whiteboard: server-load the first render; the component polls.

import type { PageLoad } from "./$types";
import { getWhiteboard } from "$lib/api/flow";

export const load: PageLoad = async ({ fetch, params }) => {
  const board = await getWhiteboard(params.pid, fetch);
  return { board };
};
