import { test, expect } from '@playwright/test';
import { SEED } from './helpers/seed';
import { unique } from './helpers/unique';
import { fieldControl } from './helpers/forms';

test.describe('buildings', () => {
    test('lists seeded buildings with their room counts', async ({ page }) => {
        await page.goto('/buildings');
        for (const name of SEED.buildings) {
            await expect(page.getByRole('link', { name })).toBeVisible();
        }
    });

    test('clicking a building opens its detail page with rooms', async ({ page }) => {
        await page.goto('/buildings');
        await page.getByRole('link', { name: 'Main Hospital' }).click();
        await expect(page).toHaveURL(/\/buildings\/[0-9a-f-]{36}$/);
        // Room names are links to the room presence page (the building's
        // presence-history table also mentions room names in cabinet labels).
        await expect(page.getByRole('link', { name: 'Ward A Records Room' })).toBeVisible();
        await expect(page.getByRole('link', { name: 'Ward B Records Room' })).toBeVisible();
    });

    test('creating a building redirects to its detail page', async ({ page }) => {
        const name = unique('E2E Building');
        await page.goto('/buildings/new');
        await fieldControl(page, 'Building name').fill(name);
        await fieldControl(page, 'Description').fill('Created by e2e test');
        await page.getByRole('button', { name: 'Save building' }).click();

        await expect(page).toHaveURL(/\/buildings\/[0-9a-f-]{36}$/);
        await expect(page.getByRole('heading', { level: 2, name })).toBeVisible();
        await expect(page.getByText('Created by e2e test')).toBeVisible();
    });

    test('a whitespace-only building name surfaces the JS validation error', async ({ page }) => {
        await page.goto('/buildings/new');
        // Whitespace passes the HTML5 `required` check but is rejected
        // by the JS submit handler.
        await fieldControl(page, 'Building name').fill('   ');
        await page.getByRole('button', { name: 'Save building' }).click();
        await expect(page.getByText(/Building name is required/i)).toBeVisible();
    });

    test('adding a room to a building updates the rooms list', async ({ page }) => {
        const roomName = unique('E2E Room');

        await page.goto('/buildings');
        await page.getByRole('link', { name: 'Main Hospital' }).click();
        await expect(page).toHaveURL(/\/buildings\/[0-9a-f-]{36}$/);

        await fieldControl(page, 'Room name').fill(roomName);
        await page.getByRole('button', { name: 'Save room' }).click();

        await expect(page.getByText(roomName)).toBeVisible({ timeout: 10_000 });
    });
});

test.describe('cabinets', () => {
    test('lists seeded cabinets with building + room columns', async ({ page }) => {
        await page.goto('/cabinets');
        for (const name of SEED.cabinets) {
            await expect(page.getByText(name).first()).toBeVisible();
        }
        await expect(page.getByText('Main Hospital').first()).toBeVisible();
        await expect(page.getByText('Ward A Records Room').first()).toBeVisible();
    });

    test('creating a cabinet returns to /cabinets with the new row', async ({ page }) => {
        const cabName = unique('E2E Cabinet');
        await page.goto('/cabinets/new');
        await fieldControl(page, 'Cabinet label').fill(cabName);
        await fieldControl(page, 'Capacity').fill('50');
        await page.getByRole('button', { name: 'Save cabinet' }).click();

        await expect(page).toHaveURL(/\/cabinets$/);
        await expect(page.getByText(cabName).first()).toBeVisible();
    });

    test('a whitespace-only cabinet label surfaces the JS validation error', async ({ page }) => {
        await page.goto('/cabinets/new');
        await fieldControl(page, 'Cabinet label').fill('   ');
        await page.getByRole('button', { name: 'Save cabinet' }).click();
        await expect(page.getByText(/Cabinet label is required/i)).toBeVisible();
    });
});
