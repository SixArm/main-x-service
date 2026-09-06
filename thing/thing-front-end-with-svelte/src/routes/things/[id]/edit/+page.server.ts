// Page-visit guard (PRO-H10): this page's only purpose is submitting an
// update — redirect an unauthenticated visitor to /signin rather than
// render a form whose submit would fail. See
// `$lib/server/session.ts::requireSignedIn` for the policy rationale.

import type { PageServerLoad } from "./$types";
import { requireSignedIn } from "$lib/server/session";

// `page.data.title` convention (see `../../../+layout.svelte`): mirrors
// this route's own <svelte:head><title> so SharePicker gets the right
// title without reading the DOM.
export const load: PageServerLoad = ({ locals, params }) => {
  requireSignedIn(locals);
  return { title: `Edit thing · ${params.id}` };
};
