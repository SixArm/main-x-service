<script lang="ts">
    // Reports (`/reports`) — operational metrics, derived live.
    //
    // Everything here is computed in the component from the load function's
    // page data (stats, folders, moves, cabinets, volumes) — there is no
    // separate reporting store. Covers move throughput (24h / 7d), cabinet
    // utilisation, the in-transit list, and a per-worker activity tally.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Icon from '$lib/components/Icon/Icon.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';
    import { t, tf } from '$lib/i18n.svelte';

    let { data } = $props();

    // Fixed "now" captured at render; `since(h)` is the epoch-ms cutoff
    // for the last `h` hours, used by the throughput windows below.
    const now = Date.now();
    const since = (hours: number) => now - hours * 3600_000;

    // Count moves within the trailing 24-hour and 7-day windows.
    const throughput24h = $derived(
        data.moves.filter((m) => new Date(m.movedAt).getTime() >= since(24)).length
    );
    const throughput7d = $derived(
        data.moves.filter((m) => new Date(m.movedAt).getTime() >= since(24 * 7)).length
    );

    const inTransit = $derived(data.folders.filter((f) => f.status === 'in-transit'));

    // Per-worker activity from the move log: tally moves by `movedBy`,
    // then sort busiest-first for the leaderboard table.
    const perWorker = $derived(
        Object.entries(
            data.moves.reduce<Record<string, number>>((acc, m) => {
                acc[m.movedBy] = (acc[m.movedBy] ?? 0) + 1;
                return acc;
            }, {})
        )
            .map(([worker, count]) => ({ worker, count }))
            .sort((a, b) => b.count - a.count)
    );

    // Occupancy percentage string; "—" when the cabinet is uncapped.
    function utilisation(folderCount: number, capacity: number | null): string {
        if (!capacity) return '—';
        return `${Math.round((folderCount / capacity) * 100)}%`;
    }
</script>

<BackLink href="/">{t('reports.backToDashboard')}</BackLink>

<h2><Icon name="clipboard" /> {t('reports.heading')}</h2>

<div class="panel">
    <h3>{t('reports.atAGlance')}</h3>
    <ul class="report-kpis">
        <li>{t('reports.kpiPatients')} <strong>{data.stats.patients}</strong></li>
        <li>{t('reports.kpiFolders')} <strong>{data.stats.folders.total}</strong> {tf('reports.kpiFoldersDetail', { inCabinet: data.stats.folders.inCabinet, inTransit: data.stats.folders.inTransit })}</li>
        <li>{t('reports.kpiVolumes')} <strong>{data.volumes.length}</strong></li>
        <li>{t('reports.kpiCabinets')} <strong>{data.stats.places.cabinets}</strong> {tf('reports.kpiCabinetsDetail', { n: data.stats.places.buildings })}</li>
        <li>{t('reports.kpiMoves')} <strong>{throughput24h}</strong> · {t('reports.kpiMoves7d')} <strong>{throughput7d}</strong></li>
    </ul>
</div>

<div class="panel">
    <h3>{t('reports.cabinetUtilisation')}</h3>
    <DataTable label={t('reports.cabinetUtilisation')}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('reports.colCabinet')}</th>
                <th scope="col">{t('reports.colFolders')}</th>
                <th scope="col">{t('reports.colCapacity')}</th>
                <th scope="col">{t('reports.colUtilisation')}</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.cabinets as cab (cab.id)}
                <DataTableRow>
                    <DataTableTD><a href="/cabinets/{cab.id}">{cab.label}</a></DataTableTD>
                    <DataTableTD>{cab.folderCount}</DataTableTD>
                    <DataTableTD>{cab.capacity ?? '—'}</DataTableTD>
                    <DataTableTD>{utilisation(cab.folderCount, cab.capacity)}</DataTableTD>
                </DataTableRow>
            {/each}
        </DataTableBody>
    </DataTable>
</div>

<div class="split">
    <div class="panel">
        <h3>{tf('reports.inTransit', { n: inTransit.length })}</h3>
        {#if inTransit.length > 0}
            <ul class="report-list">
                {#each inTransit as folder (folder.id)}
                    <li><a href="/folders/{folder.id}">{folder.title}</a> — {folder.patientName}</li>
                {/each}
            </ul>
        {:else}
            <p>{t('reports.noInTransit')}</p>
        {/if}
    </div>

    <div class="panel">
        <h3>{t('reports.activityByWorker')}</h3>
        {#if perWorker.length > 0}
            <DataTable label={t('reports.activityByWorker')}>
                <DataTableHead>
                    <DataTableRow>
                        <th scope="col">{t('reports.colWorker')}</th>
                        <th scope="col">{t('reports.colMoves')}</th>
                    </DataTableRow>
                </DataTableHead>
                <DataTableBody>
                    {#each perWorker as row (row.worker)}
                        <DataTableRow>
                            <DataTableTD>{row.worker}</DataTableTD>
                            <DataTableTD>{row.count}</DataTableTD>
                        </DataTableRow>
                    {/each}
                </DataTableBody>
            </DataTable>
        {:else}
            <p>{t('reports.noMovesYet')}</p>
        {/if}
    </div>
</div>

<Separator />
<p class="muted">{t('reports.derivedLive')}</p>
