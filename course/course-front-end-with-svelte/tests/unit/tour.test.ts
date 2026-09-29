import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup } from "@testing-library/svelte";

vi.mock("$app/environment", () => ({ browser: false }));

import Tour from "../../src/lib/components/Tour.svelte";

afterEach(cleanup);

const sections = [
  { href: "/courses/new" },
  { href: "/courses" },
  { href: "/courses/match" },
  { href: "/courses/merge" },
  { href: "/board" },
  { href: "/calendar" },
];

// Pins: /tour is static copy (no data fetching, no session), so it renders
// for an anonymous visitor: opener + six workflows x four steps, each
// workflow linking to its real screen, and no raw `tour.*` key leaks.
describe("Tour page", () => {
  it("renders 7 sections of 4 steps with the six screen links", () => {
    const { container } = render(Tour, { sections });
    expect(container.querySelectorAll("section[id]").length).toBe(7);
    expect(container.querySelectorAll(".steps").length).toBe(7);
    expect(container.querySelectorAll(".steps li").length).toBe(28);
    const hrefs = [...container.querySelectorAll("a.open")].map((a) =>
      a.getAttribute("href"),
    );
    expect(hrefs).toEqual(sections.map((s) => s.href));
    expect(container.textContent).not.toMatch(/tour\.(s\d|start|head)/);
  });
});
