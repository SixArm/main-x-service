// Folders index (`/folders`) load function.
//
// Reads the optional `q` search term from the URL (so a search is
// shareable/bookmarkable and survives reload), fetches the matching
// folders, hydrates the cache, and echoes the query back to the page.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch, url }) {
    const q = url.searchParams.get('q') ?? '';
    try {
        const list = await api.folders.list({ q }, { fetch });
        cache.setFolders(list.items);
        // `page.data.title` convention (see `../+layout.svelte`).
        return { query: q, title: 'Folders · Case Tracking' };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
