extern crate serde_json;

use file_list::{FileList, FileLocation, FileListSource};
use file_database;

use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
enum SaveableFileLocation
{
    Unsaved(PathBuf),
    Database(i32)
}
#[derive(Serialize, Deserialize)]
struct SaveableFileList
{
    pub source: FileListSource,
    pub files: Vec<SaveableFileLocation>
}

/**
  Converts a FileList to a json string
*/
fn jsonify_file_list(list: &FileList) -> Result<String, serde_json::Error>
{
    let files = list.get_files().iter()
        .map(|location| {
            match location
            {
                &FileLocation::Unsaved(ref path) => SaveableFileLocation::Unsaved(path.clone()),
                &FileLocation::Database(ref entry) => SaveableFileLocation::Database(entry.id)
            }
        })
        .collect();

    let saveable = SaveableFileList
    {
        source: list.get_source().clone(),
        files
    };

    serde_json::to_string(&saveable)
}

/**
  Converts a jsonified file list into a file list.

  If the file entry associated
  with a saved id has disappeared since the json was generated, it will be ignored
  and removed from the new list.
*/
fn file_list_from_json(json: &str, db: &file_database::FileDatabase) -> Result<FileList, serde_json::Error>
{
    let saveable_list = serde_json::from_str::<SaveableFileList>(json)?;

    let files = saveable_list.files.into_iter()
        .filter_map(|location| {
            match location
            {
                SaveableFileLocation::Unsaved(path) => Some(FileLocation::Unsaved(path)),
                SaveableFileLocation::Database(id) => {
                    match db.get_file_with_id(id)
                    {
                        Some(file) => Some(FileLocation::Database(file)),
                        None => None
                    }
                }
            }
        })
        .collect();

    Ok(FileList::from_locations(files, saveable_list.source))
}




#[cfg(test)]
mod file_list_persistence_tests
{
    use super::*;

    #[test]
    fn path_only_jsonification_test()
    {
        let file_list = FileList::from_directory(PathBuf::from("test/media"));

        let json = jsonify_file_list(&file_list).unwrap();

        file_database::db_test_helpers::run_test(|db| {
            let decoded = file_list_from_json(&json, db).unwrap();

            assert_eq!(file_list.get_source(), decoded.get_source());

            for (original, read) in file_list.get_files().iter().zip(decoded.get_files().iter())
            {
                assert_eq!(original, read);
            }
        });
    }

    #[test]
    fn tests_with_db()
    {
        file_database::db_test_helpers::run_test(|db| {
            let file_locations = vec!(
                    FileLocation::Database(db.add_new_file("filename", "thumbname", &vec!(), 0)),
                    FileLocation::Database(db.add_new_file("filename", "thumbname", &vec!(), 0)),
                    FileLocation::Unsaved(PathBuf::from("path"))
                );

            let file_list = FileList::from_locations(file_locations, FileListSource::Search);

            let json = jsonify_file_list(&file_list).unwrap();
            let decoded = file_list_from_json(&json, db).unwrap();

            assert_eq!(file_list.get_source(), decoded.get_source());

            for (original, read) in file_list.get_files().iter().zip(decoded.get_files().iter())
            {
                assert_eq!(original, read);
            }
        })
    }
}
