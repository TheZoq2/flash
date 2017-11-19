use std::path::PathBuf;

use serde_json;

use iron::*;

use persistent::Write;
use std::fs;

use std::sync::Mutex;
use std::sync::Arc;

use std::path::Path;

use std::thread;

use std::io;
use std::sync::mpsc::{channel, Receiver};

use file_database;
use file_database::FileDatabase;
use file_list::{FileListList, FileLocation};
use file_list_worker;
use persistent_file_list;
use file_util::sanitize_tag_names;
use file_util::{generate_thumbnail, get_semi_unique_identifier, get_file_timestamp};
use request_helpers::get_get_variable;
use file_handler::{save_file, FileSavingResult, ThumbnailStrategy};

use file_list_response;

use error::{Result, ErrorKind, Error};

enum FileAction {
    GetData,
    GetFile,
    GetFilename,
    GetThumbnail,
    Save
}

impl FileAction {
    fn try_parse(action: &str) -> Option<FileAction>
    {
        match action {
            "get_data" => Some(FileAction::GetData),
            "get_file" => Some(FileAction::GetFile),
            "get_filename" => Some(FileAction::GetFilename),
            "get_thumbnail" => Some(FileAction::GetThumbnail),
            "save" => Some(FileAction::Save),
            _ => None
        }
    }
}


////////////////////////////////////////////////////////////////////////////////
//                      Helper types used for passing
//                      response data between functions
////////////////////////////////////////////////////////////////////////////////
#[derive(Serialize)]
struct FileData {
    file_path: String,
    thumbnail_path: String,
    tags: Vec<String>,
}

impl FileData {
    fn from_database(source: file_database::File) -> FileData {
        FileData {
            file_path: source.filename,
            thumbnail_path: source.thumbnail_path.unwrap_or_else(|| String::from("")),
            tags: source.tags,
        }
    }

    fn from_path(source: PathBuf) -> FileData {
        FileData {
            file_path: String::from(source.to_string_lossy()),
            thumbnail_path: String::from(source.to_string_lossy()),
            tags: vec![],
        }
    }
}

#[derive(Debug)]
enum FileSaveRequestResult {
    NewDatabaseEntry(FileLocation, Receiver<FileSavingResult>),
    UpdatedDatabaseEntry(FileLocation),
}


////////////////////////////////////////////////////////////////////////////////
//                      Public request handlers
////////////////////////////////////////////////////////////////////////////////

/**
  Handles requests for actions dealing with specific entries in file lists
*/
pub fn file_list_request_handler(request: &mut Request) -> IronResult<Response> {
    let action_str = get_get_variable(request, "action")?;

    if let Some(action) = file_list_response::GlobalAction::try_parse(&action_str) {
        file_list_response::global_list_action_handler(request, &action)
    }
    else if let Some(action) = file_list_response::ListAction::try_parse(&action_str) {
        file_list_response::list_action_handler(request, &action)
    }
    else if let Some(action) = FileAction::try_parse(&action_str) {
        file_request_handler(request, &action)
    }
    else {
        Err(Error::from(ErrorKind::UnknownAction(action_str)))?
    }
}

/**
  Handles requests for actions dealing with specific entries in file lists
*/
fn file_request_handler(request: &mut Request, action: &FileAction) -> IronResult<Response> {
    let (list_id, file_index) = read_request_list_id_index(request)?;

    let file_location = {
        let mutex = request.get::<Write<FileListList>>().unwrap();
        let file_list_list = mutex.lock().unwrap();

        get_file_list_object(&*file_list_list, list_id, file_index)?
    };

    let file_storage_folder = {
        let mutex = request.get::<Write<FileDatabase>>().unwrap();
        let db = mutex.lock().unwrap();

        db.get_file_save_path()
    };

    match *action {
        FileAction::GetData => {
            let file_data = file_data_from_file_location(&file_location);
            Ok(Response::with(
                (status::Ok, serde_json::to_string(&file_data).unwrap()),
            ))
        }
        FileAction::GetFile => {
            let path = get_file_location_path(&file_storage_folder, &file_location);
            Ok(Response::with((status::Ok, path)))
        }
        FileAction::GetFilename => {
            let path = match file_location {
                FileLocation::Database(entry) => entry.filename,
                FileLocation::Unsaved(path) => String::from(path.to_string_lossy())
            };
            Ok(Response::with((status::Ok, path)))
        }
        FileAction::GetThumbnail => {
            let path = get_file_list_thumbnail(&file_storage_folder, &file_location);
            Ok(Response::with((status::Ok, path)))
        }
        FileAction::Save => {
            let db = request.get::<Write<FileDatabase>>().unwrap();
            let tags = get_tags_from_request(request)?;

            match handle_save_request(db, &file_location, &tags)? {
                FileSaveRequestResult::NewDatabaseEntry(new_location, _) |
                FileSaveRequestResult::UpdatedDatabaseEntry(new_location) => {
                    let mut file_list_list = request.get::<Write<FileListList>>().unwrap();
                    update_file_list(&mut file_list_list, list_id, file_index, &new_location);

                    send_file_list_save_command(request);

                    Ok(Response::with((status::Ok, "\"ok\"")))
                }
            }
        }
    }
}



