extern crate serde_json;

use file_list::{FileList, FileLocation, FileListSource, FileListList};
use file_database;

use std::path::{PathBuf, Path};

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
fn saveable_file_list(list: &FileList) -> SaveableFileList
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

    SaveableFileList
    {
        source: list.get_source().clone(),
        files
    }
}

/**
  Converts a jsonified file list into a file list.

  If the file entry associated
  with a saved id has disappeared since the json was generated, it will be ignored
  and removed from the new list.
*/
fn list_from_saveable(saveable_list: SaveableFileList, db: &file_database::FileDatabase) -> FileList
{
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

    FileList::from_locations(files, saveable_list.source)
}



/**
  Generates a vector of `SaveableFileList`s from a FileListList
*/
fn saveable_file_list_list(list: &FileListList) -> Vec<SaveableFileList>
{
    list.get_lists().iter()
        .map(saveable_file_list)
        .collect()
}

fn file_list_list_from_saveable(saveable: Vec<SaveableFileList>, db: &file_database::FileDatabase)
    -> FileListList
{
    let file_lists = saveable.into_iter()
        .map(|saveable| list_from_saveable(saveable, db))
        .collect();

    FileListList::from_lists(file_lists)
}




#[cfg(test)]
mod file_list_persistence_tests
{
    use super::*;

    #[test]
    fn path_only_jsonification_test()
    {
        let file_list = FileList::from_directory(PathBuf::from("test/media"));


        file_database::db_test_helpers::run_test(|db| {
            let saveable = saveable_file_list(&file_list);

            let decoded = list_from_saveable(saveable, db);

            assert_eq!(file_list.get_source(), decoded.get_source());

            for (original, read) in file_list.get_files().iter().zip(decoded.get_files().iter())
            {
                assert_eq!(original, read);
            }
        });
    }

    fn assert_lists_are_equal(list1: &FileList, list2: &FileList)
    {
        for (original, read) in list1.get_files().iter().zip(list2.get_files().iter())
        {
            assert_eq!(original, read);
        }
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

            let saveable = saveable_file_list(&file_list);
            let decoded = list_from_saveable(saveable, db);

            assert_eq!(file_list.get_source(), decoded.get_source());

            assert_lists_are_equal(&file_list, &decoded);
        })
    }

    #[test]
    fn file_list_list_test()
    {
        file_database::db_test_helpers::run_test(|db| {
            let file_lists = vec!(
                    FileList::from_locations(vec!(
                                FileLocation::Database(db.add_new_file("filename", "thumbname", &vec!(), 0)),
                                FileLocation::Database(db.add_new_file("filename", "thumbname", &vec!(), 0)),
                                FileLocation::Unsaved(PathBuf::from("path"))
                            ), FileListSource::Search),
                    FileList::from_directory(PathBuf::from("test/media"))
                );

            let file_list_list = FileListList::from_lists(file_lists);

            let saveable = saveable_file_list_list(&file_list_list);
            let decoded = file_list_list_from_saveable(saveable, db);

            for (original, decoded) in file_list_list.get_lists().iter().zip(decoded.get_lists().iter())
            {
                assert_lists_are_equal(&original, &decoded)
            }
        })
    }
}
