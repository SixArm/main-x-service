import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        const list = await api.workers.list({}, { fetch });
        return { workers: list.items };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
