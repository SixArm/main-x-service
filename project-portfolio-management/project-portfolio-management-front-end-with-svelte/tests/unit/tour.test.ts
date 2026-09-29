// The public /tour route: six workflow sections of four steps each, every
// tour.* key present in all locales, and every section link a real route.
import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { existsSync } from "node:fs";
import { resolve } from "node:path";

vi.mock("$app/environment", () => ({ browser: false }));

import Tour from "../../src/lib/components/Tour.svelte";
import { LOCALES, STRINGS_BY_LOCALE } from "../../src/lib/i18n.svelte";

afterEach(cleanup);

const SECTIONS = [
  { href: "/plans/new" },
  { href: "/plans" },
  { href: "/plans" },
  { href: "/plans" },
  { href: "/ideas" },
  { href: "/plans" },
];

describe("/tour", () => {
  it("renders the opener plus six sections of four steps, with no raw keys", () => {
    const { container } = render(Tour, { sections: SECTIONS });
    expect(container.querySelectorAll("section[id]").length).toBe(7);
    expect(container.querySelectorAll("ol.steps").length).toBe(7);
    for (const ol of container.querySelectorAll("ol.steps")) {
      expect(ol.querySelectorAll("li").length).toBe(4);
    }
    expect(container.textContent).not.toMatch(/\btour\.[a-z0-9.]+/);
  });

  it("has every tour key in every locale", () => {
    const keys = ["tour.intro", "tour.head", "nav.tour", "splash.hero.tour"];
    for (let n = 1; n <= 6; n++) {
      keys.push(`tour.s${n}.title`, `tour.s${n}.summary`);
      for (let k = 1; k <= 4; k++) keys.push(`tour.s${n}.step.${k}`);
    }
    for (const locale of LOCALES) {
      const strings = STRINGS_BY_LOCALE[locale] as Record<string, string>;
      for (const key of keys)
        expect(strings[key], `${locale} ${key}`).toBeTruthy();
    }
  });

  it("links only to routes that exist", () => {
    for (const { href } of SECTIONS) {
      const dir = resolve(__dirname, "../../src/routes", `.${href}`);
      expect(existsSync(resolve(dir, "+page.svelte")), href).toBe(true);
    }
  });
});
