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
    render: () => '<p data-testid="content">content</p>',
}));

// The header navigation collapses behind the hamburger on narrow viewports;
// the toggle's contract (driving the CSS) is `aria-expanded` on the button +
// the `open` class on the <nav>. The header nav only renders for a signed-in
// user, so seed the cache first. (spec ui-conventions "Navigation & layout".)
describe('+layout top navigation', () => {
    it('hamburger toggles nav visibility (aria-expanded + .open)', async () => {
        cache.setUser({
            email: 'op@example.test',
            name: 'Test Operator',
            role: null,
        });

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

// The signed-in utility row renders `<name> (<role>)` when the cached user
// carries a role, and just `<name>` (no parenthetical) when it doesn't —
// this was previously unasserted (ST-18): `layout.test.ts` already seeded
// `role: null` for the hamburger test above without ever checking the
// rendered text either way.
describe('+layout signed-in role suffix', () => {
    it('renders "(role)" next to the name when the user carries a role', () => {
        cache.setUser({
            email: 'op@example.test',
            name: 'Test Operator',
            role: 'clerk',
        });

        const { container } = render(Layout, { children });
        const status = container.querySelector('.auth-status');
        expect(status?.textContent).toContain('Test Operator(clerk)');
    });

    it('renders no parenthetical when the user has no role', () => {
        cache.setUser({
            email: 'op@example.test',
            name: 'Test Operator',
            role: null,
        });

        const { container } = render(Layout, { children });
        const status = container.querySelector('.auth-status');
        expect(status?.textContent).toContain('Test Operator');
        expect(status?.textContent).not.toContain('(');
    });
});
