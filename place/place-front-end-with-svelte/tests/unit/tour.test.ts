import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";

vi.mock("$app/environment", () => ({ browser: false }));

import Tour from "../../src/lib/components/Tour.svelte";
import { LOCALES, translate, type StringKey } from "../../src/lib/i18n.svelte";

afterEach(cleanup);

const sections = [
  { href: "/places/new" },
  { href: "/places" },
  { href: "/places/match" },
  { href: "/review" },
  { href: "/places/merge" },
  { href: "/places" },
];

// The /tour page is public: the component takes no session data and renders
// the opener plus six workflow sections of four steps each.
describe("Tour", () => {
  it("renders the opener and six sections of four steps, with real hrefs", () => {
    const { container } = render(Tour, { sections });
    expect(
      container.querySelectorAll(".sections > section:not(.closing)"),
    ).toHaveLength(7);
    expect(container.querySelectorAll(".steps li")).toHaveLength(28);
    const hrefs = [...container.querySelectorAll("a.open")].map((a) =>
      a.getAttribute("href"),
    );
    expect(hrefs).toEqual(sections.map((s) => s.href));
    expect(container.textContent).not.toMatch(/\btour\.[a-z0-9.]+/);
  });

  it("has every tour key translated in every locale", () => {
    const keys: string[] = ["tour.intro"];
    for (let n = 1; n <= 6; n++) {
      keys.push(`tour.s${n}.title`, `tour.s${n}.summary`);
      for (let s = 1; s <= 4; s++) keys.push(`tour.s${n}.step.${s}`);
    }
    for (const locale of LOCALES) {
      for (const k of keys) {
        const v = translate(k as StringKey, locale);
        expect(v, `${locale} ${k}`).not.toBe(k);
        expect(v.length).toBeGreaterThan(0);
      }
    }
  });
});
