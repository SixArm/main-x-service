import { test, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Automated accessibility scans (roadmap T-13). Fails on any serious or
// critical WCAG 2.0/2.1 A/AA violation on the primary signed-in routes.
const ROUTES = [
    '/',
    '/patients',
    '/folders',
    '/volumes',
    '/workers',
    '/cabinets',
    '/alerts',
    '/reports',
    '/scan'
];

for (const route of ROUTES) {
    test(`a11y: ${route} has no serious/critical violations`, async ({ page }) => {
        await page.goto(route);
        const results = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa']).analyze();
        const blocking = results.violations.filter(
            (v) => v.impact === 'serious' || v.impact === 'critical'
        );
        const summary = blocking.map((v) => `${v.id} (${v.nodes.length})`).join(', ');
        expect(blocking, `axe violations: ${summary}`).toEqual([]);
    });
}
