import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup, fireEvent } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { cache } from '$lib/store/cache.svelte';
import Layout from './+layout.svelte';

// `$app/*` and the `lily-design-system-svelte-*` helpers are aliased to test stubs in vitest.config.ts (the
// plain svelte plugin resolves neither). The Lily selects sit in the utility
// row and are irrelevant to the navigation toggle under test.

afterEach(() => {
    cleanup();
    cache.clearUser();
});

// A minimal routed-page stand-in for the layout's `children` snippet.
const children = createRawSnippet(() => ({
    render: () => '<p data-testid="content">content</p>'
}));

// The header navigation collapses behind the hamburger on narrow viewports;
// the toggle's contract (driving the CSS) is `aria-expanded` on the button +
// the `open` class on the <nav>. The header nav only renders for a signed-in
// user, so seed the cache first. (spec ui-conventions "Navigation & layout".)
describe('+layout top navigation', () => {
    it('hamburger toggles nav visibility (aria-expanded + .open)', async () => {
        cache.setUser({ email: 'op@example.test', name: 'Test Operator', role: null });

        const { getByLabelText, container } = render(Layout, { children });
        const button = getByLabelText('Toggle navigation');
        const nav = container.querySelector('nav');
        expect(nav).toBeTruthy();

        // Collapsed initially.
        expect(button.getAttribute('aria-expanded')).toBe('false');
        expect(nav!.classList.contains('open')).toBe(false);

        // Open.
        await fireEvent.click(button);
        expect(button.getAttribute('aria-expanded')).toBe('true');
        expect(nav!.classList.contains('open')).toBe(true);

        // Close again.
        await fireEvent.click(button);
        expect(button.getAttribute('aria-expanded')).toBe('false');
        expect(nav!.classList.contains('open')).toBe(false);
    });
});
