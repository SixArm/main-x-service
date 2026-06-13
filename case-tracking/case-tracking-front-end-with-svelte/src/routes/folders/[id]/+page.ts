import { api, ApiError } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        const [folder, history] = await Promise.all([
            api.folders.show(params.id, { fetch }),
            api.folders.history(params.id, { fetch })
        ]);
        return { folder, history };
    } catch (e) {
        if (e instanceof ApiError && e.status === 404) {
            error(404, 'Folder not found');
        }
        error(503, (e as Error).message);
    }
}
