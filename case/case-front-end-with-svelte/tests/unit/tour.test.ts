import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";
import { existsSync } from "node:fs";

vi.mock("$app/environment", () => ({ browser: false }));

import Tour from "../../src/lib/components/Tour.svelte";
import {
  STRING_KEYS,
  LOCALES,
  STRINGS_BY_LOCALE,
} from "../../src/lib/i18n.svelte";

afterEach(cleanup);

const sections = [
  { href: "/new" },
  { href: "/" },
  { href: "/board" },
  { href: "/merge" },
  { href: "/cases" },
  { href: "/audit" },
];

describe("Tour", () => {
  it("renders the opener plus six workflow sections of four steps", () => {
    const { container } = render(Tour, { sections });
    const secs = container.querySelectorAll(
      ".sections > section:not(.closing)",
    );
    expect(secs.length).toBe(7);
    for (const s of secs)
      expect(s.querySelectorAll(".steps li").length).toBe(4);
    expect(container.querySelectorAll("a.open").length).toBe(6);
    expect(container.textContent).not.toMatch(/\btour\.[a-z0-9.]+/);
  });

  it("links every workflow to an existing route", () => {
    for (const { href } of sections) {
      const dir = href === "/" ? "" : href.slice(1);
      expect(
        existsSync(`src/routes/${dir}/+page.svelte`.replace("//", "/")),
      ).toBe(true);
    }
  });

  it("has every tour key in all seven locales", () => {
    const keys = [...STRING_KEYS].filter((k) => k.startsWith("tour."));
    expect(keys.length).toBeGreaterThanOrEqual(37);
    for (const l of LOCALES)
      for (const k of keys) expect(STRINGS_BY_LOCALE[l][k]).toBeTruthy();
  });
});
