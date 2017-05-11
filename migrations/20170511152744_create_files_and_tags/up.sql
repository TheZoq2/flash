-- Your SQL goes here
CREATE TABLE files (
    -- Unique ID given to the image
    id SERIAL PRIMARY KEY,
    -- Filename of the uploaded image
    filename VARCHAR NOT NULL,
    -- Filename of the thumbnail
    thumbnail_path VARCHAR NOT NULL,
    -- The date the image was created
    creation_date TIMESTAMP WITH TIME ZONE,
    -- True once the file has been uploaded to the server
    is_uploaded BOOLEAN NOT NULL
)
