-- This file should undo anything in `up.sql`
ALTER TABLE files ALTER COLUMN creation_date DROP NOT NULL
