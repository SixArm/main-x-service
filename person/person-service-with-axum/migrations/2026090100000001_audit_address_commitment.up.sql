-- Let an audit row lose its caller's address without losing its hash.
--
-- Chain format `p2` binds `ip_commitment` where `p1` bound the address, so
-- that `ip_address` and `ip_salt` can be removed when the retention limit
-- passes (src/compliance/address_retention.rs). Rows already written stay
-- `p1`, because they bind the address itself.
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS ip_salt TEXT;
ALTER TABLE audit_log ADD COLUMN IF NOT EXISTS ip_commitment TEXT;

-- The sweep's row source.
CREATE INDEX IF NOT EXISTS audit_log_address_held
    ON audit_log ("timestamp") WHERE ip_address IS NOT NULL;
