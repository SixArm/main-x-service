import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        return await api.patients.show(params.nhs, { fetch });
    } catch (e) {
        error(503, (e as Error).message);
    }
}
