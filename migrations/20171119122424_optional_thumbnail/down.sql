-- This file should undo anything in `up.sql`
ALTER TABLE files ALTER COLUMN thumbnail_path SET NOT NULL
