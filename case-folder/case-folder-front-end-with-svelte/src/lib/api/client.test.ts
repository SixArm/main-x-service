import { describe, it, expect } from 'vitest';
import {
    ApiError,
    toBuilding,
    toCabinet,
    toFolder,
    toMove,
    toPatient,
    toRoom,
    toStats,
    toWorker
} from './client';

describe('snake → camel mappers', () => {
    it('maps a folder, including volume fields', () => {
        const folder = toFolder({
            id: 'f1',
            title: 'Volume 1 — General',
            patient_id: 'p1',
            nhs_number: '943 476 5919',
            patient_name: 'Alice Johnson',
            cabinet_id: 'c1',
            cabinet_label: 'Main Hospital — Ward A — Cabinet A1',
            status: 'in-cabinet',
            last_moved_at: '2026-06-01T10:15:32+00:00',
            notes: null,
            volume_id: 'v1',
            volume_title: 'Alice — Vol 1'
        });
        expect(folder.patientId).toBe('p1');
        expect(folder.cabinetLabel).toContain('Cabinet A1');
        expect(folder.volumeId).toBe('v1');
        expect(folder.volumeTitle).toBe('Alice — Vol 1');
    });

    it('maps a move event', () => {
        const move = toMove({
            id: 'm1',
            folder_id: 'f1',
            folder_title: 'Volume 1',
            patient_id: 'p1',
            nhs_number: '943 476 5919',
            patient_name: 'Alice Johnson',
            from_cabinet_id: null,
            from_cabinet_label: '(new folder)',
            to_cabinet_id: 'c1',
            to_cabinet_label: 'Cabinet A1',
            worker_id: null,
            moved_by: 'Mira',
            worker_role: 'administrator',
            moved_at: '2026-06-01T10:15:32+00:00',
            reason: 'Created'
        });
        expect(move.folderId).toBe('f1');
        expect(move.toCabinetLabel).toBe('Cabinet A1');
        expect(move.workerRole).toBe('administrator');
    });

    it('maps a patient, including count and source provenance', () => {
        const patient = toPatient({
            id: 'p1',
            nhs_number: '943 476 5919',
            name: 'Alice Johnson',
            date_of_birth: '1980-04-12',
            folder_count: 3,
            source: 'patient-service'
        });
        expect(patient.id).toBe('p1');
        expect(patient.nhsNumber).toBe('943 476 5919');
        expect(patient.dateOfBirth).toBe('1980-04-12');
        expect(patient.folderCount).toBe(3);
        expect(patient.source).toBe('patient-service');
    });

    it('maps a patient with a null date of birth', () => {
        const patient = toPatient({
            id: 'p2',
            nhs_number: '987 654 3210',
            name: 'Bob Smith',
            date_of_birth: null,
            folder_count: 0,
            source: 'snapshot'
        });
        expect(patient.dateOfBirth).toBeNull();
        expect(patient.folderCount).toBe(0);
    });

    it('maps a building, dropping place hierarchy fields', () => {
        const building = toBuilding({
            id: 'b1',
            name: 'Main Hospital',
            place_type: 'Hospital',
            place_kind: 'building',
            description: 'Acute site',
            contained_in_place: null,
            container_path: 'Main Hospital',
            capacity: null,
            source: 'place-service'
        });
        expect(building).toEqual({
            id: 'b1',
            name: 'Main Hospital',
            description: 'Acute site'
        });
    });

    it('maps a room, carrying the parent building id', () => {
        const room = toRoom({
            id: 'r1',
            name: 'Records Room',
            place_type: 'RecordsRoom',
            place_kind: 'room',
            description: null,
            contained_in_place: 'b1',
            container_path: 'Main Hospital — Records Room',
            capacity: null,
            source: 'place-service'
        });
        expect(room.id).toBe('r1');
        expect(room.name).toBe('Records Room');
        expect(room.buildingId).toBe('b1');
        expect(room.description).toBeNull();
    });

    it('maps a cabinet, including capacity, count and container path', () => {
        const cabinet = toCabinet({
            id: 'c1',
            name: 'Cabinet A1',
            place_type: 'FileCabinet',
            place_kind: 'cabinet',
            description: null,
            contained_in_place: 'r1',
            container_path: 'Main Hospital — Records Room — Cabinet A1',
            capacity: 100,
            source: 'place-service',
            folder_count: 7
        });
        expect(cabinet.id).toBe('c1');
        expect(cabinet.label).toBe('Cabinet A1');
        expect(cabinet.roomId).toBe('r1');
        expect(cabinet.capacity).toBe(100);
        expect(cabinet.folderCount).toBe(7);
        expect(cabinet.containerPath).toContain('Cabinet A1');
    });

    it('maps a worker with and without a role', () => {
        expect(toWorker({ id: 'w1', name: 'Mira', role: 'administrator' })).toEqual({
            id: 'w1',
            name: 'Mira',
            role: 'administrator'
        });
        expect(toWorker({ id: 'w2', name: 'Sam', role: null })).toEqual({
            id: 'w2',
            name: 'Sam',
            role: null
        });
    });

    it('maps stats', () => {
        const stats = toStats({
            patients: 6,
            folders: { total: 9, in_cabinet: 7, in_transit: 2 },
            places: { buildings: 3, rooms: 4, cabinets: 5 },
            moves_24h: 1
        });
        expect(stats.folders.inCabinet).toBe(7);
        expect(stats.folders.inTransit).toBe(2);
        expect(stats.moves24h).toBe(1);
    });
});

describe('ApiError', () => {
    it('carries status and body', () => {
        const err = new ApiError('boom', 422, { errors: { nhs_number: 'bad' } });
        expect(err).toBeInstanceOf(Error);
        expect(err.name).toBe('ApiError');
        expect(err.status).toBe(422);
        expect(err.body).toEqual({ errors: { nhs_number: 'bad' } });
    });
});
