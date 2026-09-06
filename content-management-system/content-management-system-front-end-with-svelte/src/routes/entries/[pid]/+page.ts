// `page.data.title` convention (see `../../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM. The svelte:head title is reactive to the
// fetched entry's key once the page's own onMount load resolves
// (`{entry?.key ?? t("nav.entries")}`); this load has no access to that
// client-fetched key, so it uses the record id instead — a stable,
// always-correct identifier for the initial share title.

import type { PageLoad } from "./$types";

export const load: PageLoad = ({ params }) => {
  return { title: `Entry · ${params.pid}` };
};
