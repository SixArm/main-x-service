<script lang="ts">
    // Cabinets index (`/cabinets`) — every cabinet with its location and
    // occupancy.
    //
    // Render-only. `rows` walks the place hierarchy (cabinet → room →
    // building) via the cache lookups to resolve names, and computes a
    // utilisation percentage from folderCount / capacity.

    import { cache } from '$lib/store/cache.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    // Resolve each cabinet's room/building names and occupancy percentage.
    const rows = $derived(
        cache.cabinets.map((c) => {
            // null capacity ⇒ uncapped, so skip the percentage maths.
            const cap = c.capacity ?? 0;
            const percent = cap > 0 ? Math.round((c.folderCount / cap) * 100) : 0;
            const room = cache.roomById(c.roomId);
            const building = cache.buildingById(room?.buildingId);
            return {
                ...c,
                percent,
                roomName: room?.name ?? '?',
                buildingName: building?.name ?? '?'
            };
        })
    );

    // Traffic-light occupancy badge: red >85%, amber >60%, else green.
    function utilType(percent: number): 'success' | 'warning' | 'error' {
        if (percent > 85) return 'error';
        if (percent > 60) return 'warning';
        return 'success';
    }
</script>

<BackLink href="/">Back to dashboard</BackLink>

<div class="toolbar">
    <h2>File cabinets</h2>
    <a href="/cabinets/new" class="button">Add cabinet</a>
</div>

<div class="panel">
    <DataTable label="Cabinets" caption="Physical file cabinets, their building/room, and occupancy">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">Label</th>
                <th scope="col">Building</th>
                <th scope="col">Room</th>
                <th scope="col">Capacity</th>
                <th scope="col">Folders</th>
                <th scope="col">Utilisation</th>
                <th scope="col">Description</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each rows as cab (cab.id)}
                <DataTableRow>
                    <DataTableTD><a href="/cabinets/{cab.id}">{cab.label}</a></DataTableTD>
                    <DataTableTD>{cab.buildingName}</DataTableTD>
                    <DataTableTD>{cab.roomName}</DataTableTD>
                    <DataTableTD>{cab.capacity ?? '—'}</DataTableTD>
                    <DataTableTD>{cab.folderCount}</DataTableTD>
                    <DataTableTD>
                        {#if cab.capacity}
                            <Badge type={utilType(cab.percent)}>{cab.percent}%</Badge>
                        {:else}
                            <Badge type="default">—</Badge>
                        {/if}
                    </DataTableTD>
                    <DataTableTD>{cab.description ?? ''}</DataTableTD>
                </DataTableRow>
            {/each}
        </DataTableBody>
    </DataTable>
</div>
