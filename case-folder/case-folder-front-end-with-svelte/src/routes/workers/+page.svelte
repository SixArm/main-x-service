<script lang="ts">
    // Workers index (`/workers`) — staff who move folders.
    //
    // Render-only. Lists workers (mirrored from the central Worker
    // Service) with their role; each links to a per-worker activity page.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();
</script>

<BackLink href="/">Back to dashboard</BackLink>

<h2>Workers</h2>
<p>
    Staff who move folders. Open a worker to see the folders they've moved and
    every folder belonging to their patients.
</p>

<div class="panel">
    <DataTable label="Workers" caption="Workers from the Main Worker Service">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">Name</th>
                <th scope="col">Role</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.workers as worker (worker.id)}
                <DataTableRow>
                    <DataTableTD><a href="/workers/{worker.id}">{worker.name}</a></DataTableTD>
                    <DataTableTD>{worker.role ?? '—'}</DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.workers.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={2}>No workers found.</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
