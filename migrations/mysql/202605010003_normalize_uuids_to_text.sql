-- MySQL: UUIDs stored as CHAR(36) strings
-- This migration documents the UUID standardization across all backends
-- The Rust code now consistently binds UUIDs as strings (already compatible with CHAR(36))

-- No schema changes needed for MySQL; CHAR(36) already stores UUID strings correctly
-- Existing data is already in the proper format, and new inserts use string binding
