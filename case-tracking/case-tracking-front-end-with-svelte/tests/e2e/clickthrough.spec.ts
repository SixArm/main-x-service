import { test, expect } from '@playwright/test';

// These rely on the stub-mode seed, which attributes a move by
// "Mira (records)" to Carol Williams' folder (into Cabinet C1).
test.describe('click-through navigation', () => {
    test('worker → folders moved + their patients folders', async ({ page }) => {
        await page.goto('/workers');
        await expect(page.getByRole('heading', { name: 'Workers' })).toBeVisible();

        await page.getByRole('link', { name: 'Mira (records)' }).click();
        await expect(page).toHaveURL(/\/workers\/[0-9a-f-]{36}$/);
        await expect(
            page.getByRole('heading', { name: /Folders moved by this worker/ })
        ).toBeVisible();
        await expect(page.getByRole('heading', { name: /patients' folders/ })).toBeVisible();
        // Mira re-filed Carol Williams' folder in the seed data.
        await expect(page.getByRole('link', { name: 'Carol Williams' }).first()).toBeVisible();
    });

    test('cabinet → folder presence history', async ({ page }) => {
        await page.goto('/cabinets');
        await page.getByRole('link', { name: 'Cabinet C1', exact: true }).click();
        await expect(page).toHaveURL(/\/cabinets\/[0-9a-f-]{36}$/);
        await expect(page.getByRole('heading', { name: 'Folder presence history' })).toBeVisible();
        // Carol's folder was moved into C1.
        await expect(page.getByText('Carol Williams').first()).toBeVisible();
    });

    test('move history row → event detail', async ({ page }) => {
        await page.goto('/history');
        // The first cell of each row is the timestamp, linked to the event.
        await page.locator('tbody tr td a').first().click();
        await expect(page).toHaveURL(/\/history\/[0-9a-f-]{36}$/);
        await expect(page.getByRole('heading', { name: 'Move event' })).toBeVisible();
        await expect(page.getByRole('heading', { name: 'Folder involved' })).toBeVisible();
    });

    test('building → room → presence history', async ({ page }) => {
        await page.goto('/buildings');
        await page.getByRole('link', { name: 'Main Hospital' }).click();
        await page.getByRole('link', { name: 'Ward A Records Room' }).click();
        await expect(page).toHaveURL(/\/rooms\/[0-9a-f-]{36}$/);
        await expect(page.getByRole('heading', { name: 'Folder presence history' })).toBeVisible();
    });
});
