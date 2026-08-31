// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.

import type { PageLoad } from "./$types";

export const load: PageLoad = () => {
  return { title: "New case — Main X" };
};
