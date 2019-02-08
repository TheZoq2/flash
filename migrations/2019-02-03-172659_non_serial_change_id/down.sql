-- This file should undo anything in `up.sql`
CREATE SEQUENCE changes_id_seq;
SELECT setval('changes_id_seq', (SELECT max(id) FROM files));
ALTER TABLE changes ALTER COLUMN id SET DEFAULT nextval('changes_id_seq');
