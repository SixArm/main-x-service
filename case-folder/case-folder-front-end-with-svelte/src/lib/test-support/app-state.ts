// Test stub for SvelteKit's `$app/state` virtual module. The unit-test
// vitest config (plain svelte plugin) aliases `$app/state` here so a layout
// render does not require the SvelteKit plugin. Only `page.url` is read.
export const page = { url: new URL('http://localhost/') };
