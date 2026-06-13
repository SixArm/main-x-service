import { test, expect } from '@playwright/test';
import { SEED } from './helpers/seed';

test.describe('iFIT software features', () => {
    test('geofence alerts list a cross-building move', async ({ page }) => {
        await page.goto('/alerts');
        await expect(page.getByRole('heading', { name: 'Geofence alerts' })).toBeVisible();
        // Seed: Mira re-filed Carol's folder from Cabinet A2 (Main Hospital)
        // to Cabinet C1 (Outpatients Wing) — a building-boundary crossing.
        await expect(page.getByText(SEED.patients.carol.name).first()).toBeVisible();
    });

    test('reports page renders KPIs and cabinet utilisation', async ({ page }) => {
        await page.goto('/reports');
        await expect(page.getByRole('heading', { name: 'Reports' })).toBeVisible();
        await expect(page.getByRole('heading', { name: 'Cabinet utilisation' })).toBeVisible();
        for (const name of SEED.cabinets) {
            await expect(page.getByText(name).first()).toBeVisible();
        }
    });

    test('scan finds a folder by NHS Number and offers a move', async ({ page }) => {
        await page.goto('/scan');
        await page.getByPlaceholder(/Scan or type/).fill(SEED.patients.alice.nhs);
        await page.getByRole('button', { name: 'Scan' }).click();
        await expect(page.getByRole('link', { name: 'Move this folder' }).first()).toBeVisible({
            timeout: 10_000
        });
    });
});
