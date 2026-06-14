<script lang="ts">
    // Volumes index (`/volumes`) — list of folder bundles + label printing.
    //
    // Lists every volume and opens a labels dialog to queue spine/box
    // labels for printing. Label printing is a demo stub: `onPrint` just
    // reports what would have been queued.
    //
    // State:
    //   showLabels — controls the LabelsDialogBox open state.
    //   printNote  — confirmation copy after a (simulated) print.

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Icon from '$lib/components/Icon/Icon.svelte';
    import LabelsDialogBox from '$lib/components/LabelsDialogBox/LabelsDialogBox.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();

    let showLabels = $state(false);
    let printNote = $state('');

    // Reduce volumes to the {id,title} shape the dialog's checklist needs.
    const labelOptions = $derived(data.volumes.map((v) => ({ id: v.id, title: v.title })));

    // Demo stub: report the queued label/copy count instead of printing.
    function onPrint(detail: { selected: string[]; copies: number }) {
        const n = detail.selected.length;
        const copy = detail.copies === 1 ? 'copy' : 'copies';
        printNote =
            n === 0
                ? 'No labels selected.'
                : `Queued ${n} label${n === 1 ? '' : 's'} × ${detail.copies} ${copy} for printing.`;
        showLabels = false;
    }

    // Status → Badge colour (green = located, amber = in transit).
    function badgeType(status: string): 'success' | 'warning' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }
</script>

<BackLink href="/">Back to dashboard</BackLink>

<div class="toolbar">
    <h2>Volumes</h2>
    <div class="actions">
        <button type="button" class="button secondary" onclick={() => (showLabels = true)}>
            <Icon name="printer" /> Print labels
        </button>
        <a href="/volumes/new" class="button">New volume</a>
    </div>
</div>
<p>A volume is a movable bundle of one patient's folders. Move the volume and every folder inside it moves together.</p>

{#if printNote}
    <Alert type="success">{printNote}</Alert>
{/if}

<LabelsDialogBox bind:open={showLabels} volumes={labelOptions} onprint={onPrint} />

<div class="panel">
    <DataTable label="Volumes" caption="Bundles of folders, each belonging to one patient">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">Title</th>
                <th scope="col">Patient</th>
                <th scope="col">Folders</th>
                <th scope="col">Location</th>
                <th scope="col">Status</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.volumes as volume (volume.id)}
                <DataTableRow>
                    <DataTableTD><a href="/volumes/{volume.id}">{volume.title}</a></DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{nhsSlug(volume.nhsNumber)}">{volume.patientName}</a>
                    </DataTableTD>
                    <DataTableTD>{volume.folderCount}</DataTableTD>
                    <DataTableTD>{volume.cabinetLabel}</DataTableTD>
                    <DataTableTD>
                        <Badge type={badgeType(volume.status)}>{volume.status}</Badge>
                    </DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.volumes.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={5}>No volumes yet. Create one to bundle a patient's folders.</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