////////////////////////////////////////////////////////////////////////////////
///                     Private functions for getting data
///                     out of iron requests
////////////////////////////////////////////////////////////////////////////////

fn read_request_list_id_index(request: &mut Request) -> Result<(usize, usize)> {
    let list_id = file_list_response::read_request_list_id(request)?;

    let file_index = get_get_variable(request, "index")?;

    let file_index = match file_index.parse::<usize>() {
        Ok(val) => val,
        Err(_) => {
            return Err(ErrorKind::InvalidVariableType("index".into(), "usize".into()).into());
        }
    };

    Ok((list_id, file_index))
}



fn get_tags_from_request(request: &mut Request) -> Result<Vec<String>> {
    //Get the important information from the request.
    let tag_string = get_get_variable(request, "tags")?;

    match serde_json::from_str::<Vec<String>>(&tag_string) {
        Ok(result) => Ok(sanitize_tag_names(&result).unwrap()),
        Err(e) => Err(ErrorKind::InvalidVariableType("tags".into(), format!("{:?}", e)).into()),
    }
}


fn send_file_list_save_command(request: &mut Request) {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();

    // Save the current file lists to disk
    let saveable_file_list = {
        let fll = file_list_list.lock().unwrap();

        persistent_file_list::saveable_file_list_list(&fll)
    };

    let flw = request.get::<Write<file_list_worker::Commander>>().unwrap();

    flw.lock()
        .unwrap()
        .send(file_list_worker::Command::Save(saveable_file_list))
        .unwrap();
}


////////////////////////////////////////////////////////////////////////////////
//                      Private functions for generating
//                      replies to file requests
////////////////////////////////////////////////////////////////////////////////

/**
  Updates the specified `file_list` with a new `FileLocation`
*/
fn update_file_list(
    file_list_list: &mut Arc<Mutex<FileListList>>,
    list_id: usize,
    file_index: usize,
    new_location: &FileLocation,
) {
    let mut file_list_list = file_list_list.lock().unwrap();
    file_list_list.edit_file_list_entry(list_id, file_index, new_location);
}

/**
  Saves the specified tags for the file. If a new `FileLocation` has been created,
  it is returned. Otherwise None. If saving failed an error is returned
*/
fn handle_save_request(
    db: Arc<Mutex<FileDatabase>>,
    file_location: &FileLocation,
    tags: &[String],
) -> Result<FileSaveRequestResult> {
    match *file_location {
        FileLocation::Unsaved(ref path) => {
            match save_new_file(db, path, tags) {
                Ok((db_entry, save_result_rx)) => {
                    Ok(FileSaveRequestResult::NewDatabaseEntry(
                        FileLocation::Database(db_entry),
                        save_result_rx,
                    ))
                }
                Err(e) => Err(e),
            }
        }
        FileLocation::Database(ref old_file) => {
            match update_stored_file(db, old_file, tags) {
                Ok(db_entry) => {
                    Ok(FileSaveRequestResult::UpdatedDatabaseEntry(
                        FileLocation::Database(db_entry),
                    ))
                }
                Err(e) => Err(e),
            }
        }
    }
}

/**
  Saves a specified file in the `Filedatabase`
*/
fn save_new_file(
    db: Arc<Mutex<FileDatabase>>,
    original_path: &Path,
    tags: &[String],
) -> Result<(file_database::File, Receiver<FileSavingResult>)> {
    let file_identifier = get_semi_unique_identifier();

    let mut fdb = db.lock().unwrap();
    save_file(original_path, ThumbnailStrategy::Generate, &mut fdb, file_identifier, tags)
}


/**
  Updates a specified file in the database with new tags
*/
fn update_stored_file(
    db: Arc<Mutex<FileDatabase>>,
    old_entry: &file_database::File,
    tags: &[String],
) -> Result<file_database::File> {
    let db = db.lock().unwrap();
    db.change_file_tags(old_entry, tags)
}


/**
  Returns a `FileData` struct for the specified file location
*/
fn file_data_from_file_location(file: &FileLocation) -> FileData {
    // Lock the file list and try to fetch the file
    match *file {
        FileLocation::Unsaved(ref path) => FileData::from_path(path.clone()),
        FileLocation::Database(ref db_entry) => FileData::from_database(db_entry.clone()),
    }
}

