<script lang="ts">
    // Move history (`/history`) — the full move audit log, filterable.
    //
    // Same URL-driven, debounced search pattern as the folders list: the
    // filter lives in `?q=`, so the load function re-fetches and the
    // result is shareable and reload-safe. The table reads `cache.moves`.
    //
    // State: `query` (mirrors the URL term) + the debounce timer handle.

    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';
    import { t } from '$lib/i18n.svelte';

    let { data } = $props();

    let query = $state('');
    // Re-sync the box when load data changes (back/forward navigation).
    $effect.pre(() => {
        query = data.query;
    });
    let debounce: ReturnType<typeof setTimeout> | null = null;

    // Push the trimmed filter into the URL after an idle, driving a
    // refetch. replaceState avoids per-keystroke history; keepFocus holds
    // the cursor in the box.
    function onSearchInput() {
        if (debounce) clearTimeout(debounce);
        debounce = setTimeout(() => {
            const next = query.trim();
            const target = next
                ? `/history?q=${encodeURIComponent(next)}`
                : '/history';
            goto(target, { keepFocus: true, replaceState: true });
        }, 200);
    }
</script>

<BackLink href="/">{t('history.backToDashboard')}</BackLink>

<div class="toolbar">
    <h2>{t('history.heading')}</h2>
    <input
        type="search"
        bind:value={query}
        oninput={onSearchInput}
        placeholder={t('history.filterPlaceholder')}
        aria-label={t('history.filterLabel')}
    />
</div>

<div class="panel">
    <DataTable
        label={t('history.tableLabel')}
        caption={t('history.tableCaption')}
    >
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.when')}</th>
                <th scope="col">{t('common.nhsNumber')}</th>
                <th scope="col">{t('common.patient')}</th>
                <th scope="col">{t('common.from')}</th>
                <th scope="col">{t('common.to')}</th>
                <th scope="col">{t('common.movedBy')}</th>
                <th scope="col">{t('common.reason')}</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each cache.moves as move (move.id)}
                <DataTableRow>
                    <DataTableTD>
                        <a href="/history/{move.id}"
                            >{new Date(move.movedAt).toLocaleString('en-GB')}</a
                        >
                    </DataTableTD>
                    <DataTableTD>
                        <a
                            href="/patients/{move.nhsNumber.replaceAll(
                                ' ',
                                '',
                            )}"
                            class="nhs-number"
                        >
                            {move.nhsNumber}
                        </a>
                    </DataTableTD>
                    <DataTableTD>{move.patientName}</DataTableTD>
                    <DataTableTD>{move.fromCabinetLabel}</DataTableTD>
                    <DataTableTD>{move.toCabinetLabel}</DataTableTD>
                    <DataTableTD>
                        {move.movedBy}{#if move.workerRole}
                            ({move.workerRole}){/if}
                    </DataTableTD>
                    <DataTableTD>{move.reason ?? ''}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if cache.moves.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={7}>{t('history.noMatch')}</DataTableTD
                    >
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
