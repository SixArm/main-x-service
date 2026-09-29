import { test, expect } from '@playwright/test';

// Signed out: the tour is public, like the splash.
test.use({ storageState: { cookies: [], origins: [] } });

test('signed out, /tour renders the walkthrough without redirecting', async ({
    page,
}) => {
    const response = await page.goto('/tour');
    expect(response?.status()).toBe(200);
    await expect(page).toHaveURL(/\/tour$/);
    await expect(
        page.getByRole('heading', { level: 1, name: 'Case Tracking' }),
    ).toBeVisible();
    await expect(page.locator('.tour section[id^="s"]')).toHaveCount(7);
    await expect(page.locator('.tour ol.steps li')).toHaveCount(28);
});

test('the splash "Take the tour" button opens /tour', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: 'Take the tour' }).click();
    await expect(page).toHaveURL(/\/tour$/);
});
