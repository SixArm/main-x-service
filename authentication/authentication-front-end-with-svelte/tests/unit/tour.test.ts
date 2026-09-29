// The public /tour page: renders anonymously (it is a plain page with no
// server guard), one "Before you begin" opener plus six workflow sections
// of four steps each, and every tour.* key exists in all 7 locales.
import { describe, it, expect, afterEach } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import Tour from "../../src/lib/components/Tour.svelte";
import {
  LOCALES,
  STRINGS_BY_LOCALE,
  translate,
  type StringKey,
} from "$lib/i18n.svelte";

afterEach(cleanup);

const sections = [
  { href: "/signup" },
  { href: "/signin" },
  { href: "/signin" },
  { href: "/" },
  { href: "/" },
  { href: "/admin/attributes" },
];

describe("Tour", () => {
  it("renders the opener plus six sections of four steps", () => {
    const { container } = render(Tour, { sections });
    expect(
      container.querySelectorAll("section:not(#start):not(.closing)").length,
    ).toBe(6);
    expect(container.querySelectorAll("section").length).toBe(8);
    expect(container.querySelectorAll(".steps").length).toBe(7);
    expect(container.querySelectorAll(".steps li").length).toBe(28);
    expect(container.querySelectorAll("a.open").length).toBe(6);
    expect(container.textContent).not.toMatch(/tour\.(s\d|start|head)/);
  });

  it("has every tour key in every locale", () => {
    const keys: string[] = ["tour.intro"];
    for (let n = 1; n <= 6; n++) {
      keys.push(`tour.s${n}.title`, `tour.s${n}.summary`);
      for (let k = 1; k <= 4; k++) keys.push(`tour.s${n}.step.${k}`);
    }
    for (const loc of LOCALES) {
      for (const key of keys) {
        expect(
          (STRINGS_BY_LOCALE[loc] as Record<string, string>)[key],
          `${loc} ${key}`,
        ).toBeTruthy();
      }
    }
    expect(translate("nav.tour" as StringKey, "cy-001")).toBe("Taith");
  });
});
