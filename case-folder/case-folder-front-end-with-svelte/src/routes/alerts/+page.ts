import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        return { alerts: await api.alerts.list({ fetch }) };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
