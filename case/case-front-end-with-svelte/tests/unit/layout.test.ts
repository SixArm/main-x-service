import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup, fireEvent } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";

// The layout reads `page.url.pathname` for `aria-current`; a static page is
// enough to render the top bar.
vi.mock("$app/state", () => ({
  page: { url: new URL("http://localhost/") },
}));

// The layout now imports the i18n store, which seeds from localStorage behind
// `$app/environment`'s `browser`; stub it to the server value so the render is
// deterministic and the `<html lang>/dir` $effect stays SSR-guarded.
vi.mock("$app/environment", () => ({ browser: false }));

import Layout from "../../src/routes/+layout.svelte";

afterEach(cleanup);

// A minimal routed-page stand-in for the layout's `children` snippet.
const children = createRawSnippet(() => ({
  render: () => `<p data-testid="content">content</p>`,
}));

// The top-bar navigation collapses behind the hamburger on narrow viewports;
// the toggle's contract (driving the CSS) is `aria-expanded` on the button +
// the `open` class on the <nav>. (spec §5 "Layout shell & navigation".)
describe("+layout top-bar navigation", () => {
  it("hamburger toggles nav visibility (aria-expanded + .open)", async () => {
    // The layout reads `data.signedIn` (server-resolved session); a
    // signed-out stub is enough for the nav test.
    const { getByLabelText, container } = render(Layout, {
      children,
      data: { signedIn: false },
    });
    const button = getByLabelText("Toggle navigation");
    const nav = container.querySelector("nav");
    expect(nav).toBeTruthy();

    // Collapsed initially.
    expect(button.getAttribute("aria-expanded")).toBe("false");
    expect(nav!.classList.contains("open")).toBe(false);

    // Open.
    await fireEvent.click(button);
    expect(button.getAttribute("aria-expanded")).toBe("true");
    expect(nav!.classList.contains("open")).toBe(true);

    // Close again.
    await fireEvent.click(button);
    expect(button.getAttribute("aria-expanded")).toBe("false");
    expect(nav!.classList.contains("open")).toBe(false);
  });
});

// The header end (always visible, outside the collapsed nav) carries the
// session control immediately before the Lily PickerBar.
describe("+layout header end", () => {
  it("shows Sign in when signed out and Sign out when signed in", () => {
    const out = render(Layout, { children, data: { signedIn: false } });
    const link = out.container.querySelector(".header-end a.signin");
    expect(link?.getAttribute("href")).toBe("/signin");
    expect(link?.textContent?.trim()).toBe("Sign in");
    expect(out.container.querySelector("nav .session-button")).toBeNull();
    cleanup();

    const inn = render(Layout, { children, data: { signedIn: true } });
    const btn = inn.container.querySelector(".header-end form button");
    expect(btn?.textContent?.trim()).toBe("Sign out");
  });
});
