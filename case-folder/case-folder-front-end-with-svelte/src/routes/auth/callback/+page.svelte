<script lang="ts">
    // Magic-link callback (`/auth/callback`) view.
    //
    // The real work (token verification + redirect) happens in the load
    // function; on success the user never lingers here. This page only
    // renders while verifying, or shows the load function's error message
    // (with a link back to /login) if the link could not be used.

    import Alert from '$lib/components/Alert/Alert.svelte';
    import { t } from '$lib/i18n.svelte';

    let { data } = $props();
</script>

<h2>{t('callback.title')}</h2>

{#if data?.error}
    <Alert type="error" heading={t('callback.error')}>{data.error}</Alert>
    <p><a href="/login">{t('callback.backToSignIn')}</a></p>
{:else}
    <p>{t('callback.completing')}</p>
{/if}
