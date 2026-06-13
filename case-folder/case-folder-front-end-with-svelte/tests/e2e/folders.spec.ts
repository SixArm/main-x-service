import { test, expect, type Page } from '@playwright/test';
import { SEED } from './helpers/seed';
import * as nhs from './helpers/nhs';
import { unique } from './helpers/unique';
import { fieldControl } from './helpers/forms';

test.describe('folders register', () => {
    test('lists seeded folders', async ({ page }) => {
        await page.goto('/folders');
        await expect(page.getByText(SEED.folders.aliceVolume1)).toBeVisible();
        await expect(page.getByText(SEED.folders.bobCardiology)).toBeVisible();
        await expect(page.getByText(SEED.folders.carolGeneral).first()).toBeVisible();
    });

    test('search by folder title narrows the list', async ({ page }) => {
        await page.goto('/folders');
        await page.getByLabel('Search folders').fill('Maternity');
        await expect(page).toHaveURL(/q=Maternity/);
        await expect(page.getByText(SEED.folders.aliceMaternity)).toBeVisible();
        await expect(page.getByText(SEED.folders.bobCardiology)).toHaveCount(0);
    });

    test('search by patient name narrows the list', async ({ page }) => {
        await page.goto('/folders');
        await page.getByLabel('Search folders').fill('Carol');
        await expect(page).toHaveURL(/q=Carol/);
        await expect(page.getByText(SEED.patients.carol.name)).toBeVisible();
        await expect(page.getByText(SEED.patients.alice.name)).toHaveCount(0);
    });

    test('search with no matches shows the empty-state row', async ({ page }) => {
        await page.goto('/folders');
        await page.getByLabel('Search folders').fill('zzzz-no-such-folder');
        await expect(page).toHaveURL(/q=zzzz-no-such-folder/);
        await expect(page.getByText('No folders match')).toBeVisible();
    });

    test('clicking a folder title opens its detail page', async ({ page }) => {
        await page.goto('/folders');
        await page.getByRole('link', { name: SEED.folders.aliceVolume1 }).first().click();
        await expect(page).toHaveURL(/\/folders\/[0-9a-f-]{36}$/);
        await expect(
            page.getByRole('heading', { level: 2 }).getByText(SEED.folders.aliceVolume1)
        ).toBeVisible();
    });
});

test.describe('folder detail', () => {
    async function gotoFolder(page: Page, title: string): Promise<void> {
        await page.goto('/folders');
        await page.getByRole('link', { name: title }).first().click();
        await expect(page).toHaveURL(/\/folders\/[0-9a-f-]{36}$/);
    }

    test('shows patient + cabinet + status + history', async ({ page }) => {
        await gotoFolder(page, SEED.folders.aliceVolume1);
        await expect(page.getByText(SEED.patients.alice.name).first()).toBeVisible();
        await expect(
            page.getByText('Main Hospital — Ward A Records Room — Cabinet A1').first()
        ).toBeVisible();
        await expect(page.getByText('in-cabinet').first()).toBeVisible();
        await expect(page.getByRole('heading', { name: 'Move history' })).toBeVisible();
        await expect(page.locator('.move-card').first()).toBeVisible();
    });

    test('unknown folder UUID renders the API error page', async ({ page }) => {
        await page.goto('/folders/00000000-0000-0000-0000-000000000000');
        await expect(page.getByText(/Folder not found/i)).toBeVisible();
    });
});

test.describe('add folder', () => {
    test('redirects to /folders/{id} on a successful create', async ({ page }) => {
        const newNhs = nhs.format(nhs.generate());
        const title = unique('E2E Volume');

        await page.goto('/folders/new');
        await page.getByLabel('NHS Number', { exact: true }).fill(newNhs);
        await fieldControl(page, 'Folder title').fill(title);
        await fieldControl(page, 'Patient name').fill('E2E Test Patient');
        await fieldControl(page, 'Date of birth').fill('1990-01-01');
        await page.getByRole('button', { name: 'Save folder' }).click();

        await expect(page).toHaveURL(/\/folders\/[0-9a-f-]{36}$/);
        await expect(page.getByRole('heading', { level: 2 }).getByText(title)).toBeVisible();
        await expect(page.getByText('E2E Test Patient').first()).toBeVisible();
    });

    test('attaches to an existing patient without requiring name/DOB', async ({ page }) => {
        const title = unique('Alice Volume');

        await page.goto('/folders/new');
        await page.getByLabel('NHS Number', { exact: true }).fill(SEED.patients.alice.nhs);
        await fieldControl(page, 'Folder title').fill(title);
        await page.getByRole('button', { name: 'Save folder' }).click();

        await expect(page).toHaveURL(/\/folders\/[0-9a-f-]{36}$/);
        await expect(page.getByText(SEED.patients.alice.name).first()).toBeVisible();
    });

    test('blocks submit on invalid NHS Number with a Modulus 11 error', async ({ page }) => {
        await page.goto('/folders/new');
        await page.getByLabel('NHS Number', { exact: true }).fill(nhs.INVALID);
        await fieldControl(page, 'Folder title').fill(unique('Should not save'));
        await page.getByRole('button', { name: 'Save folder' }).click();

        await expect(page.getByText(/Modulus 11/i)).toBeVisible();
        await expect(page).toHaveURL(/\/folders\/new/);
    });

    test('requires a non-blank title', async ({ page }) => {
        await page.goto('/folders/new');
        await page.getByLabel('NHS Number', { exact: true }).fill(nhs.format(nhs.generate()));
        // Whitespace passes HTML5 `required` but the JS validator
        // trims and rejects.
        await fieldControl(page, 'Folder title').fill('   ');
        await fieldControl(page, 'Patient name').fill('No Title Patient');
        await fieldControl(page, 'Date of birth').fill('1990-01-01');
        await page.getByRole('button', { name: 'Save folder' }).click();

        await expect(page.getByText(/Folder title is required/i)).toBeVisible();
    });
});
