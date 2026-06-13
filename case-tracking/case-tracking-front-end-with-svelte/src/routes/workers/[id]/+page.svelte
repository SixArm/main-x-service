<script lang="ts">
    import type { Folder } from '$lib/store/types';
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

    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

{#snippet folderTable(folders: Folder[], label: string)}
    <DataTable {label}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">Title</th>
                <th scope="col">Patient</th>
                <th scope="col">Cabinet</th>
                <th scope="col">Status</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each folders as folder (folder.id)}
                <DataTableRow>
                    <DataTableTD><a href="/folders/{folder.id}">{folder.title}</a></DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(folder.nhsNumber)}">{folder.patientName}</a>
                    </DataTableTD>
                    <DataTableTD>{folder.cabinetLabel}</DataTableTD>
                    <DataTableTD>
                        <Badge type={badgeType(folder.status)}>{folder.status}</Badge>
                    </DataTableTD>
                </DataTableRow>
            {/each}
        </DataTableBody>
    </DataTable>
{/snippet}

<BackLink href="/workers">Back to workers</BackLink>

<h2>{data.worker.name}</h2>
{#if data.worker.role}<p><em>{data.worker.role}</em></p>{/if}

<div class="panel">
    <h3>Folders moved by this worker ({data.movedFolders.length})</h3>
    {#if data.movedFolders.length > 0}
        {@render folderTable(data.movedFolders, 'Folders moved by this worker')}
    {:else}
        <p>This worker hasn't moved any folders yet.</p>
    {/if}
</div>

<div class="panel">
    <h3>All their patients' folders ({data.patientFolders.length})</h3>
    <p>Every folder belonging to a patient this worker has handled.</p>
    {#if data.patientFolders.length > 0}
        {@render folderTable(data.patientFolders, "This worker's patients' folders")}
    {:else}
        <p>No patient folders to show yet.</p>
    {/if}
</div>

<Separator />

<h3>Moves by this worker</h3>
<div class="move-stack">
    {#each data.moves as move (move.id)}
        <article class="move-card">
            <div class="move-route">
                <a href="/history/{move.id}"><strong>{move.folderTitle}</strong></a>:
                <span>{move.fromCabinetLabel}</span>
                <span class="move-arrow" aria-hidden="true">→</span>
                <span>{move.toCabinetLabel}</span>
            </div>
            <p class="move-meta">
                {move.patientName} · {new Date(move.movedAt).toLocaleString('en-GB')}
                {#if move.reason}· {move.reason}{/if}
            </p>
        </article>
    {/each}
    {#if data.moves.length === 0}
        <p>No moves recorded yet.</p>
    {/if}
</div>
