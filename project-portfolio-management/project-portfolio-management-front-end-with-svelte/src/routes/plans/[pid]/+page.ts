// `page.data.title` convention (see `../../+layout.svelte`): mirrors this
// route's own <svelte:head><title>{record?.name ?? "Plan"} — Main X</title>
// so SharePicker gets the right title without reading the DOM. Unlike a
// static-title route, this one's title depends on the fetched record, so
// this load fetches the plan itself (same repository call the page's own
// `onMount` makes) rather than just echoing `params.pid`. A fetch failure
// here is swallowed — the page's own `onMount` fetch surfaces the real
// error to the user; this load exists only to source SharePicker's title.

import type { PageLoad } from "./$types";
import { PlanRepository } from "$lib/api/plans";

export const load: PageLoad = async ({ params, fetch }) => {
  const pid = params.pid ?? "";
  const repo = PlanRepository.withFetch(fetch);
  let name: string | undefined;
  try {
    const record = await repo.get(pid);
    name = record?.name;
  } catch {
    // ignore — see comment above
  }
  return { title: `${name ?? "Plan"} — Main X` };
};
