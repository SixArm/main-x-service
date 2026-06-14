// Unit tests for the reactive cache store. They pin its non-trivial
// contracts in isolation (the API client is mocked): setters replace lists
// rather than accumulate, `upsertFolder` inserts-or-replaces by id,
// `cabinetLocation` resolves via containerPath → building/room → "?", and
// the `recordMove` / `addFolder` / `add{Building,Room,Cabinet}` mutations
// patch the cache correctly after a (mocked) API round-trip.

import { describe, it, expect, beforeEach, vi } from 'vitest';
import type { Building, Cabinet, Folder, MoveEvent, Room } from './types';

// Mock the API client so the cache's mutation methods (which round-trip
// through `api.*`) can be exercised without any network / Loco backend.
// Each mutation is asserted on its cache side effects, not the HTTP call.
vi.mock('$lib/api/client', () => ({
    api: {
        folders: { create: vi.fn() },
        moves: { create: vi.fn() },
        places: { create: vi.fn() }
    }
}));

import { cache } from './cache.svelte';
import { api } from '$lib/api/client';

// Minimal fixtures. Only the fields the cache reads/writes matter.
function folder(over: Partial<Folder> = {}): Folder {
    return {
        id: 'f1',
        title: 'Volume 1',
        patientId: 'p1',
        nhsNumber: '943 476 5919',
        patientName: 'Alice Johnson',
        cabinetId: 'c1',
        cabinetLabel: 'Cabinet A1',
        status: 'in-cabinet',
        lastMovedAt: '2026-06-01T10:00:00+00:00',
        notes: null,
        volumeId: null,
        volumeTitle: null,
        ...over
    };
}

function cabinet(over: Partial<Cabinet> = {}): Cabinet {
    return {
        id: 'c1',
        label: 'Cabinet A1',
        roomId: 'r1',
        capacity: 100,
        description: null,
        folderCount: 0,
        containerPath: '',
        ...over
    };
}

// The cache is a module-level singleton, so reset every list before each
// test to keep them independent (setters replace-all).
beforeEach(() => {
    cache.setStats({
        patients: 0,
        folders: { total: 0, inCabinet: 0, inTransit: 0 },
        places: { buildings: 0, rooms: 0, cabinets: 0 },
        moves24h: 0
    });
    cache.setFolders([]);
    cache.setPatients([]);
    cache.setBuildings([]);
    cache.setRooms([]);
    cache.setCabinets([]);
    cache.setWorkers([]);
    cache.setMoves([]);
    cache.clearUser();
    vi.clearAllMocks();
});

describe('setters hydrate the reactive getters', () => {
    it('setUser / clearUser', () => {
        expect(cache.user).toBeNull();
        cache.setUser({ email: 'a@b.test', name: 'Mira', role: 'administrator' });
        expect(cache.user?.name).toBe('Mira');
        cache.clearUser();
        expect(cache.user).toBeNull();
    });

    it('setFolders replaces, never accumulates', () => {
        cache.setFolders([folder({ id: 'f1' }), folder({ id: 'f2' })]);
        expect(cache.folders.map((f) => f.id)).toEqual(['f1', 'f2']);
        cache.setFolders([folder({ id: 'f3' })]);
        expect(cache.folders.map((f) => f.id)).toEqual(['f3']);
    });
});

describe('upsertFolder', () => {
    it('appends a folder that is not present', () => {
        cache.upsertFolder(folder({ id: 'f1' }));
        expect(cache.folders).toHaveLength(1);
        expect(cache.folders[0].id).toBe('f1');
    });

    it('replaces a folder that is already present (by id)', () => {
        cache.setFolders([folder({ id: 'f1', title: 'Old' })]);
        cache.upsertFolder(folder({ id: 'f1', title: 'New' }));
        expect(cache.folders).toHaveLength(1);
        expect(cache.folders[0].title).toBe('New');
    });
});

