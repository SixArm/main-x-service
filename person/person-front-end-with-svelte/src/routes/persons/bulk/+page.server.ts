// Page-visit guard (PRO-H10): this page's only purpose is submitting a
// bulk import/export job — redirect an unauthenticated visitor to /signin
// rather than render a form whose submit would fail. See
// `$lib/server/session.ts::requireSignedIn` for the policy rationale.

import type { PageServerLoad } from "./$types";
import { requireSignedIn } from "$lib/server/session";

// `page.data.title` convention (see `../../+layout.svelte`): mirrors this
// route's own <svelte:head><title> so SharePicker gets the right title
// without reading the DOM.
export const load: PageServerLoad = ({ locals }) => {
  requireSignedIn(locals);
  return { title: "Bulk import/export · Person Service" };
};
