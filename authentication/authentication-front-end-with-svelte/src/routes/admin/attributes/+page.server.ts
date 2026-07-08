// Operator UI (BFF): view / replace a user's ABAC subject attributes via
// the auth service's admin API. The target user is chosen by `?pid=…`
// (GET) and loaded server-side; saving PUTs the whole map. The admin API
// requires the signed-in operator to carry `access=admin` (else 403,
// surfaced here). No token ever reaches the browser.

import type { Actions, PageServerLoad } from "./$types";
import { fail } from "@sveltejs/kit";
import { getUserAttributes, putUserAttributes } from "$lib/server/admin";
import type { UserAttributes } from "$lib/api/types";

export const load: PageServerLoad = async ({ url, locals, fetch }) => {
  const pid = url.searchParams.get("pid")?.trim() || null;
  if (!pid) return { pid: null, target: null as UserAttributes | null };
  if (!locals.sessionId) {
    return { pid, target: null, error: "Sign in as an admin to manage attributes." };
  }
  const result = await getUserAttributes(
    fetch,
    locals.sessionId,
    locals.csrfToken,
    pid,
  );
  if (!result.ok) {
    return { pid, target: null, error: result.message, status: result.status };
  }
  return { pid, target: result.data };
};

/** Parse the editor text into the string→string-array attribute map, or
 *  throw when the shape is wrong (the message reaches the operator). */
function parseAttributes(raw: string): Record<string, string[]> {
  const parsed: unknown = JSON.parse(raw);
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("must be a JSON object");
  }
  for (const value of Object.values(parsed as Record<string, unknown>)) {
    if (!Array.isArray(value) || value.some((v) => typeof v !== "string")) {
      throw new Error("every value must be an array of strings");
    }
  }
  return parsed as Record<string, string[]>;
}

export const actions: Actions = {
  save: async ({ request, locals, fetch }) => {
    const data = await request.formData();
    const pid = String(data.get("pid") ?? "").trim();
    const raw = String(data.get("attributes") ?? "");
    if (!pid) return fail(400, { message: "Missing target user id." });

    let attributes: Record<string, string[]>;
    try {
      attributes = parseAttributes(raw);
    } catch (e) {
      const why = e instanceof Error ? e.message : "invalid JSON";
      return fail(422, {
        message: `Attributes must be a JSON object of string → string-array (${why}).`,
      });
    }

    if (!locals.sessionId) return fail(401, { message: "Sign in as an admin." });
    const result = await putUserAttributes(
      fetch,
      locals.sessionId,
      locals.csrfToken,
      pid,
      attributes,
    );
    if (!result.ok) return fail(result.status, { message: result.message });
    return { saved: true, attributes: result.data.attributes };
  },
};
