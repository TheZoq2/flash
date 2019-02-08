//infer_schema!("dotenv:DATABASE_URL");
table! {
    changes (id) {
        id -> Int4,
        timestamp -> Timestamp,
        json_data -> Text,
        affected_file -> Int4,
    }
}

table! {
    files (id) {
        id -> Int4,
        filename -> Text,
        thumbnail_path -> Nullable<Text>,
        creation_date -> Timestamp,
        is_uploaded -> Bool,
        tags -> Array<Text>,
    }
}

table! {
    syncpoints (id) {
        id -> Int4,
        last_change -> Timestamp,
    }
}

