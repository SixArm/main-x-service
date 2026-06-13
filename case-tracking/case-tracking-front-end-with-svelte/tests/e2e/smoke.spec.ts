import { test, expect } from '@playwright/test';

test.describe('smoke', () => {
    test('every primary route returns 200 and renders the layout', async ({ page }) => {
        for (const path of [
            '/',
            '/patients',
            '/folders',
            '/buildings',
            '/cabinets',
            '/move',
            '/history'
        ]) {
            const response = await page.goto(path);
            expect(response?.status(), `status of ${path}`).toBe(200);
            await expect(page.getByRole('heading', { level: 1, name: 'Case Tracking' })).toBeVisible();
        }
    });

    test('navigation menu links every primary route', async ({ page }) => {
        await page.goto('/');
        const nav = page.getByRole('navigation', { name: /primary navigation/i });
        const links = ['Dashboard', 'Patients', 'Folders', 'Buildings', 'Cabinets', 'Move folder', 'Move history'];
        for (const name of links) {
            await expect(nav.getByRole('link', { name })).toBeVisible();
        }
    });

    test('skip link is the first interactive element in the DOM', async ({ page }) => {
        await page.goto('/');
        // The skip link must come before the navigation in the DOM so
        // a single Tab keystroke focuses it. Rather than rely on the
        // browser's initial focus position (which varies by automation
        // backend), assert the DOM order directly.
        const links = page.locator('a');
        await expect(links.first()).toHaveText(/skip to main content/i);
    });

    test('aria-current marks the active nav link', async ({ page }) => {
        await page.goto('/folders');
        const nav = page.getByRole('navigation', { name: /primary navigation/i });
        await expect(nav.getByRole('link', { name: 'Folders' })).toHaveAttribute(
            'aria-current',
            'page'
        );
    });
});
