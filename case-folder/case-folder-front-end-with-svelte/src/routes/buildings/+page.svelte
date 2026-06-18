<script lang="ts">
    // Buildings index (`/buildings`) — list of sites with room counts.
    //
    // Render-only. Reads cached buildings + rooms; `rows` joins each
    // building to its room count for the table.

    import { cache } from '$lib/store/cache.svelte';
    import { t } from '$lib/i18n.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    // Join each building to its room count (counted from the cached rooms).
    const rows = $derived(
        cache.buildings.map((b) => ({
            ...b,
            roomCount: cache.rooms.filter((r) => r.buildingId === b.id).length
        }))
    );
</script>

<BackLink href="/">{t('common.backToDashboard')}</BackLink>

<div class="toolbar">
    <h2>{t('buildings.heading')}</h2>
    <a href="/buildings/new" class="button">{t('buildings.addBuilding')}</a>
</div>

<div class="panel">
    <DataTable label={t('buildings.tableLabel')} caption={t('buildings.tableCaption')}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.name')}</th>
                <th scope="col">{t('buildings.colRooms')}</th>
                <th scope="col">{t('common.description')}</th>
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
                <DataTableRow><DataTableTD colspan={3}>{t('buildings.noBuildings')}</DataTableTD></DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
