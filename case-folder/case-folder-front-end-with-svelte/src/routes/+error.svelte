<script lang="ts">
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
