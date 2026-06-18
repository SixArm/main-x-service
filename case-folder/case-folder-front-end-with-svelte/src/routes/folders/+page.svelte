<script lang="ts">
    // Folders index (`/folders`) — searchable register of every folder.
    //
    // The search box is wired to the URL `?q=` param (the load function
    // re-runs and re-hydrates the cache), so searches are debounced,
    // shareable, and survive reload. The table reads `cache.folders`.
    //
    // State:
    //   query    — mirrors the URL term; seeded from `data.query` and
    //              re-synced via $effect.pre on navigation.
    //   debounce — timer handle for the search input.

    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';
    import { t, statusLabel } from '$lib/i18n.svelte';

    let { data } = $props();

    let query = $state('');
    // Keep the box in sync when the load data changes (e.g. back/forward
    // navigation rewrites `?q=`). `$effect.pre` runs before DOM update.
    $effect.pre(() => {
        query = data.query;
    });
    let debounce: ReturnType<typeof setTimeout> | null = null;

    // Push the trimmed term into the URL after a short idle, driving the
    // load function to refetch. `replaceState` avoids a history entry per
    // keystroke; `keepFocus` keeps the cursor in the search box.
    function onSearchInput() {
        if (debounce) clearTimeout(debounce);
        debounce = setTimeout(() => {
            const next = query.trim();
            const target = next ? `/folders?q=${encodeURIComponent(next)}` : '/folders';
            goto(target, { keepFocus: true, replaceState: true });
        }, 200);
    }

    function badgeType(status: string): 'success' | 'warning' | 'info' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/">{t('common.backToDashboard')}</BackLink>

<div class="toolbar">
    <h2>{t('folders.register')}</h2>
    <div style="display:flex; gap: var(--nhs-space-2);">
        <input
            type="search"
            bind:value={query}
            oninput={onSearchInput}
            placeholder={t('folders.searchPlaceholder')}
            aria-label={t('folders.searchLabel')}
        />
        <a href="/folders/new" class="button">{t('folders.addFolder')}</a>
    </div>
</div>

<div class="panel">
    <DataTable label={t('folders.tableLabel')} caption={t('folders.tableCaption')}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('folders.colNhsNumber')}</th>
                <th scope="col">{t('folders.colPatient')}</th>
                <th scope="col">{t('folders.colFolder')}</th>
                <th scope="col">{t('folders.colCabinet')}</th>
                <th scope="col">{t('folders.colStatus')}</th>
                <th scope="col">{t('folders.colLastMoved')}</th>
                <th scope="col">{t('folders.colAction')}</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each cache.folders as folder (folder.id)}
                <DataTableRow>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(folder.nhsNumber)}" class="nhs-number">
                            {folder.nhsNumber}
                        </a>
                    </DataTableTD>
                    <DataTableTD>{folder.patientName}</DataTableTD>
                    <DataTableTD>
                        <a href="/folders/{folder.id}">{folder.title}</a>
                    </DataTableTD>
                    <DataTableTD>{folder.cabinetLabel}</DataTableTD>
                    <DataTableTD>
                        <Badge type={badgeType(folder.status)}>{statusLabel(folder.status)}</Badge>
                    </DataTableTD>
                    <DataTableTD>
                        {folder.lastMovedAt ? new Date(folder.lastMovedAt).toLocaleString('en-GB') : '—'}
                    </DataTableTD>
                    <DataTableTD>
                        <a href="/move?folder={folder.id}">{t('common.move')}</a>
                    </DataTableTD>
                </DataTableRow>
            {/each}
            {#if cache.folders.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={7}>
                        {t('folders.noMatch')} <strong>{data.query}</strong>.
                    </DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
