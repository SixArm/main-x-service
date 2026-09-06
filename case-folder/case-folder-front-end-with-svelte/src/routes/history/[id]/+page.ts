// Move-event detail (`/history/[id]`) load function.
//
// Loads one move event, then the patient's other folders (looked up by
// the move's NHS Number) so the detail page can offer cross-links. 404
// if the move is unknown, else 503.

import { api, ApiError } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        const move = await api.moves.show(params.id, { fetch });
        // The patient's other folders, so the user can jump across them.
        const siblings = await api.folders.list(
            { nhsNumber: move.nhsNumber },
            { fetch },
        );
        // `page.data.title` convention (see `../../+layout.svelte`).
        return {
            move,
            folders: siblings.items,
            title: `${move.folderTitle} move · Case Tracking`,
        };
    } catch (e) {
        if (e instanceof ApiError && e.status === 404)
            error(404, 'Move event not found');
        error(503, (e as Error).message);
    }
}
