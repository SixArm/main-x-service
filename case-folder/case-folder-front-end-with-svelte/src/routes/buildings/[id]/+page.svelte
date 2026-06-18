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
    import { t, tf } from '$lib/i18n.svelte';

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
            roomError = t('buildingDetail.roomNameRequired');
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

<BackLink href="/buildings">{t('buildingDetail.backToBuildings')}</BackLink>

<h2>{building.name}</h2>
{#if building.description}<p>{building.description}</p>{/if}

<div class="split">
    <div class="panel">
        <h3>{tf('buildingDetail.rooms', { n: data.rooms.length })}</h3>
        {#if data.rooms.length > 0}
            <DataTable label={t('buildingDetail.roomsTable')}>
                <DataTableHead>
                    <DataTableRow>
                        <th scope="col">{t('common.name')}</th>
                        <th scope="col">{t('buildingDetail.colCabinets')}</th>
                        <th scope="col">{t('common.description')}</th>
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
            <p>{t('buildingDetail.noRooms')}</p>
        {/if}
    </div>

    <div class="panel">
        <h3>{t('buildingDetail.addRoom')}</h3>
        <Form label={t('buildingDetail.addRoomLabel')} onsubmit={addRoom}>
            <Field label={t('buildingDetail.roomName')} required error={roomError}>
                <input bind:value={newRoomName} required />
            </Field>
            <Field label={t('common.description')}>
                <textarea bind:value={newRoomDescription} rows="2"></textarea>
            </Field>
            <div class="actions">
                <Button type="submit">{t('buildingDetail.saveRoom')}</Button>
            </div>
        </Form>
    </div>
</div>

<Separator />

<h3>{t('buildingDetail.presenceHistory')}</h3>
<p>{t('buildingDetail.presenceIntro')}</p>
<div class="panel">
    <DataTable label={t('buildingDetail.presenceTable')} caption={t('buildingDetail.presenceCaption')}>
        <DataTableHead>
            <DataTableRow>
                <th scope="col">{t('common.folder')}</th>
                <th scope="col">{t('common.patient')}</th>
                <th scope="col">{t('common.cabinet')}</th>
                <th scope="col">{t('common.entered')}</th>
                <th scope="col">{t('common.left')}</th>
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
                            <Badge type="success">{t('common.stillHere')}</Badge>
                        {/if}
                    </DataTableTD>
                </DataTableRow>
            {/each}
            {#if data.presences.length === 0}
                <DataTableRow>
                    <DataTableTD colspan={5}>{t('buildingDetail.noPresence')}</DataTableTD>
                </DataTableRow>
            {/if}
        </DataTableBody>
    </DataTable>
</div>
