import { describe, it, expect, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/svelte';
import Tour from './Tour.svelte';
import { STRING_KEYS } from '#lib/i18n.svelte.js';

afterEach(cleanup);

const sections = [
    { href: '/folders/new' },
    { href: '/folders' },
    { href: '/move' },
    { href: '/scan' },
    { href: '/volumes' },
    { href: '/history' },
];

describe('Tour', () => {
    it('renders the opener plus six sections of four steps, with no raw keys', () => {
        const { container } = render(Tour, { sections });
        expect(container.querySelectorAll('section[id^="s"]')).toHaveLength(7);
        expect(container.querySelectorAll('section ol.steps')).toHaveLength(7);
        expect(container.querySelectorAll('ol.steps li')).toHaveLength(28);
        expect(container.textContent).not.toMatch(/tour\.[a-z0-9.]+/);
    });

    it('links each workflow to its screen', () => {
        const { container } = render(Tour, { sections });
        const hrefs = [...container.querySelectorAll('a.open')].map((a) =>
            a.getAttribute('href'),
        );
        expect(hrefs).toEqual(sections.map((s) => s.href));
    });

    it('defines the full tour.* key set (6 sections x title, summary, 4 steps)', () => {
        const keys = STRING_KEYS.filter((k) => /^tour\.s[1-6]\./.test(k));
        expect(keys.length).toBe(6 * 6);
        expect(STRING_KEYS).toContain('tour.intro');
    });
});
