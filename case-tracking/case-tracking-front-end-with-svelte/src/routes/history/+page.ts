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
