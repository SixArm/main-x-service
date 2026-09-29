// @ts-nocheck
// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.

import type { PageLoad } from "./$types";

export const load = () => {
  return { title: "Learning — WPM" };
};
;null as any as PageLoad;