-- Add date_format column to users table
ALTER TABLE users ADD COLUMN date_format TEXT NOT NULL DEFAULT 'dd-mm-YYYY';
