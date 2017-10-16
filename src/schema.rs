//infer_schema!("dotenv:DATABASE_URL");
table! {
    files (id) {
        id -> Int4,
        filename -> Text,
        thumbnail_path -> Text,
        creation_date -> Nullable<Timestamp>,
        is_uploaded -> Bool,
        tags -> Array<Text>,
    }
}


table! {
    changes (id) {
        id -> Int4,
        timestamp -> Timestamp,
        json_data -> Text,
    }
}

table! {
    syncpoints (id) {
        id -> Int4,
        last_change -> Timestamp,
    }
}
