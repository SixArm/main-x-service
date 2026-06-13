<script lang="ts">
    import { goto } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();

    let query = $state('');
    $effect.pre(() => {
        query = data.query;
    });
    let debounce: ReturnType<typeof setTimeout> | null = null;

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

    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/">Back to dashboard</BackLink>

<div class="toolbar">
    <h2>Folder register</h2>
    <div style="display:flex; gap: var(--nhs-space-2);">
        <input
            type="search"
            bind:value={query}
            oninput={onSearchInput}
            placeholder="Search by NHS Number, patient, folder title, or cabinet"
            aria-label="Search folders"
        />
        <a href="/folders/new" class="button">Add folder</a>
    </div>
</div>

<div class="panel">
    <DataTable label="Folders" caption="All paper case-note folders tracked by the system">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">NHS Number</th>
                <th scope="col">Patient</th>
                <th scope="col">Folder</th>
                <th scope="col">Cabinet</th>
                <th scope="col">Status</th>
                <th scope="col">Last moved</th>
                <th scope="col">Action</th>
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
                        <Badge type={badgeType(folder.status)}>{folder.status}</Badge>
                    </DataTableTD>
                    <DataTableTD>
                        {folder.lastMovedAt ? new Date(folder.lastMovedAt).toLocaleString('en-GB') : '—'}
                    </DataTableTD>
                    <DataTableTD>
                        <a href="/move?folder={folder.id}">Move</a>
                    </DataTableTD>
                </DataTableRow>
            {/each}
            {#if cache.folders.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={7}>
                        No folders match <strong>{data.query}</strong>.
                    </DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
