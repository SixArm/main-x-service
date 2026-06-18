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
    import { t, tf, statusLabel } from '$lib/i18n.svelte';

    let { data } = $props();

    let showLabels = $state(false);
    let printNote = $state('');

    // Reduce volumes to the {id,title} shape the dialog's checklist needs.
    const labelOptions = $derived(data.volumes.map((v) => ({ id: v.id, title: v.title })));

    // Demo stub: report the queued label/copy count instead of printing.
    function onPrint(detail: { selected: string[]; copies: number }) {
        const n = detail.selected.length;
        const copy = detail.copies === 1 ? t('volumes.copy') : t('volumes.copies');
        printNote =
            n === 0
                ? t('volumes.noLabelsSelected')
                : tf(n === 1 ? 'volumes.queued.one' : 'volumes.queued.other', {
                      n,
                      copies: detail.copies,
                      copy
                  });
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

<BackLink href="/">{t('common.backToDashboard')}</BackLink>

<div class="toolbar">
    <h2>{t('volumes.heading')}</h2>
    <div class="actions">
        <button type="button" class="button secondary" onclick={() => (showLabels = true)}>
            <Icon name="printer" /> {t('volumes.printLabels')}
        </button>
        <a href="/volumes/new" class="button">{t('volumes.newVolume')}</a>
    </div>
</div>
<p>{t('volumes.intro')}</p>

{#if printNote}
    <Alert type="success">{printNote}</Alert>
{/if}

<LabelsDialogBox bind:open={showLabels} volumes={labelOptions} onprint={onPrint} />

<div class="panel">
    <DataTable label={t('volumes.tableLabel')} caption={t('volumes.tableCaption')}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.title')}</th>
                <th scope="col">{t('common.patient')}</th>
                <th scope="col">{t('volumes.colFolders')}</th>
                <th scope="col">{t('volumes.colLocation')}</th>
                <th scope="col">{t('common.status')}</th>
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
                        <Badge type={badgeType(volume.status)}>{statusLabel(volume.status)}</Badge>
                    </DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.volumes.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={5}>{t('volumes.noVolumes')}</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
