// `page.data.title` convention (see `./+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.

import type { PageLoad } from "./$types";

export const load: PageLoad = () => {
  // Sensible for both states: the splash and the dashboard share it.
  return { title: "Main X · CMS" };
};
