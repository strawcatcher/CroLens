-- One-time schema migration for existing D1 databases.
-- Adds ip and request size fields for improved observability.

ALTER TABLE request_logs ADD COLUMN ip_address TEXT;
ALTER TABLE request_logs ADD COLUMN request_size INTEGER;

