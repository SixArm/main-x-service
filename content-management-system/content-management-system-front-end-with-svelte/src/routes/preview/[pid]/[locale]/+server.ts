// Server-side preview (CMS-T25; `../../spec/auth.md`).
//
// The authoring UI needs to show an editor what an unpublished revision
// looks like. The service's answer is a preview token — a credential
// that renders unpublished content — and the whole point of the BFF is
// that credentials stay here. So this route does the round trip on the
// server:
//
//   mint a token → render with it → revoke it → return the render
//
// The browser receives the rendered revision and never the token, so
// there is no shareable URL to leak, nothing to paste into a chat, and
// nothing in the client bundle to steal. The generic proxy refuses the
// token endpoints outright for the same reason.
//
// The token is **revoked immediately after use** rather than left to
// expire. Its 15-minute default lifetime exists for a human sharing a
// link; this route has already spent it by the time it returns, and a
// live credential kept for no reason is just exposure.
//
// A revoke that fails is logged, not raised: the render succeeded, the
// token expires on its own, and failing the editor's preview over
// cleanup would trade a real feature for a tidiness that time will
// handle anyway.

import { error, json } from "@sveltejs/kit";
import type { RequestHandler } from "./$types";
import { CMS_API_URL } from "$lib/server/config";
import { exchangeToken } from "$lib/server/auth";

interface IssuedToken {
  pid: string;
  token: string;
  url: string;
  revision_pid: string;
  expires_at: string;
}

export const GET: RequestHandler = async ({ params, url, locals, fetch }) => {
  const site = url.searchParams.get("site");
  if (!site) {
    error(400, { message: "a preview needs its site key (?site=…)" });
  }
  const revision = url.searchParams.get("revision");

  const headers = new Headers({
    "content-type": "application/json",
    "accepts-version": "1.0",
  });
  if (locals.sessionId) {
    const bearer = await exchangeToken(fetch, locals.sessionId);
    if (bearer) headers.set("authorization", `Bearer ${bearer}`);
  }

  const minted = await fetch(
    `${CMS_API_URL}/api/entries/${params.pid}/variants/${params.locale}/preview`,
    {
      method: "POST",
      headers,
      body: JSON.stringify(revision ? { revision_pid: revision } : {}),
    },
  );
  if (!minted.ok) {
    const detail: unknown = await minted.json().catch(() => null);
    error(minted.status, {
      message: "could not open a preview for that revision",
      details: detail,
    });
  }
  const issued = (await minted.json()) as IssuedToken;

  const rendered = await fetch(
    `${CMS_API_URL}/delivery/${encodeURIComponent(site)}/preview/${issued.token}`,
    { headers: { "accepts-version": "1.0" } },
  );
  const body: unknown = rendered.ok ? await rendered.json() : null;

  // Spend once, then withdraw. Best-effort: see the module note.
  const revoked = await fetch(
    `${CMS_API_URL}/api/preview-tokens/${issued.pid}`,
    { method: "DELETE", headers },
  ).catch(() => null);
  if (!revoked?.ok) {
    console.warn(
      `preview token ${issued.pid} was not revoked after use; it expires at ${issued.expires_at}`,
    );
  }

  if (!rendered.ok) {
    error(rendered.status, { message: "the preview link was not honoured" });
  }

  // Unpublished content: never cached, never indexed — the same
  // headers the service sets, restated because this response is a
  // different one on a different origin.
  return json(body, {
    headers: {
      "cache-control": "private, no-store",
      "x-robots-tag": "noindex, nofollow, noarchive",
    },
  });
};
