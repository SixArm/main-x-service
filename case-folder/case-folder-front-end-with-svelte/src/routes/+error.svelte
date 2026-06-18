<script lang="ts">
    // Error boundary (`+error.svelte`) — shown when a load function throws
    // (e.g. the 503 raised when the Loco API is unreachable). Surfaces the
    // thrown message and a hint about the `/api` proxy / running the backend.

    import { page } from '$app/state';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import { t } from '$lib/i18n.svelte';
</script>

<BackLink href="/">{t('error.backToDashboard')}</BackLink>

<Alert type="error" heading={t('error.heading')}>
    <p>{page.error?.message ?? t('error.unknown')}</p>
    <p>{t('error.apiHint')}</p>
</Alert>
