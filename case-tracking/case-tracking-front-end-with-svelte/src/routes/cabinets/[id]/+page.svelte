<script lang="ts">
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();

    function badgeType(status: string): 'success' | 'warning' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    function when(iso: string): string {
        return new Date(iso).toLocaleString('en-GB');
    }

    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/cabinets">Back to cabinets</BackLink>

<h2>{data.place.name}</h2>
{#if data.place.container_path}<p>{data.place.container_path}</p>{/if}

<div class="panel">
    <h3>Folders currently in this cabinet ({data.folders.length})</h3>
    {#if data.folders.length > 0}
        <DataTable label="Current folders">
            <DataTableHead>
                <DataTableRow>
                    <th scope="col">Title</th>
                    <th scope="col">Patient</th>
                    <th scope="col">Status</th>
                </DataTableRow>
            </DataTableHead>
            <DataTableBody>
                {#each data.folders as folder (folder.id)}
                    <DataTableRow>
                        <DataTableTD><a href="/folders/{folder.id}">{folder.title}</a></DataTableTD>
                        <DataTableTD>
                            <a href="/patients/{nhsSlug(folder.nhsNumber)}">{folder.patientName}</a>
                        </DataTableTD>
                        <DataTableTD>
                            <Badge type={badgeType(folder.status)}>{folder.status}</Badge>
                        </DataTableTD>
                    </DataTableRow>
                {/each}
            </DataTableBody>
        </DataTable>
    {:else}
        <p>This cabinet is currently empty.</p>
    {/if}
</div>

<Separator />

<h3>Folder presence history</h3>
<p>Which folders have been in this cabinet, and when. Newest first.</p>
<div class="panel">
    <DataTable label="Folder presence history" caption="Derived from the move audit log">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">Folder</th>
                <th scope="col">Patient</th>
                <th scope="col">Entered</th>
                <th scope="col">Left</th>
                <th scope="col">Reason left</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.presences as p (p.folderId + p.enteredAt)}
                <DataTableRow>
                    <DataTableTD><a href="/folders/{p.folderId}">{p.folderTitle}</a></DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(p.nhsNumber)}">{p.patientName}</a>
                    </DataTableTD>
                    <DataTableTD>{when(p.enteredAt)}</DataTableTD>
                    <DataTableTD>
                        {#if p.leftAt}{when(p.leftAt)}{:else}<Badge type="success">Still here</Badge>{/if}
                    </DataTableTD>
                    <DataTableTD>{p.leftReason ?? ''}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.presences.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={5}>No folder has been recorded in this cabinet yet.</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
