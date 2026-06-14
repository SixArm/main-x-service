<script lang="ts">
    // Building detail (`/buildings/[id]`) — rooms + add-room + history.
    //
    // Lists the building's rooms (with a live cabinet count read from the
    // cache), an inline "add a room" form that creates under this building
    // and re-loads, and the aggregated folder presence history across all
    // the building's cabinets.
    //
    // State: the add-room form fields (name/description) + roomError.

    import { invalidateAll } from '$app/navigation';
    import { cache } from '$lib/store/cache.svelte';
    import { ApiError } from '$lib/api/client';

    import BackLink from '$lib/components/BackLink/BackLink.svelte';
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
    const building = $derived(data.building);

    let newRoomName = $state('');
    let newRoomDescription = $state('');
    let roomError = $state('');

    // Create a room under this building, then re-load so the rooms table
    // and cabinet counts refresh from the server.
    async function addRoom() {
        roomError = '';
        if (!newRoomName.trim()) {
            roomError = 'Room name is required.';
            return;
        }
        try {
            await cache.addRoom({
                name: newRoomName.trim(),
                buildingId: building.id,
                description: newRoomDescription.trim() || undefined
            });
            newRoomName = '';
            newRoomDescription = '';
            await invalidateAll();
        } catch (e) {
            if (e instanceof ApiError && e.status === 422) {
                const body = e.body as { errors?: Record<string, string> } | null;
                roomError = body?.errors?.name ?? e.message;
            } else {
                roomError = (e as Error).message;
            }
        }
    }
</script>

<BackLink href="/buildings">Back to buildings</BackLink>

<h2>{building.name}</h2>
{#if building.description}<p>{building.description}</p>{/if}

<div class="split">
    <div class="panel">
        <h3>Rooms ({data.rooms.length})</h3>
        {#if data.rooms.length > 0}
            <DataTable label="Rooms">
                <DataTableHead>
                    <DataTableRow>
                        <th scope="col">Name</th>
                        <th scope="col">Cabinets</th>
                        <th scope="col">Description</th>
                    </DataTableRow>
                </DataTableHead>
                <DataTableBody>
                    {#each data.rooms as room (room.id)}
                        <DataTableRow>
                            <DataTableTD><a href="/rooms/{room.id}">{room.name}</a></DataTableTD>
                            <DataTableTD>{cache.cabinets.filter((c) => c.roomId === room.id).length}</DataTableTD>
                            <DataTableTD>{room.description ?? ''}</DataTableTD>
                        </DataTableRow>
                    {/each}
                </DataTableBody>
            </DataTable>
        {:else}
            <p>No rooms yet.</p>
        {/if}
    </div>

    <div class="panel">
        <h3>Add a room</h3>
        <Form label="Add room" onsubmit={addRoom}>
            <Field label="Room name" required error={roomError}>
                <input bind:value={newRoomName} required />
            </Field>
            <Field label="Description">
                <textarea bind:value={newRoomDescription} rows="2"></textarea>
            </Field>
            <div class="actions">
                <Button type="submit">Save room</Button>
            </div>
        </Form>
    </div>
</div>

<Separator />

<h3>Folder presence history</h3>
<p>Folders that have been in any cabinet in this building, newest first.</p>
<div class="panel">
    <DataTable label="Building folder presence history" caption="Aggregated across this building's cabinets">
        <DataTableHead>
            <DataTableRow>
                <th scope="col">Folder</th>
                <th scope="col">Patient</th>
                <th scope="col">Cabinet</th>
                <th scope="col">Entered</th>
                <th scope="col">Left</th>
            </DataTableRow>
        </DataTableHead>
        <DataTableBody>
            {#each data.presences as p (p.cabinetId + p.folderId + p.enteredAt)}
                <DataTableRow>
                    <DataTableTD><a href="/folders/{p.folderId}">{p.folderTitle}</a></DataTableTD>
                    <DataTableTD>
                        <a href="/patients/{p.nhsNumber.replaceAll(' ', '')}">{p.patientName}</a>
                    </DataTableTD>
                    <DataTableTD>{p.cabinetLabel}</DataTableTD>
                    <DataTableTD>{new Date(p.enteredAt).toLocaleString('en-GB')}</DataTableTD>
                    <DataTableTD>
                        {#if p.leftAt}
                            {new Date(p.leftAt).toLocaleString('en-GB')}
                        {:else}
                            <Badge type="success">Still here</Badge>
                        {/if}
                    </DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.presences.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={5}>No folder presence recorded in this building yet.</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
