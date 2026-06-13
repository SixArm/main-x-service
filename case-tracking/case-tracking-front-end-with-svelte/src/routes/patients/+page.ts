import { api } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const list = await api.patients.list({}, { fetch });
        cache.setPatients(list.items);
        return {};
    } catch (e) {
        error(503, (e as Error).message);
    }
}
