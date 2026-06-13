import { test, expect } from '@playwright/test';
import { SEED } from './helpers/seed';

test.describe('audit history', () => {
    test('lists the synthetic "Folder created" moves from seed', async ({ page }) => {
        await page.goto('/history');
        // Every seed folder writes a synthetic creation event, so the
        // patient name shows up at least once.
        await expect(page.getByText(SEED.patients.alice.name).first()).toBeVisible();
        await expect(page.getByText(SEED.patients.bob.name).first()).toBeVisible();
    });

    test('search by patient name filters the audit log via the API', async ({ page }) => {
        await page.goto('/history');
        await page.getByLabel('Filter audit log').fill(SEED.patients.carol.name);
        await expect(page).toHaveURL(/q=/);
        await expect(page.getByText(SEED.patients.carol.name).first()).toBeVisible();
        // Alice shouldn't appear in a Carol-filtered audit log.
        await expect(page.getByText(SEED.patients.alice.name)).toHaveCount(0);
    });

    test('search by NHS Number works', async ({ page }) => {
        await page.goto('/history');
        await page
            .getByLabel('Filter audit log')
            .fill(SEED.patients.eleanor.nhs.replaceAll(' ', ''));
        await expect(page.getByText(SEED.patients.eleanor.name).first()).toBeVisible();
    });

    test('empty filter returns to the full list', async ({ page }) => {
        await page.goto('/history?q=Carol');
        await page.getByLabel('Filter audit log').fill('');
        await expect(page).toHaveURL(/\/history$/);
        // Multiple patients now visible.
        await expect(page.getByText(SEED.patients.alice.name).first()).toBeVisible();
        await expect(page.getByText(SEED.patients.bob.name).first()).toBeVisible();
    });

    test('renders at least the seeded synthetic creation events', async ({ page }) => {
        await page.goto('/history');
        // Wait for at least one seeded row to render.
        await expect(page.getByText(SEED.patients.alice.name).first()).toBeVisible();
        // Many rows carry the seed-task reason "Folder created".
        const cells = page.getByRole('gridcell', { name: 'Folder created' });
        expect(await cells.count()).toBeGreaterThan(0);
    });
});
