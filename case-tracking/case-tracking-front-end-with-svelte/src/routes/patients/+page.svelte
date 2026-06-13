<script lang="ts">
    import { cache } from '$lib/store/cache.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let query = $state('');

    const filtered = $derived.by(() => {
        const q = query.trim().toLowerCase();
        if (!q) return cache.patients;
        return cache.patients.filter(
            (p) =>
                p.name.toLowerCase().includes(q) ||
                p.nhsNumber.replaceAll(' ', '').includes(q.replaceAll(' ', ''))
        );
    });

    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/">Back to dashboard</BackLink>

<div class="toolbar">
    <h2>Patients</h2>
    <input
        type="search"
        bind:value={query}
        placeholder="Search by NHS Number or name"
        aria-label="Search patients"
    />
</div>

<div class="panel">
    <DataTable label="Patients" caption="All patients with one or more registered folders">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">NHS Number</th>
                <th scope="col">Name</th>
                <th scope="col">Date of birth</th>
                <th scope="col">Folders</th>
                <th scope="col">Source</th>
                <th scope="col">Action</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each filtered as patient (patient.id)}
                <DataTableRow>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(patient.nhsNumber)}" class="nhs-number">
                            {patient.nhsNumber}
                        </a>
                    </DataTableTD>
                    <DataTableTD>{patient.name}</DataTableTD>
                    <DataTableTD>{patient.dateOfBirth ?? '—'}</DataTableTD>
                    <DataTableTD>{patient.folderCount}</DataTableTD>
                    <DataTableTD>{patient.source}</DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(patient.nhsNumber)}">View</a>
                    </DataTableTD>
                </DataTableRow>
            {/each}
            {#if filtered.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={6}>No patients match <strong>{query}</strong>.</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
