import { api, ApiError } from '$lib/api/client';
import { cache } from '$lib/store/cache.svelte';
import { error } from '@sveltejs/kit';

export async function load({ fetch, params }) {
    try {
        const [detail, places] = await Promise.all([
            api.volumes.show(params.id, { fetch }),
            api.places.list({ kind: 'cabinet' }, { fetch })
        ]);
        cache.setCabinets(places.cabinets);
        // Folders that could still be added: this patient's folders not
        // already in the volume.
        const patientFolders = await api.folders.list(
            { nhsNumber: detail.volume.nhsNumber },
            { fetch }
        );
        const candidates = patientFolders.items.filter((f) => f.volumeId !== detail.volume.id);
        return { detail, candidates };
    } catch (e) {
        if (e instanceof ApiError && e.status === 404) error(404, 'Volume not found');
        error(503, (e as Error).message);
    }
}
