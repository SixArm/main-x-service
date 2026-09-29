import { test, expect } from '@playwright/test';

// These run *signed out* — drop the shared authenticated storageState.
test.use({ storageState: { cookies: [], origins: [] } });

const LOGIN_EMAIL = process.env.E2E_LOGIN_EMAIL ?? 'records@example.nhs.uk';

test('signed out, / shows the splash with a sign-in call to action', async ({
    page,
}) => {
    await page.goto('/');
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(
        page.getByRole('link', { name: 'Sign in' }).first(),
    ).toHaveAttribute('href', '/login');
    // The dashboard is not rendered for an anonymous visitor.
    await expect(page.getByText('Folder register')).toHaveCount(0);
});

test('a protected route redirects to /login when signed out', async ({
    page,
}) => {
    await page.goto('/folders');
    await expect(page).toHaveURL(/\/login$/);
    await expect(page.getByRole('heading', { name: 'Sign in' })).toBeVisible();
});

test('sign in via magic link, then sign out', async ({ page }) => {
    await page.goto('/login');

    // Lily's Field doesn't wire <label for>, so target the input directly.
    await page.locator('input[type="email"]').fill(LOGIN_EMAIL);
    await page.getByRole('button', { name: /sign-in link/i }).click();

    // Dev mode exposes the link inline; click it to complete sign-in.
    const magicLink = page.getByTestId('magic-link');
    await expect(magicLink).toBeVisible();
    await magicLink.click();

    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByRole('button', { name: 'Sign out' })).toBeVisible();

    await page.getByRole('button', { name: 'Sign out' }).click();
    await expect(page).toHaveURL(/\/login$/);
});

test('an unknown email does not reveal whether it exists', async ({ page }) => {
    await page.goto('/login');
    await page.locator('input[type="email"]').fill('nobody@example.com');
    await page.getByRole('button', { name: /sign-in link/i }).click();
    // Same confirmation as a known address; no magic link is offered.
    await expect(page.getByText('Check your email')).toBeVisible();
    await expect(page.getByTestId('magic-link')).toHaveCount(0);
});
