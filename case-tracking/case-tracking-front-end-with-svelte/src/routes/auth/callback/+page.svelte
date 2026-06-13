<script lang="ts">
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
