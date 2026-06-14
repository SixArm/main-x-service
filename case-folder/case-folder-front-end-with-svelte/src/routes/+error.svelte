<script lang="ts">
    // Error boundary (`+error.svelte`) — shown when a load function throws
    // (e.g. the 503 raised when the Loco API is unreachable). Surfaces the
    // thrown message and a hint about the `/api` proxy / running the backend.

    import { page } from '$app/state';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
</script>

<BackLink href="/">Back to dashboard</BackLink>

<Alert type="error" heading="Case Tracking API error">
    <p>{page.error?.message ?? 'Unknown error'}</p>
    <p>
        The app talks to the Loco JSON API through <code>/api</code> (proxied
        to the Loco server in dev). Make sure that API is running. See
        <code>README.md</code> &sect; "Quick start".
    </p>
</Alert>
