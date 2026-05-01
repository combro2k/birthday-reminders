-- PostgreSQL: UUID type is native and properly enforced
-- This migration documents the UUID standardization across all backends
-- PostgreSQL UUID columns are already in the correct format

-- No changes needed for PostgreSQL; UUID columns are properly typed as UUID
-- The Rust code now consistently binds UUIDs as strings, which PostgreSQL accepts and normalizes
