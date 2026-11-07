-- Drop the transactional-outbox table (rollback).

DROP TABLE IF EXISTS event_outbox CASCADE;
