// Page-visit guard (PRO-H10): this page's only purpose is submitting a
// bulk import/export job — redirect an unauthenticated visitor to /signin
// rather than render a form whose submit would fail. See
// `$lib/server/session.ts::requireSignedIn` for the policy rationale.

import type { PageServerLoad } from "./$types";
import { requireSignedIn } from "$lib/server/session";

export const load: PageServerLoad = ({ locals }) => {
  requireSignedIn(locals);
};
