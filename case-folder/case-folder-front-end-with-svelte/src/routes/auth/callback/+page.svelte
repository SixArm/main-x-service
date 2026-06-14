<script lang="ts">
    // Magic-link callback (`/auth/callback`) view.
    //
    // The real work (token verification + redirect) happens in the load
    // function; on success the user never lingers here. This page only
    // renders while verifying, or shows the load function's error message
    // (with a link back to /login) if the link could not be used.

    import Alert from '$lib/components/Alert/Alert.svelte';

    let { data } = $props();
</script>

<h2>Signing you in</h2>

{#if data?.error}
    <Alert type="error" heading="Sign-in link could not be used">{data.error}</Alert>
    <p><a href="/login">Back to sign in</a></p>
{:else}
    <p>One moment — completing your sign-in…</p>
{/if}