describe('lookups', () => {
    beforeEach(() => {
        cache.setBuildings([{ id: 'b1', name: 'Main Hospital', description: null } as Building]);
        cache.setRooms([
            { id: 'r1', name: 'Records Room', buildingId: 'b1', description: null } as Room
        ]);
        cache.setCabinets([cabinet({ id: 'c1', roomId: 'r1' })]);
    });

    it('buildingById / roomById / cabinetById find by id', () => {
        expect(cache.buildingById('b1')?.name).toBe('Main Hospital');
        expect(cache.roomById('r1')?.name).toBe('Records Room');
        expect(cache.cabinetById('c1')?.label).toBe('Cabinet A1');
    });

    it('lookups return undefined for null / undefined / unknown id', () => {
        expect(cache.buildingById(null)).toBeUndefined();
        expect(cache.roomById(undefined)).toBeUndefined();
        expect(cache.cabinetById('nope')).toBeUndefined();
    });
});

describe('cabinetLocation', () => {
    it('returns "In transit" when the id is null or unknown', () => {
        expect(cache.cabinetLocation(null)).toBe('In transit');
        expect(cache.cabinetLocation('unknown')).toBe('In transit');
    });

    it('prefers the cabinet containerPath when present', () => {
        cache.setCabinets([
            cabinet({ id: 'c1', containerPath: 'Main Hospital — Records Room — Cabinet A1' })
        ]);
        expect(cache.cabinetLocation('c1')).toBe('Main Hospital — Records Room — Cabinet A1');
    });

    it('derives "Building — Room" when containerPath is empty', () => {
        cache.setBuildings([{ id: 'b1', name: 'Main Hospital', description: null } as Building]);
        cache.setRooms([
            { id: 'r1', name: 'Records Room', buildingId: 'b1', description: null } as Room
        ]);
        cache.setCabinets([cabinet({ id: 'c1', roomId: 'r1', containerPath: '' })]);
        expect(cache.cabinetLocation('c1')).toBe('Main Hospital — Records Room');
    });

    it('substitutes "?" for an unresolved room or building', () => {
        // Cabinet present but its room (and therefore building) is not cached.
        cache.setCabinets([cabinet({ id: 'c1', roomId: 'missing', containerPath: '' })]);
        expect(cache.cabinetLocation('c1')).toBe('? — ?');
    });
});

describe('recordMove cache side effects', () => {
    it('prepends the move and updates the folder location in place', async () => {
        cache.setFolders([folder({ id: 'f1', cabinetId: 'c1', status: 'in-cabinet' })]);
        const event: MoveEvent = {
            id: 'm1',
            folderId: 'f1',
            folderTitle: 'Volume 1',
            patientId: 'p1',
            nhsNumber: '943 476 5919',
            patientName: 'Alice Johnson',
            fromCabinetId: 'c1',
            fromCabinetLabel: 'Cabinet A1',
            toCabinetId: 'c2',
            toCabinetLabel: 'Cabinet B2',
            workerId: null,
            movedBy: 'Mira',
            workerRole: null,
            movedAt: '2026-06-02T09:00:00+00:00',
            reason: 'Transfer'
        };
        vi.mocked(api.moves.create).mockResolvedValue(event);

        const returned = await cache.recordMove({ folderId: 'f1', toCabinetId: 'c2' });

        expect(returned).toBe(event);
        expect(cache.moves[0].id).toBe('m1');
        const moved = cache.folders.find((f) => f.id === 'f1');
        expect(moved?.cabinetId).toBe('c2');
        expect(moved?.cabinetLabel).toBe('Cabinet B2');
        expect(moved?.status).toBe('in-cabinet');
        expect(moved?.lastMovedAt).toBe('2026-06-02T09:00:00+00:00');
    });

    it('marks the folder in-transit when the move has no destination cabinet', async () => {
        cache.setFolders([folder({ id: 'f1', cabinetId: 'c1', status: 'in-cabinet' })]);
        const event: MoveEvent = {
            id: 'm2',
            folderId: 'f1',
            folderTitle: 'Volume 1',
            patientId: 'p1',
            nhsNumber: '943 476 5919',
            patientName: 'Alice Johnson',
            fromCabinetId: 'c1',
            fromCabinetLabel: 'Cabinet A1',
            toCabinetId: null,
            toCabinetLabel: '(in transit)',
            workerId: null,
            movedBy: 'Mira',
            workerRole: null,
            movedAt: '2026-06-02T11:00:00+00:00',
            reason: null
        };
        vi.mocked(api.moves.create).mockResolvedValue(event);

        await cache.recordMove({ folderId: 'f1', toCabinetId: null });

        const moved = cache.folders.find((f) => f.id === 'f1');
        expect(moved?.cabinetId).toBeNull();
        expect(moved?.status).toBe('in-transit');
    });

    it('still records the move when the folder is not in the cache', async () => {
        const event: MoveEvent = {
            id: 'm3',
            folderId: 'ghost',
            folderTitle: 'Unknown',
            patientId: 'p9',
            nhsNumber: '987 654 3210',
            patientName: 'Bob Smith',
            fromCabinetId: null,
            fromCabinetLabel: '(new folder)',
            toCabinetId: 'c1',
            toCabinetLabel: 'Cabinet A1',
            workerId: null,
            movedBy: 'Mira',
            workerRole: null,
            movedAt: '2026-06-02T12:00:00+00:00',
            reason: null
        };
        vi.mocked(api.moves.create).mockResolvedValue(event);

        await cache.recordMove({ folderId: 'ghost', toCabinetId: 'c1' });

        expect(cache.moves[0].id).toBe('m3');
        expect(cache.folders).toHaveLength(0);
    });
});

