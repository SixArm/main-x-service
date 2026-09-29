import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";

vi.mock("$app/state", () => ({
  page: { url: new URL("http://localhost/tour") },
}));
vi.mock("$app/environment", () => ({ browser: false }));

import TourPage from "../../src/routes/tour/+page.svelte";
import { LOCALES, i18n } from "../../src/lib/i18n.svelte.js";

afterEach(cleanup);

// /tour is public: the page component needs no session, and renders the
// "Before you begin" opener plus six workflow sections of four steps each.
describe("/tour page", () => {
  it("renders seven sections with four steps and no raw keys", () => {
    const { container } = render(TourPage);
    const sections = container.querySelectorAll(".sections > section");
    // 1 opener + 6 workflows + closing CTA
    expect(sections.length).toBe(8);
    const withSteps = container.querySelectorAll("ol.steps");
    expect(withSteps.length).toBe(7);
    withSteps.forEach((ol) => expect(ol.querySelectorAll("li").length).toBe(4));
    expect(container.textContent).not.toMatch(/tour\.[a-z0-9.]+/);
  });

  it("links each workflow to a real screen", () => {
    const { container } = render(TourPage);
    const hrefs = [...container.querySelectorAll("a.open")].map((a) =>
      a.getAttribute("href"),
    );
    expect(hrefs).toEqual([
      "/events/new",
      "/events",
      "/events/match",
      "/events/merge",
      "/calendar",
      "/events",
    ]);
  });

  it("has copy for every locale", () => {
    for (const code of LOCALES) {
      i18n.set(code);
      const { container, unmount } = render(TourPage);
      expect(container.textContent).not.toMatch(/tour\.[a-z0-9.]+/);
      unmount();
    }
    i18n.set("en-001");
  });
});
