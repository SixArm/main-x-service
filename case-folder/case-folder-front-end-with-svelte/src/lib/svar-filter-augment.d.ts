// Upstream packaging gap (SVAR 2.6): @svar-ui/filter-store's package
// `types` field points at dist/types/index.d.ts, but the published
// tarball ships dist/types/src/index.d.ts — so the
// `export * from "@svar-ui/filter-store"` re-export in
// @svar-ui/svelte-filter's declarations contributes nothing and
// `createArrayFilter` appears untyped. This file is a **module
// augmentation** (the import below makes it merge rather than shadow)
// adding the one helper this app uses; delete it when upstream fixes
// the types path.
import '@svar-ui/svelte-filter';

declare module '@svar-ui/svelte-filter' {
    /**
     * Compile a FilterBar / FilterBuilder rule tree into an array
     * transform: returns a function that filters a row array.
     */
    export function createArrayFilter<T = Record<string, unknown>>(
        rules: unknown,
    ): (rows: T[]) => T[];
}
