// Kiosk (wall touchscreen) render of the same board, chrome-less.
// `?masked=1` suppresses patient-identifying fields client-side for
// screens visible to visitors (server-side ABAC masking is the real
// control once enforcement is on — spec `auth.md`).

import type { PageLoad } from "./$types";
import { getWhiteboard } from "$lib/api/flow";

export const load: PageLoad = async ({ fetch, params, url }) => {
  const board = await getWhiteboard(params.pid, fetch);
  const masked = url.searchParams.get("masked") === "1";
  return { board, masked };
};
