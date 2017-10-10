-- Your SQL goes here
CREATE TABLE syncpoints (
    id SERIAL PRIMARY KEY,
    last_change TIMESTAMP not NULL
)
