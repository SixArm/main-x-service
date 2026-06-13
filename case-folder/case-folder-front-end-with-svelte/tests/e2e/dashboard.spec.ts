import { test, expect } from '@playwright/test';
import { SEED } from './helpers/seed';

test.describe('dashboard', () => {
    test.beforeEach(async ({ page }) => {
        await page.goto('/');
    });

    test('renders KPI cards with seed values', async ({ page }) => {
        await expect(page.getByRole('heading', { name: 'Patients' })).toBeVisible();
        await expect(page.getByRole('heading', { name: 'In cabinet' })).toBeVisible();
        await expect(page.getByRole('heading', { name: 'In transit' })).toBeVisible();
        await expect(page.getByRole('heading', { name: 'Buildings' })).toBeVisible();
        await expect(page.getByRole('heading', { name: /Moves \(24h\)/ })).toBeVisible();
    });

    test('Patients card reports a positive seeded count', async ({ page }) => {
        const card = page.locator('.metric-grid').getByText('Patients').locator('..');
        const value = await card.locator('.metric-value').first().innerText();
        expect(Number(value)).toBeGreaterThanOrEqual(Object.keys(SEED.patients).length);
    });

    test('Recent moves panel lists at least one seeded move', async ({ page }) => {
        const recent = page.getByRole('heading', { name: 'Recent moves' }).locator('..');
        await expect(recent.locator('.move-card').first()).toBeVisible();
    });

    test('Cabinet utilisation lists every seeded cabinet', async ({ page }) => {
        const util = page.getByRole('heading', { name: 'Cabinet utilisation' }).locator('..');
        for (const name of SEED.cabinets) {
            await expect(util.getByText(name).first()).toBeVisible();
        }
    });

    test('FolderGrid renders rows for seeded folders', async ({ page }) => {
        await expect(page.getByText(SEED.folders.aliceVolume1).first()).toBeVisible();
        await expect(page.getByText(SEED.folders.bobCardiology).first()).toBeVisible();
    });

    test('"Move folder" banner link reaches /move', async ({ page }) => {
        await page.getByRole('link', { name: 'Move folder' }).first().click();
        await expect(page).toHaveURL(/\/move$/);
    });
});
