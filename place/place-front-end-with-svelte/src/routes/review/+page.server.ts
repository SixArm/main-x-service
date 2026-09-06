// Page-visit guard (PRO-H10): this page's only purpose is recording
// review-queue decisions (confirm/reject) — redirect an unauthenticated
// visitor to /signin rather than render a UI whose only actions would
// fail. See `$lib/server/session.ts::requireSignedIn` for the policy
// rationale.

import type { PageServerLoad } from "./$types";
import { requireSignedIn } from "$lib/server/session";

export const load: PageServerLoad = ({ locals }) => {
  requireSignedIn(locals);
};
