extern crate serde_json;

use file_list::{FileList, FileLocation, FileListSource, FileListList};
use file_database;

use std::path::{PathBuf, Path};

use std::{fs};
use std::io::{Write, Read};

use error::{Result};

#[derive(Serialize, Deserialize)]
pub enum SaveableFileLocation
{
    Unsaved(PathBuf),
    Database(i32)
}

#[derive(Serialize, Deserialize)]
pub struct SaveableFileList
{
    pub source: FileListSource,
    pub files: Vec<SaveableFileLocation>
}

/**
  Converts a `FileList` to a json string
*/
fn saveable_file_list(list: &FileList) -> SaveableFileList
{
    let files = list.get_files().iter()
        .map(|location| {
            match *location
            {
                FileLocation::Unsaved(ref path) => SaveableFileLocation::Unsaved(path.clone()),
                FileLocation::Database(ref entry) => SaveableFileLocation::Database(entry.id)
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
  Generates a vector of `SaveableFileList`s from a `FileListList`. Only file lists
  originating from a directory will be saved
*/
pub fn saveable_file_list_list(list: &FileListList) -> Vec<SaveableFileList>
{
    list.get_lists().iter()
        .filter(|file_list| match *file_list.get_source(){
            FileListSource::Search => false,
            _ => true
        })
        .map(saveable_file_list)
        .collect()
}

/**
  Converts a vector of `SaveableFileList` to a `FileListList`
*/
fn file_list_list_from_saveable(saveable: Vec<SaveableFileList>, db: &file_database::FileDatabase)
    -> FileListList
{
    let file_lists = saveable.into_iter()
        .map(|saveable| list_from_saveable(saveable, db))
        .collect();

    FileListList::from_lists(file_lists)
}

/**
  Saves a `FileListList` to the specified file
*/
pub fn save_file_list_list(list: &[SaveableFileList], destination: &Path) -> Result<()>
{
    let mut file = fs::File::create(destination)?;

    let as_json = serde_json::to_string(&list)?;

    Ok(file.write_all(&as_json.into_bytes())?)
}

/**
  Reads a `FileListList` from the specified file
*/
pub fn read_file_list_list(file: &Path, db: &file_database::FileDatabase) -> Result<FileListList>
{
    // Ensure that the file exists
    if file.exists()
    {
        let mut file = fs::File::open(file)?;

        let mut json_string = String::new();
        file.read_to_string(&mut json_string)?;

        let saveable = serde_json::from_str(&json_string)?;

        Ok(file_list_list_from_saveable(saveable, db))
    }
    else
    {
        Ok(FileListList::new())
    }
}

#[cfg(test)]
mod file_list_persistence_tests
{
    use super::*;

    // Helpers
    fn assert_lists_are_equal(list1: &FileList, list2: &FileList)
    {
        for (original, read) in list1.get_files().iter().zip(list2.get_files().iter())
        {
            assert_eq!(list1.get_source(), list2.get_source());

            assert_eq!(original, read);
        }
    }

    fn dummy_database_list(db: &mut file_database::FileDatabase) -> FileList
    {
        FileList::from_locations(vec!(
                FileLocation::Database(db.add_new_file("filename", "thumbname", &vec!(), 0)),
                FileLocation::Database(db.add_new_file("filename", "thumbname", &vec!(), 0)),
                FileLocation::Unsaved(PathBuf::from("path"))
            ), FileListSource::Folder(PathBuf::from("test/media")))
    }


    // Tests
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

    #[test]
    fn tests_with_db()
    {
        file_database::db_test_helpers::run_test(|db| {
            let file_list = dummy_database_list(db);

            let saveable = saveable_file_list(&file_list);
            let decoded = list_from_saveable(saveable, db);


            assert_lists_are_equal(&file_list, &decoded);
        })
    }

    #[test]
    fn file_list_list_test()
    {
        file_database::db_test_helpers::run_test(|db| {
            let file_lists = vec!(
                    dummy_database_list(db),
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

    #[test]
    fn file_list_save_test()
    {
        file_database::db_test_helpers::run_test(|db| {
            let file_lists = vec!(
                    dummy_database_list(db),
                    FileList::from_directory(PathBuf::from("test/media"))
                );

            let file_list_list = FileListList::from_lists(file_lists);

            let save_path = db.get_file_save_path().join(&PathBuf::from("persistent_file_list.json"));
            save_file_list_list(&saveable_file_list_list(&file_list_list), &save_path).unwrap();

            let decoded = read_file_list_list(&save_path, db).unwrap();

            for (original, decoded) in file_list_list.get_lists().iter().zip(decoded.get_lists().iter())
            {
                assert_lists_are_equal(&original, &decoded)
            }
        })
    }
}
