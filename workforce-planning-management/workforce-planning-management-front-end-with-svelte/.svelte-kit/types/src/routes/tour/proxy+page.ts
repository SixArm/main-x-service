// @ts-nocheck
// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title.
import type { PageLoad } from "./$types";

export const load = () => {
  return { title: "Tour · Main X · WPM" };
};
;null as any as PageLoad;