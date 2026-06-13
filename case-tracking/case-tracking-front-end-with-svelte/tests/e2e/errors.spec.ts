import { test, expect } from '@playwright/test';
import * as nhs from './helpers/nhs';
import { unique } from './helpers/unique';
import { fieldControl } from './helpers/forms';

test.describe('API error route', () => {
    test('renders +error.svelte when the API is unreachable', async ({ page, context }) => {
        // Block every /api/* request so the loader throws and SvelteKit
        // renders the error page client-side. Match on the path only —
        // a substring glob like **/api/** would also abort the Vite
        // source module /src/lib/api/client.ts and blank the whole app.
        await context.route(
            (url) => url.pathname.startsWith('/api/'),
            (route) => route.abort('failed')
        );

        await page.goto('/folders');
        await expect(page.getByText(/Case Tracking API error/i)).toBeVisible();
        await expect(page.getByText(/Loco JSON API/i)).toBeVisible();
    });

    test('renders the error page for an unknown folder UUID (404 path)', async ({ page }) => {
        await page.goto('/folders/00000000-0000-0000-0000-000000000000');
        await expect(page.getByText(/Folder not found/i)).toBeVisible();
    });
});

test.describe('NHS Number validation', () => {
    test('the /folders/new form rejects an invalid Modulus 11 number', async ({ page }) => {
        await page.goto('/folders/new');
        await page.getByLabel('NHS Number', { exact: true }).fill(nhs.INVALID);
        await fieldControl(page, 'Folder title').fill(unique('Should not save'));
        await page.getByRole('button', { name: 'Save folder' }).click();
        await expect(page.getByText(/Modulus 11/i)).toBeVisible();
    });

    test('the /move form rejects an invalid NHS Number', async ({ page }) => {
        await page.goto('/move');
        await fieldControl(page, 'Patient NHS Number').fill(nhs.INVALID);
        // The submit button stays disabled until a folder is selected,
        // so dispatch a form submit directly to reach the JS validator.
        await page.locator('form[aria-label="Move folder"]').evaluate((f) =>
            (f as HTMLFormElement).dispatchEvent(new Event('submit', { cancelable: true }))
        );
        await expect(page.getByText(/Enter a valid 10-digit NHS Number/i)).toBeVisible();
    });
});
