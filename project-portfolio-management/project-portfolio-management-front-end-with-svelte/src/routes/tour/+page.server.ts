// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title.
import type { PageServerLoad } from "./$types";

export const load: PageServerLoad = () => {
  return { title: "Tour · Main X Plans" };
};
