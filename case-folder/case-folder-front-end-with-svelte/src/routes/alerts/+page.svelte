<script lang="ts">
    // Alerts (`/alerts`) — geofence-breach log.
    //
    // Render-only. Lists moves whose origin and destination cabinets sit
    // in different buildings (a folder leaving its building), newest first.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Icon from '$lib/components/Icon/Icon.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/">Back to dashboard</BackLink>

<h2><Icon name="scales" /> Geofence alerts</h2>
<p>
    Case notes that crossed a building boundary. A geofence breach is any
    move whose origin and destination cabinets are in different buildings.
</p>

<div class="panel">
    <DataTable label="Geofence alerts" caption="Boundary-crossing moves, newest first">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">When</th>
                <th scope="col">Folder</th>
                <th scope="col">Patient</th>
                <th scope="col">Crossed</th>
                <th scope="col">Moved by</th>
                <th scope="col">Reason</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.alerts as alert (alert.moveId)}
                <DataTableRow>
                    <DataTableTD>
                        <a href="/history/{alert.moveId}">{new Date(alert.movedAt).toLocaleString('en-GB')}</a>
                    </DataTableTD>
                    <DataTableTD><a href="/folders/{alert.folderId}">{alert.folderTitle}</a></DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(alert.nhsNumber)}">{alert.patientName}</a>
                    </DataTableTD>
                    <DataTableTD>
                        <Badge type="warning">{alert.fromBuilding} → {alert.toBuilding}</Badge>
                    </DataTableTD>
                    <DataTableTD>{alert.movedBy}</DataTableTD>
                    <DataTableTD>{alert.reason ?? ''}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.alerts.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={6}>No geofence breaches — every folder has stayed within its building.</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