describe('addFolder', () => {
    it('upserts the created folder into the cache', async () => {
        const created = folder({ id: 'fNew', title: 'Brand New' });
        vi.mocked(api.folders.create).mockResolvedValue(created);

        const returned = await cache.addFolder({ nhsNumber: '943 476 5919', title: 'Brand New' });

        expect(returned).toBe(created);
        expect(cache.folders.find((f) => f.id === 'fNew')?.title).toBe('Brand New');
    });
});

describe('addBuilding / addRoom / addCabinet', () => {
    it('addBuilding appends a building and returns its id', async () => {
        vi.mocked(api.places.create).mockResolvedValue({
            id: 'b1',
            name: 'Main Hospital',
            description: 'Acute',
            contained_in_place: null,
            container_path: 'Main Hospital'
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
        } as any);

        const id = await cache.addBuilding({ name: 'Main Hospital', description: 'Acute' });

        expect(id).toBe('b1');
        expect(cache.buildings).toEqual([
            { id: 'b1', name: 'Main Hospital', description: 'Acute' }
        ]);
    });

    it('addRoom appends a room carrying the parent building id', async () => {
        vi.mocked(api.places.create).mockResolvedValue({
            id: 'r1',
            name: 'Records Room',
            description: null,
            contained_in_place: 'b1',
            container_path: 'Main Hospital — Records Room'
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
        } as any);

        const id = await cache.addRoom({ name: 'Records Room', buildingId: 'b1' });

        expect(id).toBe('r1');
        expect(cache.rooms[0].buildingId).toBe('b1');
    });

    it('addCabinet appends a cabinet with a zero folder count', async () => {
        vi.mocked(api.places.create).mockResolvedValue({
            id: 'c1',
            name: 'Cabinet A1',
            description: null,
            contained_in_place: 'r1',
            container_path: 'Main Hospital — Records Room — Cabinet A1',
            capacity: 100
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
        } as any);

        const id = await cache.addCabinet({ label: 'Cabinet A1', roomId: 'r1', capacity: 100 });

        expect(id).toBe('c1');
        expect(cache.cabinets[0].label).toBe('Cabinet A1');
        expect(cache.cabinets[0].roomId).toBe('r1');
        expect(cache.cabinets[0].folderCount).toBe(0);
    });
});
