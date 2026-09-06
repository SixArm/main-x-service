// Test shim for the `lily-design-system-svelte-share-picker` package
// (aliased in vitest.config.ts). The real package's `SharePicker` is a
// named export; re-export the generic StubComponent under that name so a
// layout render doesn't depend on the sibling design-system repo being
// resolvable. See StubComponent.svelte for the rationale.
export { default as SharePicker } from './StubComponent.svelte';
