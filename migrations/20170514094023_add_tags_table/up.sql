-- Your SQL goes here

CREATE TABLE tags (
    -- Unique ID given to the tag
    id SERIAL PRIMARY KEY,
    -- Filename of the uploaded image
    text TEXT NOT NULL UNIQUE
);

CREATE TABLE tag_links (
    -- ID of the linked file
    file_id INTEGER NOT NULL references files(id),
    -- ID of the linked tag
    tag_id INTEGER NOT NULL references tags(id),

    PRIMARY KEY (file_id, tag_id)
);
