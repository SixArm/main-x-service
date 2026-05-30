-- Create audit tables for HIPAA compliance

-- Audit log table
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    user_id VARCHAR(255),
    action VARCHAR(50) NOT NULL,
    entity_type VARCHAR(50) NOT NULL,
    entity_id UUID NOT NULL,
    old_values JSONB,
    new_values JSONB,
    ip_address VARCHAR(45),  -- IPv4 (15) or IPv6 (45) address
    user_agent TEXT
);

-- Indexes for audit_log
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_log_entity ON audit_log(entity_type, entity_id);
CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);
CREATE INDEX idx_audit_log_action ON audit_log(action);

-- Create partition for audit logs by month (optional, for high-volume systems)
-- This is commented out but can be enabled for production
-- CREATE TABLE audit_log_y2024m12 PARTITION OF audit_log
--     FOR VALUES FROM ('2024-12-01') TO ('2025-01-01');
