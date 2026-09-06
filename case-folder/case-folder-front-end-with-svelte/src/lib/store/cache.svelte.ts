// Rune-reactive cache, hydrated by route `+page.ts` load functions.
//
// The cache holds the *most recently fetched* projection of each
// resource so the UI can render reactively. Load functions are the
// only callers that write into the cache; pages read via the exported
// getters.
//
// Mutation methods (`addFolder`, `recordMove`, `addPlace`) round-trip
// through the API client and, on success, splice the new record into
// the cache so subsequent renders see the change without a refetch.

import { api } from '$lib/api/client';
import type {
    Building,
    Cabinet,
    Folder,
    MoveEvent,
    Patient,
    Room,
    Stats,
    User,
    Worker,
} from './types';

/**
 * Build the singleton reactive cache.
 *
 * Returns an object of Svelte rune-backed getters (read by pages),
 * setters (called by `+page.ts` load functions to hydrate state),
 * mutation methods (which call the API then patch the cache), and
 * synchronous lookup helpers. Exactly one instance is created at module
 * load and exported as `cache`; there is no per-component state here.
 *
 * @returns The cache facade (getters + setters + mutations + lookups).
 */
function createCache() {
    let user = $state<User | null>(null);
    let stats = $state<Stats | null>(null);
    const folders = $state<Folder[]>([]);
    const patients = $state<Patient[]>([]);
    const buildings = $state<Building[]>([]);
    const rooms = $state<Room[]>([]);
    const cabinets = $state<Cabinet[]>([]);
    const workers = $state<Worker[]>([]);
    const moves = $state<MoveEvent[]>([]);

    /**
     * Replace a reactive array's contents in place.
     *
     * Mutates the existing array (via `splice`) instead of reassigning it,
     * so the `$state` proxy keeps tracking the same reference and all
     * subscribers re-render.
     *
     * @param target - The reactive array to overwrite.
     * @param next - The new contents.
     */
    function replaceAll<T>(target: T[], next: T[]): void {
        target.splice(0, target.length, ...next);
    }

    // -----------------------------------------------------------------------
    // Setters used by `+page.ts` load functions to hydrate the cache after a
    // successful API call.
    // -----------------------------------------------------------------------

    /** Set the signed-in user (or null). @param u - The user, or null. */
    function setUser(u: User | null): void {
        user = u;
    }

    /** Clear the signed-in user (e.g. on sign-out / 401). */
    function clearUser(): void {
        user = null;
    }

    /** Replace the cached dashboard stats. @param s - The latest stats. */
    function setStats(s: Stats): void {
        stats = s;
    }

    /** Replace the cached folder list in place. @param next - New folders. */
    function setFolders(next: Folder[]): void {
        replaceAll(folders, next);
    }

    /** Replace the cached patient list in place. @param next - New patients. */
    function setPatients(next: Patient[]): void {
        replaceAll(patients, next);
    }

    /** Replace the cached building list in place. @param next - New buildings. */
    function setBuildings(next: Building[]): void {
        replaceAll(buildings, next);
    }

    /** Replace the cached room list in place. @param next - New rooms. */
    function setRooms(next: Room[]): void {
        replaceAll(rooms, next);
    }

    /** Replace the cached cabinet list in place. @param next - New cabinets. */
    function setCabinets(next: Cabinet[]): void {
        replaceAll(cabinets, next);
    }

    /** Replace the cached worker list in place. @param next - New workers. */
    function setWorkers(next: Worker[]): void {
        replaceAll(workers, next);
    }

    /** Replace the cached move log in place. @param next - New move events. */
    function setMoves(next: MoveEvent[]): void {
        replaceAll(moves, next);
    }

    /**
     * Insert or replace one folder in the cache by id.
     *
     * Used by `addFolder` so a freshly-created folder is visible without a
     * refetch. Replaces an existing entry in place when the id matches,
     * otherwise appends.
     *
     * @param folder - The folder to upsert.
     */
    function upsertFolder(folder: Folder): void {
        const idx = folders.findIndex((f) => f.id === folder.id);
        if (idx >= 0) folders[idx] = folder;
        else folders.push(folder);
    }

    // -----------------------------------------------------------------------
    // Mutations — talk to the API, then update the cache on success. Errors
    // propagate to the caller (the page's submit handler shows them inline).
    // -----------------------------------------------------------------------

    /**
     * Create a folder via the API, then cache it.
     *
     * Side effect: upserts the returned folder into `folders` so list
     * views update without a refetch.
     *
     * @param input - New-folder fields (NHS Number + title required;
     *   patient name / DOB are only needed for a not-yet-registered patient).
     * @returns The created folder.
     * @throws {ApiError} If the API rejects the create (e.g. 422 validation).
     */
    async function addFolder(input: {
        nhsNumber: string;
        patientName?: string;
        dateOfBirth?: string;
        title: string;
        cabinetId?: string | null;
        notes?: string;
    }): Promise<Folder> {
        const folder = await api.folders.create(input);
        upsertFolder(folder);
        return folder;
    }

    /**
     * Record a folder move via the API, then reflect it in the cache.
     *
     * Two side effects on success:
     *   1. the new move event is prepended to `moves` (newest first);
     *   2. if the moved folder is cached, its `cabinetId` / `cabinetLabel`
     *      / `status` (in-cabinet vs in-transit) / `lastMovedAt` are
     *      patched in place so any folder list rendered next to the move
     *      form updates immediately without a refetch.
     *
     * @param input - The move: `folderId` plus a destination cabinet
     *   (`toCabinetId` null means "in transit"), and optional worker / mover
     *   / reason.
     * @returns The created move event.
     * @throws {ApiError} If the API rejects the move (e.g. 404 / 422).
     */
    async function recordMove(input: {
        folderId: string;
        toCabinetId: string | null;
        workerId?: string | null;
        movedBy?: string;
        reason?: string;
    }): Promise<MoveEvent> {
        const event = await api.moves.create(input);
        moves.unshift(event);
        // Reflect the new location on the cached folder, if present, so
        // any list rendered alongside the move form updates immediately.
        const idx = folders.findIndex((f) => f.id === event.folderId);
        if (idx >= 0) {
            folders[idx] = {
                ...folders[idx],
                cabinetId: event.toCabinetId,
                cabinetLabel: event.toCabinetLabel,
                status: event.toCabinetId ? 'in-cabinet' : 'in-transit',
                lastMovedAt: event.movedAt,
            };
        }
        return event;
    }

    /**
     * Create a building (a top-level `Place`) and cache it.
     *
     * @param input - Building name and optional description.
     * @returns The new building's id (so callers can navigate to it).
     * @throws {ApiError} On a rejected create.
     */
    async function addBuilding(input: {
        name: string;
        description?: string;
    }): Promise<string> {
        const place = await api.places.create({
            name: input.name,
            kind: 'building',
            description: input.description,
        });
        buildings.push({
            id: place.id,
            name: place.name,
            description: place.description,
        });
        return place.id;
    }

    /**
     * Create a room inside a building (a `Place` contained in the building)
     * and cache it.
     *
     * @param input - Room name, parent `buildingId`, optional description.
     * @returns The new room's id.
     * @throws {ApiError} On a rejected create.
     */
    async function addRoom(input: {
        name: string;
        buildingId: string;
        description?: string;
    }): Promise<string> {
        const place = await api.places.create({
            name: input.name,
            kind: 'room',
            containedInPlace: input.buildingId,
            description: input.description,
        });
        rooms.push({
            id: place.id,
            name: place.name,
            buildingId: place.contained_in_place,
            description: place.description,
        });
        return place.id;
    }

    /**
     * Create a cabinet inside a room (a `Place` contained in the room) and
     * cache it. The new cabinet starts with `folderCount` 0.
     *
     * @param input - Cabinet label, parent `roomId`, optional capacity
     *   (null/omitted = uncapped) and description.
     * @returns The new cabinet's id.
     * @throws {ApiError} On a rejected create.
     */
    async function addCabinet(input: {
        label: string;
        roomId: string;
        capacity?: number | null;
        description?: string;
    }): Promise<string> {
        const place = await api.places.create({
            name: input.label,
            kind: 'cabinet',
            containedInPlace: input.roomId,
            capacity: input.capacity,
            description: input.description,
        });
        cabinets.push({
            id: place.id,
            label: place.name,
            roomId: place.contained_in_place,
            capacity: place.capacity,
            description: place.description,
            folderCount: 0,
            containerPath: place.container_path,
        });
        return place.id;
    }

    // -----------------------------------------------------------------------
    // Lookup helpers (synchronous; read from cache only).
    // -----------------------------------------------------------------------

    /**
     * Look up a cached building by id.
     * @param id - Building id (null/undefined yields undefined).
     * @returns The building, or undefined if not cached.
     */
    function buildingById(id: string | null | undefined): Building | undefined {
        return id ? buildings.find((b) => b.id === id) : undefined;
    }

    /**
     * Look up a cached room by id.
     * @param id - Room id (null/undefined yields undefined).
     * @returns The room, or undefined if not cached.
     */
    function roomById(id: string | null | undefined): Room | undefined {
        return id ? rooms.find((r) => r.id === id) : undefined;
    }

    /**
     * Look up a cached cabinet by id.
     * @param id - Cabinet id (null/undefined yields undefined).
     * @returns The cabinet, or undefined if not cached.
     */
    function cabinetById(id: string | null | undefined): Cabinet | undefined {
        return id ? cabinets.find((c) => c.id === id) : undefined;
    }

    /**
     * Resolve a cabinet id to a human-readable location string.
     *
     * Resolution order:
     *   - no id / unknown cabinet → "In transit" (the folder is between
     *     cabinets);
     *   - if the cabinet carries a precomputed `containerPath` from the
     *     API, use it verbatim;
     *   - otherwise walk cabinet → room → building from the cache and
     *     assemble "Building — Room", substituting "?" for any missing link.
     *
     * @param id - The cabinet id to describe.
     * @returns A display string for the folder's current location.
     */
    function cabinetLocation(id: string | null | undefined): string {
        const cab = cabinetById(id);
        if (!cab) return 'In transit';
        if (cab.containerPath) return cab.containerPath;
        const room = roomById(cab.roomId);
        const building = buildingById(room?.buildingId);
        return `${building?.name ?? '?'} — ${room?.name ?? '?'}`;
    }

    return {
        // Reactive getters
        get user() {
            return user;
        },
        get stats() {
            return stats;
        },
        get folders() {
            return folders;
        },
        get patients() {
            return patients;
        },
        get buildings() {
            return buildings;
        },
        get rooms() {
            return rooms;
        },
        get cabinets() {
            return cabinets;
        },
        get workers() {
            return workers;
        },
        get moves() {
            return moves;
        },
        // Setters
        setUser,
        clearUser,
        setStats,
        setFolders,
        setPatients,
        setBuildings,
        setRooms,
        setCabinets,
        setWorkers,
        setMoves,
        upsertFolder,
        // Mutations
        addFolder,
        recordMove,
        addBuilding,
        addRoom,
        addCabinet,
        // Lookups
        buildingById,
        roomById,
        cabinetById,
        cabinetLocation,
    };
}

/** The application-wide reactive cache singleton (one per page load). */
export const cache = createCache();
/** The shape of {@link cache}, for typing references that receive it. */
export type Cache = ReturnType<typeof createCache>;
