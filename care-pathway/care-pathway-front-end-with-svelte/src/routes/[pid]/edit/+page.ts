// `page.data.title` convention (see `../../+layout.svelte`): mirrors this
// route's own <svelte:head><title>, whose real value comes from the
// pathway fetched client-side (`pathway?.name`) with a "care pathway"
// fallback — the load function runs before that fetch, so it falls back
// to the pid-qualified form of the same fallback text.
import type { PageLoad } from "./$types";

export const load: PageLoad = ({ params }) => {
  return { title: `Edit care pathway · ${params.pid} — Main X` };
};
