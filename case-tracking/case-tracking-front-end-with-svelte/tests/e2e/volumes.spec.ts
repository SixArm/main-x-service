import { test, expect } from '@playwright/test';
import { SEED } from './helpers/seed';
import { unique } from './helpers/unique';
import { fieldControl } from './helpers/forms';

test.describe('volumes', () => {
    test('list → seeded volume detail lists its folders', async ({ page }) => {
        await page.goto('/volumes');
        await expect(page.getByRole('heading', { name: 'Volumes' })).toBeVisible();

        // The stub seed bundles Alice's two folders into a volume.
        await page.getByRole('link', { name: 'Alice Johnson — Vol 1' }).click();
        await expect(page).toHaveURL(/\/volumes\/[0-9a-f-]{36}$/);
        // Scope to the members table (the folder title also appears in the
        // move-history cards below).
        const members = page.getByLabel('Volume folders');
        await expect(members.getByRole('link', { name: SEED.folders.aliceVolume1 })).toBeVisible();
        await expect(members.getByRole('link', { name: SEED.folders.aliceMaternity })).toBeVisible();
    });

    test('create a volume, add a folder, then move the whole volume', async ({ page }) => {
        const title = unique('Frank Vol');

        await page.goto('/volumes/new');
        await page.getByLabel('NHS Number', { exact: true }).fill(SEED.patients.frank.nhs);
        await fieldControl(page, 'Volume title').fill(title);
        await page.getByRole('button', { name: 'Create volume' }).click();

        await expect(page).toHaveURL(/\/volumes\/[0-9a-f-]{36}$/);
        await expect(page.getByRole('heading', { level: 2, name: title })).toBeVisible();

        // Add the patient's folder to the volume.
        const addForm = page.locator('form[aria-label="Add a folder"]');
        await addForm.locator('select').selectOption({ index: 1 });
        await addForm.getByRole('button', { name: 'Add to volume' }).click();
        await expect(page.getByRole('link', { name: SEED.folders.frankGeneral }).first()).toBeVisible(
            { timeout: 10_000 }
        );

        // Move the whole volume to a cabinet — the member folder follows.
        const moveForm = page.locator('form[aria-label="Move volume"]');
        await moveForm.locator('select').selectOption({ index: 1 });
        await moveForm.getByRole('button', { name: 'Move volume' }).click();
        await expect(page.getByText('in-cabinet').first()).toBeVisible({ timeout: 10_000 });
    });
});
