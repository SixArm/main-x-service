<script lang="ts">
    // Scan (`/scan`) — the Scan4Safety fast path to a folder.
    //
    // A keyboard-wedge barcode scanner (or manual typing) feeds the single
    // input; submitting routes to the matching folder so a move can be
    // recorded. Accepts either a folder UUID (barcode encodes the id) or
    // an NHS Number; the input is classified by shape and queried on the
    // right endpoint. No scanner hardware/integration is required — the
    // scanner just types digits into the box.
    //
    // State:
    //   term     — the scanned/typed search string.
    //   results  — matching folders (one for a UUID, possibly many by NHS).
    //   searched — true once a scan completed, to switch "no results" copy.
    //   errorMsg — non-404 API failure to surface in an alert.

    import { api, ApiError } from '$lib/api/client';
    import { formatNhsNumber } from '$lib/store/nhs';
    import type { Folder } from '$lib/store/types';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Icon from '$lib/components/Icon/Icon.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import TextInput from '$lib/components/TextInput/TextInput.svelte';
    import { t, tf, statusLabel } from '$lib/i18n.svelte';

    let term = $state('');
    let results = $state<Folder[]>([]);
    let searched = $state(false);
    let errorMsg = $state('');

    // Distinguishes a scanned folder id (UUID v4 shape) from an NHS Number
    // so `scan()` can pick the exact-show vs. NHS-list endpoint.
    const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

    function badgeType(status: string): 'success' | 'warning' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    async function scan() {
        errorMsg = '';
        results = [];
        searched = false;
        const raw = term.trim();
        if (!raw) return;
        try {
            if (UUID.test(raw)) {
                // Barcode encoded the folder id — fetch that one folder.
                results = [await api.folders.show(raw)];
            } else {
                // Treat the input as an NHS Number; normalise then list.
                const list = await api.folders.list({ nhsNumber: formatNhsNumber(raw) });
                results = list.items;
            }
            searched = true;
        } catch (e) {
            // A 404 is a valid "no match" outcome, not an error to show.
            if (e instanceof ApiError && e.status === 404) {
                searched = true;
            } else {
                errorMsg = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/">{t('common.backToDashboard')}</BackLink>

<h2><Icon name="magnifying-glass" /> {t('scan.heading')}</h2>
<p>{t('scan.intro')}</p>

{#if errorMsg}
    <Alert type="error" heading={t('scan.failed')}>{errorMsg}</Alert>
{/if}

<Form label={t('scan.formLabel')} onsubmit={scan}>
    <Field label={t('scan.fieldLabel')} description={t('scan.fieldDescription')}>
        <TextInput label={t('scan.fieldLabel')} bind:value={term} placeholder={t('scan.placeholder')} />
    </Field>
    <div class="actions">
        <Button type="submit">{t('scan.formLabel')}</Button>
    </div>
</Form>

{#if searched}
    <div class="panel">
        {#if results.length > 0}
            <h3>{tf('scan.matches', { n: results.length })}</h3>
            <ul class="report-list">
                {#each results as folder (folder.id)}
                    <li>
                        <a href="/folders/{folder.id}">{folder.title}</a>
                        — {folder.patientName}
                        <Badge type={badgeType(folder.status)}>{statusLabel(folder.status)}</Badge>
                        <a href="/move?folder={folder.id}" class="button">{t('scan.moveThisFolder')}</a>
                    </li>
                {/each}
            </ul>
        {:else}
            <p>{tf('scan.noFolderFound', { term })}</p>
        {/if}
    </div>
{/if}
