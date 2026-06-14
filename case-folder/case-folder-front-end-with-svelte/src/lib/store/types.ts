// Domain types for the Case Tracking.
//
// Shapes mirror the Loco JSON API (snake_case fields are converted to
// camelCase by the API client in `$lib/api/client.ts`). The Loco API
// is the source of truth — see `case-folder-service-with-rust/spec.md`
// §11 for the wire contract.
//
// Buildings / Rooms / Cabinets are all `Place`s in the API,
// discriminated by `placeKind`. We keep three separate TypeScript
// types because the UI grids/forms treat them differently (e.g. a
// cabinet has capacity + folder count).

/**
 * Where a folder / volume physically is: parked in a cabinet, or being
 * carried between cabinets. Drives the status badge colour throughout the UI.
 */
export type FolderStatus = 'in-cabinet' | 'in-transit';

/// The signed-in user, as returned by the auth API.
export interface User {
    email: string;
    name: string;
    role: string | null;
}

/** A site (top of the place hierarchy: building → room → cabinet). */
export interface Building {
    id: string;
    name: string;
    description: string | null;
}

/** A room within a building; holds cabinets. */
export interface Room {
    id: string;
    name: string;
    buildingId: string | null;
    description: string | null;
}

/**
 * A physical file cabinet within a room — the leaf location where folders
 * live. `folderCount` / `capacity` drive utilisation reporting;
 * `containerPath` is the API-supplied "Building — Room" display string.
 */
export interface Cabinet {
    id: string;
    label: string;
    roomId: string | null;
    capacity: number | null;
    description: string | null;
    folderCount: number;
    containerPath: string;
}

/**
 * A patient, keyed in the UI by NHS Number. `source` records where the
 * record came from (e.g. central Patient Service vs. local snapshot).
 */
export interface Patient {
    id: string;
    nhsNumber: string;
    name: string;
    dateOfBirth: string | null;
    folderCount: number;
    source: string;
}

/**
 * A paper case-note folder for one patient. Carries denormalised
 * snapshots (NHS Number, patient name, cabinet label) echoed by the API so
 * lists render without joins. May belong to a `Volume`.
 */
export interface Folder {
    id: string;
    title: string;
    patientId: string;
    nhsNumber: string;
    patientName: string;
    cabinetId: string | null;
    cabinetLabel: string;
    status: FolderStatus;
    lastMovedAt: string | null;
    notes: string | null;
    volumeId: string | null;
    volumeTitle: string | null;
}

/// A movable bundle of one patient's folders.
export interface Volume {
    id: string;
    title: string;
    patientId: string;
    nhsNumber: string;
    patientName: string;
    cabinetId: string | null;
    cabinetLabel: string;
    status: FolderStatus;
    folderCount: number;
}

/// A geofence breach: a move that crossed a building boundary.
export interface Alert {
    moveId: string;
    folderId: string;
    folderTitle: string;
    patientName: string;
    nhsNumber: string;
    fromBuilding: string;
    toBuilding: string;
    fromCabinetLabel: string;
    toCabinetLabel: string;
    movedBy: string;
    movedAt: string;
    reason: string | null;
}

/**
 * One audited folder move (the unit of the move history / audit log).
 * Records from/to cabinets, who moved it (worker or free-text), and why.
 */
export interface MoveEvent {
    id: string;
    folderId: string;
    folderTitle: string;
    patientId: string;
    nhsNumber: string;
    patientName: string;
    fromCabinetId: string | null;
    fromCabinetLabel: string;
    toCabinetId: string | null;
    toCabinetLabel: string;
    workerId: string | null;
    movedBy: string;
    workerRole: string | null;
    movedAt: string;
    reason: string | null;
}

/** A member of staff who moves folders (mirrored from the Worker Service). */
export interface Worker {
    id: string;
    name: string;
    role: string | null;
}

/// One stay of a folder in a cabinet (from the move-event log).
/// `leftAt === null` means the folder is still present.
export interface Presence {
    folderId: string;
    folderTitle: string;
    patientId: string;
    nhsNumber: string;
    patientName: string;
    cabinetId: string;
    cabinetLabel: string;
    enteredAt: string;
    leftAt: string | null;
    enteredReason: string | null;
    leftReason: string | null;
}

/** Dashboard summary counts (patients, folder states, place counts, 24h moves). */
export interface Stats {
    patients: number;
    folders: { total: number; inCabinet: number; inTransit: number };
    places: { buildings: number; rooms: number; cabinets: number };
    moves24h: number;
}
