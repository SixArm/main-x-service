// Test stub for SvelteKit's `$app/navigation` virtual module. The unit-test
// vitest config aliases `$app/navigation` here so a layout render does not
// require the SvelteKit plugin. `goto` is a no-op in tests.
export function goto(_url: string): Promise<void> {
    return Promise.resolve();
}
