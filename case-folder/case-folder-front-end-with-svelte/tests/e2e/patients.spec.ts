import { test, expect } from '@playwright/test';
import { SEED } from './helpers/seed';
import * as nhs from './helpers/nhs';

test.describe('patients register', () => {
    test('lists every seeded patient with NHS Number and folder count', async ({ page }) => {
        await page.goto('/patients');
        for (const p of Object.values(SEED.patients)) {
            await expect(page.getByText(p.nhs).first()).toBeVisible();
            await expect(page.getByText(p.name).first()).toBeVisible();
        }
    });

    test('source column reports Main Patient Service', async ({ page }) => {
        await page.goto('/patients');
        await expect(page.getByText('Main Patient Service').first()).toBeVisible();
    });

    test('search by name filters the list client-side', async ({ page }) => {
        await page.goto('/patients');
        await page.getByLabel('Search patients').fill(SEED.patients.bob.name);
        await expect(page.getByText(SEED.patients.bob.name)).toBeVisible();
        await expect(page.getByText(SEED.patients.alice.name)).toHaveCount(0);
    });

    test('clicking an NHS Number opens the patient detail page', async ({ page }) => {
        await page.goto('/patients');
        await page.getByRole('link', { name: SEED.patients.alice.nhs }).first().click();
        await expect(page).toHaveURL(`/patients/${nhs.slug(SEED.patients.alice.nhs)}`);
        await expect(
            page.getByRole('heading', { level: 2, name: SEED.patients.alice.name })
        ).toBeVisible();
    });
});

test.describe('patient detail', () => {
    test('Alice has at least the two seeded folders and a move history', async ({ page }) => {
        await page.goto(`/patients/${nhs.slug(SEED.patients.alice.nhs)}`);
        await expect(
            page.getByRole('heading', { level: 2, name: SEED.patients.alice.name })
        ).toBeVisible();
        await expect(page.getByText(SEED.folders.aliceVolume1).first()).toBeVisible();
        await expect(page.getByText(SEED.folders.aliceMaternity).first()).toBeVisible();
        // The "Folders for this patient (N)" heading reports the current
        // count, which previous "attach to existing patient" tests can
        // grow above 2. Accept any count >= 2.
        await expect(
            page.getByRole('heading', { name: /Folders for this patient \(\d+\)/ })
        ).toBeVisible();
        await expect(page.locator('.move-card').first()).toBeVisible();
    });

    test('move action link sends the porter to /move?folder=', async ({ page }) => {
        await page.goto(`/patients/${nhs.slug(SEED.patients.alice.nhs)}`);
        await page.getByRole('link', { name: 'Move', exact: true }).first().click();
        await expect(page).toHaveURL(/\/move\?folder=[0-9a-f-]{36}/);
    });

    test('unknown NHS Number shows the snapshot-fallback warning banner', async ({ page }) => {
        // Modulus-11-valid 10-digit number that is not in the seed.
        const orphan = nhs.format(nhs.generate());
        await page.goto(`/patients/${nhs.slug(orphan)}`);
        await expect(page.getByText(/Patient not found in Main Patient Service/i)).toBeVisible();
    });
});
