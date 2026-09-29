// The public /tour walkthrough: every section has copy in every locale, and
// the route is exempt from the signed-out redirect.
import { describe, expect, it, vi } from "vitest";

vi.mock("$app/environment", () => ({ browser: false }));

import { LOCALES, translate, type StringKey } from "../../src/lib/i18n.svelte";
import { load } from "../../src/routes/+layout.server";

const key = (k: string) => k as StringKey;

describe("tour copy", () => {
  it("has a title, summary and four steps for each of six sections, in every locale", () => {
    for (const locale of LOCALES) {
      for (let n = 1; n <= 6; n++) {
        for (const suffix of [
          "title",
          "summary",
          "step.1",
          "step.2",
          "step.3",
          "step.4",
        ]) {
          const k = `tour.s${n}.${suffix}`;
          const text = translate(key(k), locale);
          expect(text, `${locale} ${k}`).not.toBe(k);
          expect(text.length).toBeGreaterThan(3);
        }
      }
      expect(translate(key("tour.intro"), locale)).not.toBe("tour.intro");
      expect(translate("nav.tour", locale)).not.toBe("nav.tour");
    }
  });
});

describe("tour route gate", () => {
  const run = (pathname: string) =>
    load({
      locals: { sessionId: null },
      url: new URL(`http://localhost${pathname}`),
    } as never);

  it("renders for a signed-out visitor", () => {
    expect(run("/tour")).toEqual({ signedIn: false });
  });

  it("still redirects other signed-out routes", () => {
    expect(() => run("/wards")).toThrow();
  });
});
