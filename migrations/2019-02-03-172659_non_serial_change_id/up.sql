ALTER TABLE changes ALTER COLUMN id DROP DEFAULT;
ALTER SEQUENCE changes_id_seq OWNED BY NONE;
DROP SEQUENCE changes_id_seq;