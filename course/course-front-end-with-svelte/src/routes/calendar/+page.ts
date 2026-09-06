// `page.data.title` convention (see `../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.

import type { PageLoad } from "./$types";
import { t } from "$lib/i18n.svelte.js";

export const load: PageLoad = () => {
  return { title: `${t("nav.calendar")} — Main X` };
};
