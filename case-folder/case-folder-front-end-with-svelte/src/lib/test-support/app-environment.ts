// Test stub for SvelteKit's `$app/environment` virtual module. The unit-test
// vitest config (plain svelte plugin) aliases `$app/environment` here so the
// i18n store and the root layout can import `browser` without the SvelteKit
// plugin. `browser` is false in tests, so the i18n store seeds from the
// default locale and the layout's lang/dir effect is a no-op.
export const browser = false;
export const dev = false;
export const building = false;
export const version = 'test';
