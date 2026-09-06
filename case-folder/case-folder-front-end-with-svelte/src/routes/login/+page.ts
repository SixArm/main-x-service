// Login (`/login`) load function.
//
// This page has no data to fetch — the form itself calls the API on
// submit — so the only reason for a load function is the
// `page.data.title` convention (see `../+layout.svelte`): it mirrors
// what this page renders as its own heading, and the layout reads it
// for SharePicker.

export function load() {
    return { title: 'Sign in · Case Tracking' };
}
