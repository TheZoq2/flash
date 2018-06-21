
use serde_json;

use iron::*;
use persistent::Write;

use std::path::{Path, PathBuf};

use std::sync::Mutex;
use std::sync::Arc;

use chrono::{NaiveDateTime, Utc};

use file_database;
use file_database::FileDatabase;
use file_list::{FileListList, FileLocation};
use file_list_worker;
use persistent_file_list;
use file_util::sanitize_tag_names;
use file_util::{get_semi_unique_identifier};
use request_helpers::{get_get_variable, setup_db_connection};
use file_handler::{save_file, FileSavingWorkerResults, ThumbnailStrategy};
use byte_source::ByteSource;
use changelog;
use changelog::ChangeCreationPolicy;

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
    NewDatabaseEntry(FileLocation, FileSavingWorkerResults),
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
    let fdb = setup_db_connection(request)?;

    let file_location = {
        let mutex = request.get::<Write<FileListList>>().unwrap();
        let file_list_list = mutex.lock().unwrap();

        get_file_list_object(&*file_list_list, list_id, file_index)?
    };

    let file_storage_folder = fdb.get_file_save_path();

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
        // TODO: Don't return an absolute path for unsaved files. Do we even need the path? (maybe
        // in flash_cli)
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
            let tags = get_tags_from_request(request)?;
            let current_time = NaiveDateTime::from_timestamp(Utc::now().timestamp(), 0);
            match handle_save_request(&fdb, &file_location, &tags, current_time)? {
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
        Ok(result) => Ok(sanitize_tag_names(&result)),
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
    db: &FileDatabase,
    file_location: &FileLocation,
    tags: &[String],
    change_timestamp: NaiveDateTime
) -> Result<FileSaveRequestResult> {
    match *file_location {
        FileLocation::Unsaved(ref path) => {
            match save_new_file(db, path, tags, change_timestamp) {
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
            match update_stored_file_tags(db, old_file, tags, change_timestamp) {
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
    db: &FileDatabase,
    original_path: &Path,
    tags: &[String],
    current_time: NaiveDateTime
) -> Result<(file_database::File, FileSavingWorkerResults)> {
    let file_identifier = get_semi_unique_identifier();

    let file = ByteSource::File(original_path.into());

    let extension = match original_path.extension() {
        Some(val) => Ok(val),
        None => Err(ErrorKind::NoFileExtension(original_path.to_owned().into()))
    }?;


    save_file(
        file,
        ThumbnailStrategy::Generate,
        file_identifier,
        tags,
        db,
        changelog::ChangeCreationPolicy::Yes(current_time),
        &extension.to_string_lossy(),
        current_time.timestamp() as u64
    )
}


/**
  Updates a specified file in the database with new tags
*/
fn update_stored_file_tags(
    db: &FileDatabase,
    old_entry: &file_database::File,
    tags: &[String],
    change_timestamp: NaiveDateTime
) -> Result<file_database::File> {
    db.change_file_tags(old_entry, tags, ChangeCreationPolicy::Yes(change_timestamp))
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

    use chrono::NaiveDate;

    use changelog::ChangeCreationPolicy;

    fn dummy_database_entry(file_path: &str, thumbnail_path: &str) -> file_database::File {
        file_database::File {
            id: 0,
            filename: file_path.to_owned(),
            thumbnail_path: Some(thumbnail_path.to_owned()),
            creation_date: NaiveDate::from_ymd(2016,1,1).and_hms(0,0,0),
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


    // TODO: Use db_test! instead of this test runner
    #[test]
    fn database_related_tests() {
        let fdb = file_database::db_test_helpers::get_database();
        let fdb = fdb.lock().unwrap();

        fdb.reset();
        saving_a_file_without_extension_fails(&fdb);

        fdb.reset();
        file_list_saving_works(&fdb);

        fdb.reset();
        file_list_updates_work(&fdb);

        fdb.reset();
        file_list_save_requests_work(&fdb);
    }

    fn saving_a_file_without_extension_fails(fdb: &FileDatabase) {
        let tags = vec!("test1".to_owned(), "test2".to_owned());
        assert_matches!(
                save_new_file(
                    fdb,
                    &PathBuf::from("test"),
                    &tags,
                    NaiveDate::from_ymd(2017,1,1).and_hms(0,0,0)
                ),
                Err(Error(ErrorKind::NoFileExtension(_), _))
            );
    }

    fn file_list_saving_works(fdb: &FileDatabase) {
        let tags = vec!("test1".to_owned(), "test2".to_owned());

        let src_path = PathBuf::from("test/media/10x10.png");
        let (result, worker_results) = save_new_file(
                &fdb,
                &src_path,
                &tags,
                NaiveDate::from_ymd(2017, 1, 1).and_hms(0,0,0)
            )
            .unwrap();


        let save_result = worker_results.file.recv().unwrap();
        assert_matches!(save_result, Ok(()));
        let thumbnail_save_result = worker_results.thumbnail.unwrap().recv().expect("Thumbnail creator crashed");
        assert_matches!(thumbnail_save_result, Ok(()));

        let full_path = {
            PathBuf::from(fdb.get_file_save_path()).join(PathBuf::from(&result.filename))
        };

        // Make sure that the saved file exists
        assert!(full_path.exists());

        //Make sure that the file was actually added to the database
        assert!(
            fdb.search_files(search::SavedSearchQuery::with_tags((tags, vec!())))
                .iter()
                .fold(false, |acc, file| { acc || file.id == result.id })
            )
    }

    fn file_list_save_requests_work(fdb: &FileDatabase) {
        let old_path = PathBuf::from("test/media/10x10.png");

        let tags = vec!("new1".to_owned());

        let saved_entry = {
            let timestamp = NaiveDate::from_ymd(2017,1,1).and_hms(0,0,0);
            let result = handle_save_request(
                fdb,
                &FileLocation::Unsaved(old_path),
                &tags,
                timestamp
            );

            assert_matches!(result, Ok(_));
            let result = result.unwrap();

            assert_matches!(result, FileSaveRequestResult::NewDatabaseEntry(FileLocation::Database(_), _));
            match result {
                FileSaveRequestResult::NewDatabaseEntry(file_entry, worker_receivers) => {
                    assert_matches!(worker_receivers.file.recv().unwrap(), Ok(()));
                    assert_matches!(worker_receivers
                            .thumbnail
                            .expect("Thumbnail generator channel not created")
                            .recv()
                            .expect("Thumbnail saving failed"), Ok(())
                        );

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
                fdb
                    .search_files(search::SavedSearchQuery::with_tags((tags, vec!())))
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
            );
    }

    fn file_list_updates_work(fdb: &FileDatabase) {
        let old_tags = vec!(String::from("old"));
        let old_location = {
            fdb.add_new_file(
                1,
                "test",
                Some("thumb"),
                &old_tags,
                0,
                ChangeCreationPolicy::No
            )
        };

        let tags = vec!("new1".to_owned());

        let saved_entry = {
            let timestamp = NaiveDate::from_ymd(2017,1,1).and_hms(0,0,0);

            let result =
                handle_save_request(
                    &fdb,
                    &FileLocation::Database(old_location),
                    &tags,
                    timestamp
                );

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
                fdb
                    .search_files(search::SavedSearchQuery::with_tags((tags, vec!())))
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
            );

        // Make sure the old entry was removed
        assert!(
                fdb
                    .search_files(search::SavedSearchQuery::with_tags((old_tags, vec!())))
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
                ==
                false
            );
    }
}
