<script lang="ts">
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Icon from '$lib/components/Icon/Icon.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();

    const now = Date.now();
    const since = (hours: number) => now - hours * 3600_000;

    const throughput24h = $derived(
        data.moves.filter((m) => new Date(m.movedAt).getTime() >= since(24)).length
    );
    const throughput7d = $derived(
        data.moves.filter((m) => new Date(m.movedAt).getTime() >= since(24 * 7)).length
    );

    const inTransit = $derived(data.folders.filter((f) => f.status === 'in-transit'));

    // Per-worker activity from the move log.
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

    function utilisation(folderCount: number, capacity: number | null): string {
        if (!capacity) return '—';
        return `${Math.round((folderCount / capacity) * 100)}%`;
    }
</script>

<BackLink href="/">Back to dashboard</BackLink>

<h2><Icon name="clipboard" /> Reports</h2>

<div class="panel">
    <h3>At a glance</h3>
    <ul class="report-kpis">
        <li>Patients: <strong>{data.stats.patients}</strong></li>
        <li>Folders: <strong>{data.stats.folders.total}</strong> ({data.stats.folders.inCabinet} in cabinet, {data.stats.folders.inTransit} in transit)</li>
        <li>Volumes: <strong>{data.volumes.length}</strong></li>
        <li>Cabinets: <strong>{data.stats.places.cabinets}</strong> in {data.stats.places.buildings} buildings</li>
        <li>Moves — last 24h: <strong>{throughput24h}</strong> · last 7d: <strong>{throughput7d}</strong></li>
    </ul>
</div>

<div class="panel">
    <h3>Cabinet utilisation</h3>
    <DataTable label="Cabinet utilisation">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">Cabinet</th>
                <th scope="col">Folders</th>
                <th scope="col">Capacity</th>
                <th scope="col">Utilisation</th>
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
        <h3>In transit ({inTransit.length})</h3>
        {#if inTransit.length > 0}
            <ul class="report-list">
                {#each inTransit as folder (folder.id)}
                    <li><a href="/folders/{folder.id}">{folder.title}</a> — {folder.patientName}</li>
                {/each}
            </ul>
        {:else}
            <p>No folders are in transit.</p>
        {/if}
    </div>

    <div class="panel">
        <h3>Activity by worker</h3>
        {#if perWorker.length > 0}
            <DataTable label="Activity by worker">
                <DataTableHead>
                    <DataTableRow>
                        <th scope="col">Worker</th>
                        <th scope="col">Moves</th>
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
            <p>No moves recorded yet.</p>
        {/if}
    </div>
</div>

<Separator />
<p class="muted">Reports are derived live from the API — no separate reporting store.</p>
