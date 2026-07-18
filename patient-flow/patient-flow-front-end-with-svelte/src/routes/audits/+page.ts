// Audit trail: recent entries, plus the ward-scoped handover query
// (?ward=<pid>&since=<rfc3339>).

import type { PageLoad } from "./$types";
import { getHandover, getRecentAudits, getWards } from "$lib/api/flow";

export const load: PageLoad = async ({ fetch, url }) => {
  const ward = url.searchParams.get("ward");
  const since = url.searchParams.get("since");
  const wards = await getWards(fetch);
  const entries =
    ward && since
      ? await getHandover(ward, since, fetch)
      : await getRecentAudits(fetch);
  return { entries, wards, ward, since };
};
