// Worker detail (`/workers/[id]`) load function.
//
// Returns the worker plus the folders they moved, all folders of their
// patients, and their move log. 404 if unknown, else 503.

import { api, ApiError } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        return await api.workers.show(params.id, { fetch });
    } catch (e) {
        if (e instanceof ApiError && e.status === 404) error(404, 'Worker not found');
        error(503, (e as Error).message);
    }
}
