<script lang="ts">
    // Patient detail (`/patients/[nhs]`) — one patient's record.
    //
    // Keyed by NHS Number. Shows the addressograph box, the patient's
    // folders, and their move history. When the central Patient Service
    // has no record (`patientServiceMatch` false), a warning explains the
    // folders are reconstructed from local snapshots. The ButtonBar mimics
    // a clinical record's action set; only a few actions are live in this
    // demo (see `onAction`).

    import { goto } from '$app/navigation';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import AddressographBox from '$lib/components/AddressographBox/AddressographBox.svelte';
    import ButtonBar from '$lib/components/ButtonBar/ButtonBar.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();

    let actionNote = $state('');

    function badgeType(status: string): 'success' | 'warning' | 'info' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    // Patient-record action bar. Wire the actions that have a real
    // destination in this app; note the rest as demo-only.
    function onAction(key: string) {
        actionNote = '';
        const nhs = data.patient?.nhsNumber ?? data.nhsNumber;
        if (key === 'audit') {
            goto(`/history?q=${encodeURIComponent(nhs)}`);
        } else if (key === 'quick-reports') {
            goto('/reports');
        } else if (key === 'patient' || key === 'case-notes') {
            // Already on the patient record; case notes are listed below.
        } else {
            actionNote = `“${key.replaceAll('-', ' ')}” isn't available in this demo.`;
        }
    }
</script>

<BackLink href="/patients">Back to patients</BackLink>

{#if !data.patientServiceMatch}
    <Alert type="warning" heading="Patient not found in Main Patient Service">
        No patient record exists for NHS Number <span class="nhs-number">{data.nhsNumber}</span>.
        The folders below are reconstructed from local snapshots written when
        each folder was created.
    </Alert>
{/if}

{#if data.patient}
    <h2>{data.patient.name}</h2>
    <AddressographBox
        name={data.patient.name}
        nhsNumber={data.patient.nhsNumber}
        dateOfBirth={data.patient.dateOfBirth}
    />
    <p class="muted">Source: {data.patient.source}</p>
    <ButtonBar active="patient" onselect={onAction} label="Patient record actions" />
    {#if actionNote}
        <Alert type="info">{actionNote}</Alert>
    {/if}
{:else}
    <h2>NHS Number {data.nhsNumber}</h2>
{/if}

<div class="panel">
    <h3>Folders for this patient ({data.folders.length})</h3>
    {#if data.folders.length > 0}
        <DataTable label="Patient folders">
            <DataTableHead>
                <DataTableRow>
                    <th scope="col">Title</th>
                    <th scope="col">Volume</th>
                    <th scope="col">Cabinet</th>
                    <th scope="col">Status</th>
                    <th scope="col">Last moved</th>
                    <th scope="col">Action</th>
                </DataTableRow>
            </DataTableHead>
            <DataTableBody>
                {#each data.folders as folder (folder.id)}
                    <DataTableRow>
                        <DataTableTD>
                            <a href="/folders/{folder.id}">{folder.title}</a>
                        </DataTableTD>
                        <DataTableTD>
                            {#if folder.volumeId}
                                <a href="/volumes/{folder.volumeId}">{folder.volumeTitle ?? 'Volume'}</a>
                            {:else}
                                —
                            {/if}
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
            </DataTableBody>
        </DataTable>
    {:else}
        <p>No folders yet.</p>
    {/if}
    <p style="margin-top: var(--nhs-space-3);">
        <a href="/folders/new" class="button">Add a folder for this patient</a>
    </p>
</div>

<Separator />

<h3>Move history for this patient</h3>
<div class="move-stack">
    {#each data.history as move (move.id)}
        <article class="move-card">
            <div class="move-route">
                <strong>{move.folderTitle}</strong>:
                <span>{move.fromCabinetLabel}</span>
                <span class="move-arrow" aria-hidden="true">→</span>
                <span>{move.toCabinetLabel}</span>
            </div>
            <p class="move-meta">
                {move.movedBy}{#if move.workerRole} ({move.workerRole}){/if}
                · {new Date(move.movedAt).toLocaleString('en-GB')}
                {#if move.reason}· {move.reason}{/if}
            </p>
        </article>
    {/each}
    {#if data.history.length === 0}
        <p>No moves recorded yet.</p>
    {/if}
</div>
