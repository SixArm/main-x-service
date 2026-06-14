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

<BackLink href="/">Back to dashboard</BackLink>

<h2><Icon name="magnifying-glass" /> Scan a folder</h2>
<p>
    Scan a barcode or type an NHS Number (or folder id) to jump straight to a
    folder and record its move — the Scan4Safety fast path. No hardware scanner
    needed; a keyboard-wedge scanner types into the box below.
</p>

{#if errorMsg}
    <Alert type="error" heading="Scan failed">{errorMsg}</Alert>
{/if}

<Form label="Scan" onsubmit={scan}>
    <Field label="Scan or search" description="NHS Number (e.g. 943 476 5919) or a folder id.">
        <TextInput label="Scan or search" bind:value={term} placeholder="Scan or type…" />
    </Field>
    <div class="actions">
        <Button type="submit">Scan</Button>
    </div>
</Form>

{#if searched}
    <div class="panel">
        {#if results.length > 0}
            <h3>Matches ({results.length})</h3>
            <ul class="report-list">
                {#each results as folder (folder.id)}
                    <li>
                        <a href="/folders/{folder.id}">{folder.title}</a>
                        — {folder.patientName}
                        <Badge type={badgeType(folder.status)}>{folder.status}</Badge>
                        <a href="/move?folder={folder.id}" class="button">Move this folder</a>
                    </li>
                {/each}
            </ul>
        {:else}
            <p>No folder found for “{term}”.</p>
        {/if}
    </div>
{/if}
