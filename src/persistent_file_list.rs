extern crate serde_json;

use file_list::{FileList, FileLocation};
use file_database;

use std::path::PathBuf;

#[derive(Serialize)]
enum SaveableFileLocation
{
    Unsaved(PathBuf),
    Database(i32)
}

/**
  Converts a FileList to a json string
*/
fn jsonify_file_list(list: &FileList, filename: PathBuf) -> String
{
    let saveable = list.iter()
        .map(|location| {
            match *location
            {
                FileLocation::Unsaved(path) => SaveableFileLocation::Unsaved(path),
                FileLocation::Database(entry) => SaveableFileLocation::Database(entry.id)
            }
        })
        .collect();

    serde_json::to_string(saveable)
}

/**
  Converts a jsonified file list into a file list
*/
fn file_list_from_json(json: &str, db: &file_database::FileDatabase) -> FileList
{
    let saveable_list = serde_json::from_str::<Vec<SaveableFileLocation>>(json);

    saveable_list.iter()
        .map(|location| {
            match *location
            {
                SaveableFileLocation::Unsaved(path) => FileLocation::Unsaved(path),
                SaveableFileLocation::Database(id) => {
                    FileLocation::Database(db.get_file_with_id(id))
                }
            }
        })
        .collect()
}
