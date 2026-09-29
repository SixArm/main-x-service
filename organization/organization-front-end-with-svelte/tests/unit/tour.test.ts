import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { existsSync } from "node:fs";

vi.mock("$app/environment", () => ({ browser: false }));

import Tour from "../../src/lib/components/Tour.svelte";
import { LOCALES, STRINGS_BY_LOCALE, t } from "../../src/lib/i18n.svelte";

afterEach(cleanup);

// The routes each workflow section links to (mirrors src/routes/tour).
const HREFS = [
  "/new",
  "/organizations",
  "/organizations",
  "/organizations",
  "/review",
  "/merge",
];

describe("/tour", () => {
  it("renders the opener plus six workflows of four steps, anonymously", () => {
    const { container } = render(Tour, {
      sections: HREFS.map((href) => ({ href })),
    });
    expect(container.querySelectorAll("section[id^=s]:not(#start)").length).toBe(6);
    expect(container.querySelector("section#start")).toBeTruthy();
    expect(container.querySelectorAll("ol.steps").length).toBe(7);
    for (const ol of container.querySelectorAll("ol.steps")) {
      expect(ol.querySelectorAll("li").length).toBe(4);
    }
    expect(container.textContent).not.toMatch(/tour\.s\d|tour\.start/);
    expect(container.querySelectorAll("a.open").length).toBe(6);
  });

  it("has every tour.* key in every locale", () => {
    const keys = ["tour.intro"];
    for (let n = 1; n <= 6; n++) {
      keys.push(`tour.s${n}.title`, `tour.s${n}.summary`);
      for (let i = 1; i <= 4; i++) keys.push(`tour.s${n}.step.${i}`);
    }
    for (const loc of LOCALES) {
      for (const k of keys) {
        expect(STRINGS_BY_LOCALE[loc][k], `${loc} ${k}`).toBeTruthy();
      }
    }
    expect(t("nav.tour")).toBeTruthy();
  });

  it("links only to routes that exist", () => {
    for (const href of HREFS) {
      const dir = `src/routes${href}`;
      expect(existsSync(`${dir}/+page.svelte`), href).toBe(true);
    }
  });
});
