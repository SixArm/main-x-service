// Move history (`/history`) load function.
//
// Reads the optional `q` filter from the URL (shareable/bookmarkable),
// fetches the matching move-event log, hydrates the cache, and echoes
// the query back to the page. 503 on failure.

import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch, url }) {
    const q = url.searchParams.get('q') ?? '';
    try {
        const list = await api.moves.list({ q }, { fetch });
        cache.setMoves(list.items);
        return { query: q };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
