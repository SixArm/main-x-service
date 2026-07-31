// The BFF's one refusal: preview tokens never reach the browser.
//
// `POST …/variants/{locale}/preview` returns a credential that renders
// unpublished content. If the generic proxy forwarded it, that
// credential would land in client JavaScript — the thing the whole
// session design exists to prevent (`../../spec/auth.md`).

import { describe, expect, it } from "vitest";
import { isPreviewTokenPath } from "$lib/proxy-paths";

describe("the BFF proxy", () => {
  it("refuses the paths that mint or manage preview tokens", () => {
    expect(isPreviewTokenPath("api/entries/abc/variants/en/preview")).toBe(
      true,
    );
    expect(isPreviewTokenPath("/api/entries/abc/variants/fr-CA/preview")).toBe(
      true,
    );
    expect(isPreviewTokenPath("api/entries/abc/variants/en/preview/")).toBe(
      true,
    );
    expect(isPreviewTokenPath("preview-tokens/abc")).toBe(true);
    expect(isPreviewTokenPath("/preview-tokens/abc")).toBe(true);
    expect(isPreviewTokenPath("api/preview-tokens/abc")).toBe(true);
  });

  it("forwards everything else", () => {
    for (const path of [
      "api/sites",
      "api/sites/abc/entries",
      "api/entries/abc/variants/en/revisions",
      "api/sites/abc/insights/health",
      // Not a preview *token* path: a name that merely contains the
      // word must still work, or the refusal becomes a trap.
      "api/sites/abc/entries/preview-of-the-year",
      "api/assets/abc/previews",
    ]) {
      expect(isPreviewTokenPath(path), path).toBe(false);
    }
  });
});
