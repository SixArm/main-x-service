// Test-only helper: wraps a plain value in a real Svelte 5 `$state`
// reactive proxy, the same way a page's `let x = $state(...)` does. Has
// to live in a `.svelte.ts` file — `$state` is a rune and can only be
// used inside a module the Svelte compiler processes as such — so a
// plain `tests/unit/*.test.ts` file can't construct one directly.
//
// Exists to reproduce, in a unit test, the exact shape a page like
// `/persons/[id]/edit` hands to `createForm`: not a plain object, but a
// `$state`-wrapped one (see `form.svelte.ts`'s `$state.snapshot` fix).

/** Wrap `value` in a `$state` reactive proxy and return it. */
export function reactiveState<T>(value: T): T {
  let state = $state(value);
  return state;
}
