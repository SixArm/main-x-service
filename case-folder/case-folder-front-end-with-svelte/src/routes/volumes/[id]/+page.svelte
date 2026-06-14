<script lang="ts">
    // Volume detail (`/volumes/[id]`) — manage one movable bundle.
    //
    // A volume groups several of a patient's folders so they move together.
    // This page supports four mutations: rename, add a candidate folder,
    // remove a folder, and move the whole volume. Each goes through the
    // shared `run()` wrapper, which surfaces errors and then calls
    // `invalidateAll()` so the load function re-fetches the canonical state
    // (folders list, candidates, history) rather than patching locally.
    //
    // State: editable `title`, the add/move form fields, and `pageError`.

    import { invalidateAll } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import { api, ApiError } from '$lib/api/client';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
    import Alert from '$lib/components/Alert/Alert.svelte';
    import Badge from '$lib/components/Badge/Badge.svelte';
    import Separator from '$lib/components/Separator/Separator.svelte';
    import Form from '$lib/components/Form/Form.svelte';
    import Field from '$lib/components/Field/Field.svelte';
    import Button from '$lib/components/Button/Button.svelte';
    import DataTable from '$lib/components/DataTable/DataTable.svelte';
    import DataTableHead from '$lib/components/DataTableHead/DataTableHead.svelte';
    import DataTableBody from '$lib/components/DataTableBody/DataTableBody.svelte';
    import DataTableRow from '$lib/components/DataTableRow/DataTableRow.svelte';
    import DataTableTD from '$lib/components/DataTableTD/DataTableTD.svelte';

    let { data } = $props();
    const volume = $derived(data.detail.volume);
    const folders = $derived(data.detail.folders);
    const history = $derived(data.detail.history);

    // Editable rename field, re-seeded from load data on each navigation
    // / invalidate so it reflects the latest persisted title.
    let title = $state('');
    $effect.pre(() => {
        title = data.detail.volume.title;
    });

    let addFolderId = $state('');
    let moveCabinetId = $state('');
    let moveReason = $state('');
    let pageError = $state('');

    // Folder status → Badge colour (green = located, amber = in transit).
    function badgeType(status: string): 'success' | 'warning' | 'default' {
        if (status === 'in-cabinet') return 'success';
        if (status === 'in-transit') return 'warning';
        return 'default';
    }

    // The patient route is keyed by the bare (spaceless) NHS Number.
    function nhsSlug(nhs: string): string {
        return nhs.replaceAll(' ', '');
    }

    // Shared mutation runner: clear the error, perform the action, then
    // re-load page data (so the UI shows the server's authoritative state).
    // Any failure is captured into `pageError` for the alert banner.
    async function run(action: () => Promise<unknown>) {
        pageError = '';
        try {
            await action();
            await invalidateAll();
        } catch (e) {
            pageError = e instanceof ApiError ? e.message : (e as Error).message;
        }
    }

    // The four volume mutations, each wrapped by `run()`. Add/move also
    // reset their form field on success.
    const rename = () => run(() => api.volumes.rename(volume.id, title.trim()));
    const addFolder = () => {
        if (!addFolderId) return;
        return run(async () => {
            await api.volumes.addFolder(volume.id, addFolderId);
            addFolderId = '';
        });
    };
    const removeFolder = (folderId: string) =>
        run(() => api.volumes.removeFolder(volume.id, folderId));
    const moveVolume = () =>
        run(async () => {
            await api.volumes.move(volume.id, {
                toCabinetId: moveCabinetId || null,
                reason: moveReason.trim() || undefined
            });
            moveReason = '';
        });
</script>

<BackLink href="/volumes">Back to volumes</BackLink>

<h2>{volume.title}</h2>
<p>
    <a href="/patients/{nhsSlug(volume.nhsNumber)}">{volume.patientName}</a>
    · <Badge type={badgeType(volume.status)}>{volume.status}</Badge>
    · {volume.cabinetLabel}
</p>

{#if pageError}
    <Alert type="error" heading="Something went wrong">{pageError}</Alert>
{/if}

<div class="panel">
    <h3>Folders in this volume ({folders.length})</h3>
    {#if folders.length > 0}
        <DataTable label="Volume folders">
            <DataTableHead>
                <DataTableRow>
                    <th scope="col">Title</th>
                    <th scope="col">Cabinet</th>
                    <th scope="col">Status</th>
                    <th scope="col">Action</th>
                </DataTableRow>
            </DataTableHead>
            <DataTableBody>
                {#each folders as folder (folder.id)}
                    <DataTableRow>
                        <DataTableTD><a href="/folders/{folder.id}">{folder.title}</a></DataTableTD>
                        <DataTableTD>{folder.cabinetLabel}</DataTableTD>
                        <DataTableTD>
                            <Badge type={badgeType(folder.status)}>{folder.status}</Badge>
                        </DataTableTD>
                        <DataTableTD>
                            <button type="button" class="link-button" onclick={() => removeFolder(folder.id)}>
                                Remove
                            </button>
                        </DataTableTD>
                    </DataTableRow>
                {/each}
            </DataTableBody>
        </DataTable>
    {:else}
        <p>No folders in this volume yet.</p>
    {/if}

    {#if data.candidates.length > 0}
        <Form label="Add a folder" onsubmit={addFolder}>
            <Field label="Add a folder for {volume.patientName}">
                <select bind:value={addFolderId}>
                    <option value="">— Choose a folder —</option>
                    {#each data.candidates as f (f.id)}
                        <option value={f.id}>{f.title}</option>
                    {/each}
                </select>
            </Field>
            <div class="actions">
                <Button type="submit">Add to volume</Button>
            </div>
        </Form>
    {/if}
</div>

<div class="split">
    <div class="panel">
        <h3>Rename volume</h3>
        <Form label="Rename volume" onsubmit={rename}>
            <Field label="Volume title" required>
                <input bind:value={title} required />
            </Field>
            <div class="actions">
                <Button type="submit">Rename</Button>
            </div>
        </Form>
    </div>

    <div class="panel">
        <h3>Move this volume</h3>
        <p>Relocates every folder in the volume together.</p>
        <Form label="Move volume" onsubmit={moveVolume}>
            <Field label="Destination cabinet">
                <select bind:value={moveCabinetId}>
                    <option value="">— In transit —</option>
                    {#each cache.cabinets as c (c.id)}
                        <option value={c.id}>{c.label} ({c.containerPath})</option>
                    {/each}
                </select>
            </Field>
            <Field label="Reason">
                <input bind:value={moveReason} placeholder="e.g. Outpatient clinic" />
            </Field>
            <div class="actions">
                <Button type="submit">Move volume</Button>
            </div>
        </Form>
    </div>
</div>

<Separator />

<h3>Move history</h3>
<div class="move-stack">
    {#each history as move (move.id)}
        <article class="move-card">
            <div class="move-route">
                <a href="/history/{move.id}"><strong>{move.folderTitle}</strong></a>:
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
    {#if history.length === 0}
        <p>No moves recorded yet.</p>
    {/if}
</div>
