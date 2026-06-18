import { describe, it, expect, afterEach, vi } from "vitest";
import { render, cleanup, fireEvent } from "@testing-library/svelte";
import { createRawSnippet } from "svelte";

// The layout reads `page.url.pathname` for `aria-current`; a static page is
// enough to render the top bar.
vi.mock("$app/state", () => ({
    page: { url: new URL("http://localhost/") },
}));

// Force the non-browser branch so the i18n store seeds from the default
// locale instead of touching `localStorage` at import time.
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
        // The layout reads `data.user` (server-resolved session) for the
        // signed-in badge; a signed-out stub is enough for the nav test.
        const { getByLabelText, container } = render(Layout, {
            children,
            data: { user: null },
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
