<script lang="ts">
    import { cache } from '$lib/store/cache.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    const rows = $derived(
        cache.buildings.map((b) => ({
            ...b,
            roomCount: cache.rooms.filter((r) => r.buildingId === b.id).length
        }))
    );
</script>

<BackLink href="/">Back to dashboard</BackLink>

<div class="toolbar">
    <h2>Buildings</h2>
    <a href="/buildings/new" class="button">Add building</a>
</div>

<div class="panel">
    <DataTable label="Buildings" caption="Physical sites holding records rooms">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">Name</th>
                <th scope="col">Rooms</th>
                <th scope="col">Description</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each rows as b (b.id)}
                <DataTableRow>
                    <DataTableTD><a href="/buildings/{b.id}">{b.name}</a></DataTableTD>
                    <DataTableTD>{b.roomCount}</DataTableTD>
                    <DataTableTD>{b.description ?? ''}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if rows.length === 0}
                <DataTableRow><DataTableTD colspan={3}>No buildings yet.</DataTableTD></DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
