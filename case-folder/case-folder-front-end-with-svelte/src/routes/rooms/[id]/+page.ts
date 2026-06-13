import { api, ApiError } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        const [show, history] = await Promise.all([
            api.places.show(params.id, { fetch }),
            api.places.history(params.id, { fetch })
        ]);
        return { place: show.place, presences: history.presences };
    } catch (e) {
        if (e instanceof ApiError && e.status === 404) error(404, 'Room not found');
        error(503, (e as Error).message);
    }
}
