// Alerts (`/alerts`) load function.
//
// Returns the list of geofence-breach alerts (moves that crossed a
// building boundary). Returned as page data rather than cached because
// the alerts page is the only consumer. A failure becomes a 503.

import { api } from '$lib/api/client';
import { error } from '@sveltejs/kit';

export async function load({ fetch }) {
    try {
        return { alerts: await api.alerts.list({ fetch }) };
    } catch (e) {
        error(503, (e as Error).message);
    }
}
