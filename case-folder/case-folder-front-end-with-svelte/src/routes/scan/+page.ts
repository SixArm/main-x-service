// Scan (`/scan`) load function.
//
// This page has no data to fetch — a scan/type submits directly to the
// API — so the only reason for a load function is the `page.data.title`
// convention (see `../+layout.svelte`): it mirrors what this page
// renders as its own heading, and the layout reads it for SharePicker.

export function load() {
    return { title: 'Scan · Case Tracking' };
}
