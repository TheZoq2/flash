-- Your SQL goes here
CREATE TABLE changes (
    id SERIAL PRIMARY KEY,
    -- Timestamp indicating the time the change was made
    timestamp TIMESTAMP not NULL,
    -- Data as serde json to make enums easy
    json_data TEXT not NULL
)
