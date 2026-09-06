// `page.data.title` convention (see `../../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM. The svelte:head title is reactive to the
// fetched organization's name once the page's own onMount load resolves
// (`{t("edit.title")}: {org?.name ?? t("edit.organizationFallback")} —
// Main X`); this load has no access to that client-fetched name, so it
// uses the record id instead — a stable, always-correct identifier for
// the initial share title.

import type { PageLoad } from "./$types";

export const load: PageLoad = ({ params }) => {
  return { title: `Edit organization: ${params.pid} — Main X` };
};
