import { test, expect } from '@playwright/test';
import { SEED } from './helpers/seed';
import * as nhs from './helpers/nhs';

test.describe('wired components', () => {
    test('patient page shows the addressograph and action bar', async ({ page }) => {
        await page.goto(`/patients/${nhs.slug(SEED.patients.alice.nhs)}`);
        await expect(page.getByRole('region', { name: 'Patient addressograph' })).toBeVisible();
        await expect(page.getByRole('button', { name: 'Case Notes' })).toBeVisible();
    });

    test('volumes page opens the Labels print dialog', async ({ page }) => {
        await page.goto('/volumes');
        await page.getByRole('button', { name: 'Print labels' }).click();
        const dialog = page.getByRole('dialog', { name: 'Labels' });
        await expect(dialog).toBeVisible();
        await dialog.getByRole('button', { name: 'Close' }).click();
        await expect(dialog).toHaveCount(0);
    });
});