/**
  Returns a the path to a `FileLocation`
*/
fn get_file_location_path(storage_folder: &Path, file: &FileLocation) -> PathBuf {
    match *file {
        FileLocation::Unsaved(ref path) => path.clone(),
        FileLocation::Database(ref db_entry) => {
            PathBuf::from(storage_folder.join(db_entry.filename.clone()))
        }
    }
}

/**
  Returns the path to the thumbnail of a `FileLocation`
*/
fn get_file_list_thumbnail(storage_folder: &Path, file: &FileLocation) -> PathBuf {
    match *file {
        FileLocation::Unsaved(ref path) => path.clone(),
        FileLocation::Database(ref db_entry) => {
            PathBuf::from(storage_folder.join(
                    db_entry.thumbnail_path.clone().unwrap_or_else(|| String::from(""))
                ))
        }
    }
}

/**
  Returns a `FileLocation` from a `FileListList`, a list id and a file id
*/
fn get_file_list_object(
    file_list_list: &FileListList,
    list_id: usize,
    file_index: usize,
) -> Result<FileLocation> {
    let file_list = match file_list_list.get(list_id) {
        Some(list) => list,
        None => {
            return Err(ErrorKind::NoSuchList(list_id).into());
        }
    };

    match file_list.get(file_index) {
        Some(file) => Ok(file.clone()),
        None => Err(ErrorKind::NoSuchFileInList(list_id, file_index).into()),
    }
}

/**
  Returns the index of the last file that was saved to the database in a specific file
*/
fn get_last_saved(file_list: &[FileLocation]) -> Option<usize>
{
    file_list.iter()
        .enumerate()
        .fold(None, |last, (id, file)| {
            match *file {
                FileLocation::Database(_) => Some(id),
                _ => last
            }
        })
}


/*
  Database tests are not currently run because the database can't be shared
  between test threads
*/
#[cfg(test)]
mod file_request_tests {
    use super::*;

    use search;

    use file_list::{FileList, FileListSource};


    fn dummy_database_entry(file_path: &str, thumbnail_path: &str) -> file_database::File {
        file_database::File {
            id: 0,
            filename: file_path.to_owned(),
            thumbnail_path: thumbnail_path.to_owned(),
            creation_date: None,
            is_uploaded: true,
            tags: vec![],
        }
    }

    fn make_dummy_file_list_list() -> FileListList {
        let mut fll = FileListList::new();

        let flist1 = FileList::from_locations(
            vec![
                FileLocation::Unsaved(PathBuf::from("l0f0")),
                FileLocation::Unsaved(PathBuf::from("l0f1")),
                FileLocation::Unsaved(PathBuf::from("l0f2")),
            ],
            FileListSource::Search,
        );

        let flist2 = FileList::from_locations(
            vec![
                FileLocation::Unsaved(PathBuf::from("l1f0")),
                FileLocation::Unsaved(PathBuf::from("l1f1")),
            ],
            FileListSource::Search,
        );

        let flist3 = FileList::from_locations(
            vec![
                FileLocation::Database(dummy_database_entry("test1", "thumb1")),
                FileLocation::Database(dummy_database_entry("test2", "thumb2")),
            ],
            FileListSource::Search,
        );

        fll.add(flist1);
        fll.add(flist2);
        fll.add(flist3);

        fll
    }

    #[test]
    fn file_list_object_test() {
        let fll = make_dummy_file_list_list();

        // Getting files from the first list works
        assert_eq!(
            get_file_list_object(&fll, 0, 0).unwrap(),
            FileLocation::Unsaved(PathBuf::from("l0f0"))
        );
        assert_eq!(
            get_file_list_object(&fll, 0, 2).unwrap(),
            FileLocation::Unsaved(PathBuf::from("l0f2"))
        );
        // Getting files from the second list works
        assert_eq!(
            get_file_list_object(&fll, 1, 1).unwrap(),
            FileLocation::Unsaved(PathBuf::from("l1f1"))
        );

        //Out of bounds
        assert!(get_file_list_object(&fll, 0, 3).is_err());
        assert!(get_file_list_object(&fll, 1, 2).is_err());
        assert!(get_file_list_object(&fll, 3, 2).is_err());
    }

    #[test]
    fn updating_file_list_entries_works() {
        let mut fll = Arc::new(Mutex::new(make_dummy_file_list_list()));

        let new_db_entry = FileLocation::Database(dummy_database_entry("yolo", "swag"));

        update_file_list(&mut fll, 0, 0, &new_db_entry);

        let fll = fll.lock().unwrap();
        assert_matches!(get_file_list_object(&fll, 0, 0).unwrap()
                        , FileLocation::Database(_))
    }


