import { test, expect } from '@playwright/test';
import { SEED } from './helpers/seed';
import * as nhs from './helpers/nhs';
import { unique } from './helpers/unique';
import { fieldControl } from './helpers/forms';

test.describe('move folder workflow', () => {
    test('entering an NHS Number populates the patient folders pane', async ({ page }) => {
        await page.goto('/move');
        await fieldControl(page, 'Patient NHS Number').fill(SEED.patients.alice.nhs);

        const pane = page.getByRole('heading', { name: 'Patient folders' }).locator('..');
        await expect(pane.getByText(SEED.folders.aliceVolume1).first()).toBeVisible();
        await expect(pane.getByText(SEED.folders.aliceMaternity).first()).toBeVisible();
    });

    test('worker dropdown is present', async ({ page }) => {
        await page.goto('/move');
        await expect(
            fieldControl(page, 'Worker (from Main Worker Service)')
        ).toBeVisible();
    });

    test('cabinet picker contains every seeded cabinet plus "In transit"', async ({ page }) => {
        await page.goto('/move');
        const destination = fieldControl(page, 'Destination');
        await expect(destination).toBeVisible();
        const options = await destination.locator('option').allInnerTexts();
        expect(options.join('\n')).toContain('In transit');
        for (const cab of SEED.cabinets) {
            expect(options.join('\n')).toContain(cab);
        }
    });

    async function selectOptionByText(
        select: ReturnType<typeof fieldControl>,
        match: string | RegExp,
        opts: { timeout?: number } = {}
    ): Promise<void> {
        // selectOption() needs a string label, not a RegExp. Poll the
        // <select>'s options for one whose text matches and pick its
        // value — this also waits out any debounce on the upstream
        // lookup that populates the options.
        const timeout = opts.timeout ?? 5_000;
        const deadline = Date.now() + timeout;
        let value: string | null = null;
        const source = typeof match === 'string' ? match : match.source;
        while (Date.now() < deadline && !value) {
            const handle = await select.elementHandle();
            if (handle) {
                value = await handle.evaluate((el, text) => {
                    const sel = el as HTMLSelectElement;
                    const re = new RegExp(text);
                    const opt = Array.from(sel.options).find((o) =>
                        re.test(o.textContent ?? '')
                    );
                    return opt?.value ?? null;
                }, source);
            }
            if (!value) await new Promise((r) => setTimeout(r, 100));
        }
        if (!value) throw new Error(`No <option> matching ${match} within ${timeout}ms`);
        await select.selectOption(value);
    }

    test('records a move and shows the success alert + updated history', async ({ page }) => {
        const newNhs = nhs.format(nhs.generate());
        const title = unique('Move Test Folder');

        await page.goto('/folders/new');
        await page.getByLabel('NHS Number', { exact: true }).fill(newNhs);
        await fieldControl(page, 'Folder title').fill(title);
        await fieldControl(page, 'Patient name').fill('Move Test Patient');
        await fieldControl(page, 'Date of birth').fill('1990-01-01');
        await page.getByRole('button', { name: 'Save folder' }).click();
        await expect(page).toHaveURL(/\/folders\/[0-9a-f-]{36}$/);
        const folderId = page.url().split('/').pop() ?? '';

        await page.goto(`/move?folder=${folderId}`);
        await fieldControl(page, 'Patient NHS Number').fill(newNhs);
        await selectOptionByText(fieldControl(page, 'Folder'), title);
        await selectOptionByText(fieldControl(page, 'Destination'), /Cabinet A1/);
        await fieldControl(page, 'Moved by (free text)').fill('E2E Porter');
        await fieldControl(page, 'Reason').fill('Outpatient appointment (e2e)');
        await page.getByRole('button', { name: 'Record move' }).click();

        await expect(page.getByText(/Move recorded/i)).toBeVisible();
        await expect(page.getByText(/Cabinet A1/).first()).toBeVisible();

        await page.goto('/history');
        await page.getByLabel('Filter audit log').fill('E2E Porter');
        // The history table columns are When / NHS / Patient /
        // From / To / Moved by / Reason — folder title is not shown
        // here. Assert on the porter + reason instead.
        await expect(page.getByText('E2E Porter').first()).toBeVisible();
        await expect(page.getByText('Outpatient appointment (e2e)').first()).toBeVisible();
    });

    test('"In transit" destination clears the folder cabinet', async ({ page }) => {
        const newNhs = nhs.format(nhs.generate());
        const title = unique('Transit Folder');

        await page.goto('/folders/new');
        await page.getByLabel('NHS Number', { exact: true }).fill(newNhs);
        await fieldControl(page, 'Folder title').fill(title);
        await fieldControl(page, 'Patient name').fill('Transit Patient');
        await fieldControl(page, 'Date of birth').fill('1990-01-01');
        await selectOptionByText(fieldControl(page, 'Initial cabinet'), /Cabinet B1/);
        await page.getByRole('button', { name: 'Save folder' }).click();
        await expect(page).toHaveURL(/\/folders\/[0-9a-f-]{36}$/);
        const folderId = page.url().split('/').pop() ?? '';

        await page.goto(`/move?folder=${folderId}`);
        await fieldControl(page, 'Patient NHS Number').fill(newNhs);
        await selectOptionByText(fieldControl(page, 'Folder'), title);
        await fieldControl(page, 'Destination').selectOption({
            label: 'In transit (porter carrying)'
        });
        await fieldControl(page, 'Moved by (free text)').fill('Transit Porter');
        await page.getByRole('button', { name: 'Record move' }).click();

        await expect(page.getByText(/Move recorded/i)).toBeVisible();
        await expect(page.getByText(/In transit/i).first()).toBeVisible();
    });
});
