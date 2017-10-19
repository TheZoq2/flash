-- This file should undo anything in `up.sql`
CREATE SEQUENCE files_id_seq;
SELECT setval('files_id_seq', (SELECT max(id) FROM files));
ALTER TABLE files ALTER COLUMN id SET DEFAULT nextval('files_id_seq');