    #[test]
    fn database_related_tests() {
        let outer_fdb = file_database::db_test_helpers::get_database();

        let fdb = outer_fdb.lock().unwrap();

        fdb.lock().unwrap().reset();
        saving_a_file_without_extension_fails(fdb.clone());

        fdb.lock().unwrap().reset();
        file_list_saving_works(fdb.clone());

        fdb.lock().unwrap().reset();
        file_list_updates_work(fdb.clone());

        fdb.lock().unwrap().reset();
        file_list_save_requests_work(fdb.clone());
    }

    fn saving_a_file_without_extension_fails(fdb: Arc<Mutex<FileDatabase>>) {
        let tags = vec!("test1".to_owned(), "test2".to_owned());
        assert_matches!(
                save_new_file(fdb.clone(), &PathBuf::from("test"), &tags),
                Err(Error(ErrorKind::NoFileExtension(_), _))
            );
    }

    fn file_list_saving_works(fdb: Arc<Mutex<FileDatabase>>) {
        let tags = vec!("test1".to_owned(), "test2".to_owned());

        let src_path = PathBuf::from("test/media/DSC_0001.JPG");
        let (result, save_result_rx) = save_new_file(fdb.clone(), &src_path, &tags).unwrap();


        let save_result = save_result_rx.recv().unwrap();
        assert_matches!(save_result, FileSavingResult::Success);

        let full_path = {
            let fdb = fdb.lock().unwrap();
            PathBuf::from(fdb.get_file_save_path()).join(PathBuf::from(&result.filename))
        };

        // Make sure that the saved file exists
        assert!(full_path.exists());

        //Make sure that the file was actually added to the database
        match fdb.lock() {
            Ok(fdb) => {
                assert!(
                    fdb.search_files(search::SavedSearchQuery::with_tags((tags, vec!())))
                        .iter()
                        .fold(false, |acc, file| { acc || file.id == result.id })
                    )
            }
            Err(e) => panic!("{:?}", e),
        }
    }

    fn file_list_save_requests_work(fdb: Arc<Mutex<FileDatabase>>) {
        let old_path = PathBuf::from("test/media/DSC_0001.JPG");

        let tags = vec!("new1".to_owned());

        let saved_entry = {
            let result = handle_save_request(fdb.clone(), &FileLocation::Unsaved(old_path), &tags);

            assert_matches!(result, Ok(_));
            let result = result.unwrap();

            assert_matches!(result, FileSaveRequestResult::NewDatabaseEntry(FileLocation::Database(_), _));
            match result {
                FileSaveRequestResult::NewDatabaseEntry(file_entry, receiver) => {
                    assert_matches!(receiver.recv().unwrap(), FileSavingResult::Success);

                    match file_entry {
                        FileLocation::Database(result) => {
                            assert!(result.tags.contains(&String::from("new1")));
                            assert!(!result.tags.contains(&String::from("old")));
                            result
                        }
                        _ => panic!("Unreachable branch"),
                    }
                }
                _ => panic!("Unreachable branch"),
            }
        };

        //Make sure that the file was actually added to the database
        assert!(
                fdb.lock().unwrap()
                    .search_files(search::SavedSearchQuery::with_tags((tags, vec!())))
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
            );
    }

    fn file_list_updates_work(fdb: Arc<Mutex<FileDatabase>>) {
        let old_tags = vec!(String::from("old"));
        let old_location = {
            let mut fdb = fdb.lock().unwrap();
            fdb.add_new_file(1, "test", "thumb", &old_tags, 0)
        };

        let tags = vec!("new1".to_owned());

        let saved_entry = {
            let result =
                handle_save_request(fdb.clone(), &FileLocation::Database(old_location), &tags);

            assert_matches!(result, Ok(_));
            let result = result.unwrap();

            assert_matches!(result, FileSaveRequestResult::UpdatedDatabaseEntry(FileLocation::Database(_)));
            match result {
                FileSaveRequestResult::UpdatedDatabaseEntry(result) => {
                    match result {
                        FileLocation::Database(result) => {
                            assert!(result.tags.contains(&String::from("new1")));
                            assert!(!result.tags.contains(&String::from("old")));
                            result
                        }
                        _ => panic!("Unreachable branch"),
                    }
                }
                _ => panic!("Unreachable branch"),
            }
        };

        // Make sure that the file was actually added to the database
        assert!(
                fdb.lock().unwrap()
                    .search_files(search::SavedSearchQuery::with_tags((tags, vec!())))
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
            );

        // Make sure the old entry was removed
        assert!(
                fdb.lock().unwrap()
                    .search_files(search::SavedSearchQuery::with_tags((old_tags, vec!())))
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
                ==
                false
            );
    }
}
