# Purpose

> Part of the [Svelte edition specification](index.md). Shared product
> purpose: [root purpose](../../spec/purpose.md).

Track the physical location of paper case-note folders in a UK NHS
hospital setting. Answer fast: _"Where is the paper folder for NHS
Number `XXX XXX XXXX` right now?"_

This subproject demonstrates the **same domain** served by the
[Loco JSON API back-end](../../case-tracker-service-with-rust/spec/index.md),
with a SvelteKit reference UI client. It owns no data — every page
fetches from `/api/*`, hydrates a reactive cache, and renders.
