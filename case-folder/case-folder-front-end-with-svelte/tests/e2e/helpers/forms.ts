// Form-field selectors.
//
// The Lily `Field` component renders a `<label for=...>` with an
// auto-generated id but doesn't apply that id to the child input, so
// `page.getByLabel(...)` doesn't match for plain inputs / selects /
// textareas wrapped in a Field. This helper locates the field by
// label text and returns the control inside it.

import type { Page, Locator } from '@playwright/test';

export function fieldControl(page: Page, labelText: string): Locator {
    return page
        .locator('.field', { hasText: labelText })
        .locator('input, textarea, select')
        .first();
}
