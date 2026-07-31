// The endpoint contract, pinned without a running service.
//
// Every path the UI calls is built in `$lib/api/cms`, so this test is
// the one place a reader can check the client against the service's
// OpenAPI document. A path that drifts here fails at runtime with a
// 404 that looks like a backend problem.

import { describe, expect, it, vi } from "vitest";
import * as cms from "$lib/api/cms";

/** A fetch stub that records the URL and answers with `{}`. */
function recorder() {
  const calls: { url: string; method: string; body?: string }[] = [];
  const fetchStub = vi.fn(async (url: string | URL, init?: RequestInit) => {
    calls.push({
      url: String(url),
      method: init?.method ?? "GET",
      body: typeof init?.body === "string" ? init.body : undefined,
    });
    return new Response("{}", {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  });
  return { calls, fetch: fetchStub as unknown as typeof fetch };
}

describe("the CMS API client", () => {
  it("calls the same-origin proxy, never the service directly", async () => {
    const r = recorder();
    await cms.listSites({ fetch: r.fetch });
    await cms.listEntries("site-1", { fetch: r.fetch });
    await cms.throughput("site-1", 30, { fetch: r.fetch });
    for (const call of r.calls) {
      expect(call.url.startsWith("/api/proxy/"), call.url).toBe(true);
    }
  });

  // "Documented" means checked against the service's OpenAPI document
  // **with its methods**, not merely plausible. Two guesses were wrong:
  // `transition` is singular, and `/api/entries/{pid}/variants` is
  // POST-only — there is no variants listing, because the entry read
  // returns them.
  it("builds the documented paths", async () => {
    const r = recorder();
    await cms.listSites({ fetch: r.fetch });
    await cms.listContentTypes("s", { fetch: r.fetch });
    await cms.listEntries("s", { fetch: r.fetch });
    await cms.getEntry("e", { fetch: r.fetch });
    await cms.entryTranslations("e", { fetch: r.fetch });
    await cms.listRevisions("e", "fr-CA", { fetch: r.fetch });
    await cms.publishCheck("e", "en", { fetch: r.fetch });
    await cms.backlog("s", { fetch: r.fetch });
    await cms.localeCoverage("s", { fetch: r.fetch });
    expect(r.calls.map((c) => c.url)).toEqual([
      "/api/proxy/api/sites",
      "/api/proxy/api/sites/s/content-types",
      "/api/proxy/api/sites/s/entries",
      "/api/proxy/api/entries/e",
      "/api/proxy/api/entries/e/translations",
      "/api/proxy/api/entries/e/variants/fr-CA/revisions",
      "/api/proxy/api/entries/e/variants/en/publish-check",
      "/api/proxy/api/sites/s/insights/backlog",
      "/api/proxy/api/sites/s/locale-coverage",
    ]);
  });

  it("sends `base_revision_pid` on every revision write", async () => {
    // Omitting it is how a concurrent edit becomes a silent overwrite
    // instead of a 409 the UI can show as a comparison.
    const r = recorder();
    await cms.createRevision(
      "e",
      "en",
      { base_revision_pid: "rev-1", title: "T", blocks: [] },
      { fetch: r.fetch },
    );
    const call = r.calls[0];
    expect(call?.method).toBe("POST");
    expect(JSON.parse(call?.body ?? "{}")).toMatchObject({
      base_revision_pid: "rev-1",
    });
  });

  it("carries a reason on transitions that require one", async () => {
    const r = recorder();
    await cms.transition("e", "en", "publish", undefined, { fetch: r.fetch });
    await cms.transition("e", "en", "archive", "superseded", {
      fetch: r.fetch,
    });
    expect(JSON.parse(r.calls[0]?.body ?? "{}")).toEqual({ action: "publish" });
    expect(JSON.parse(r.calls[1]?.body ?? "{}")).toEqual({
      action: "archive",
      reason: "superseded",
    });
  });

  it("fetches a preview from this app's server, not the proxy", async () => {
    // The proxy refuses the token endpoints; the preview round trip
    // happens server-side so the token never reaches the browser.
    const r = recorder();
    await cms.preview("e", "en", "demo", "rev-2", { fetch: r.fetch });
    const url = r.calls[0]?.url ?? "";
    expect(url.startsWith("/preview/e/en?")).toBe(true);
    expect(url).toContain("site=demo");
    expect(url).toContain("revision=rev-2");
    expect(url).not.toContain("/api/proxy");
  });
});
