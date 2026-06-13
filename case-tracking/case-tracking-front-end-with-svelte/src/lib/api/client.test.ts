import { describe, it, expect } from 'vitest';
import { ApiError, toFolder, toMove, toStats } from './client';

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
