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

export type FolderStatus = 'in-cabinet' | 'in-transit';

/// The signed-in user, as returned by the auth API.
export interface User {
    email: string;
    name: string;
    role: string | null;
}

export interface Building {
    id: string;
    name: string;
    description: string | null;
}

export interface Room {
    id: string;
    name: string;
    buildingId: string | null;
    description: string | null;
}

export interface Cabinet {
    id: string;
    label: string;
    roomId: string | null;
    capacity: number | null;
    description: string | null;
    folderCount: number;
    containerPath: string;
}

export interface Patient {
    id: string;
    nhsNumber: string;
    name: string;
    dateOfBirth: string | null;
    folderCount: number;
    source: string;
}

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

export interface Stats {
    patients: number;
    folders: { total: number; inCabinet: number; inTransit: number };
    places: { buildings: number; rooms: number; cabinets: number };
    moves24h: number;
}
