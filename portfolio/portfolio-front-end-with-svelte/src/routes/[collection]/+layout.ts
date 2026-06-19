// Validate the `{collection}` route segment for every nested page; an
// unknown collection is a 404. Returns the narrowed collection so child
// pages get it type-safely via `data.collection`.

import { error } from "@sveltejs/kit";
import { isCollection, type Collection } from "$lib/api/types";
import type { LayoutLoad } from "./$types";

export const load: LayoutLoad = ({ params }): { collection: Collection } => {
  if (!isCollection(params.collection)) {
    error(404, `unknown collection: ${params.collection}`);
  }
  return { collection: params.collection };
};
