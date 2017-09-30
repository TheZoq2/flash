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
