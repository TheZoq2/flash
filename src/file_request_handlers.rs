use std::path::PathBuf;

use urlencoded::UrlEncodedQuery;
use rustc_serialize::json;

use serde_json;

use iron::*;

use persistent::{Write};
use std::option::Option;
use std::fs;

use std::sync::Mutex;
use std::sync::Arc;

use std::thread;
use std::path::Path;

use std::io;
use std::sync::mpsc::{channel, Receiver};

use file_database;
use file_database::{FileDatabase};
use file_list::{FileList, FileListList, FileListSource, FileLocation};
use file_util::{sanitize_tag_names};
use file_util::{
    generate_thumbnail,
    get_semi_unique_identifier,
    get_file_timestamp
};

use file_request_error::{
    FileRequestError,
    err_invalid_variable_type
};

////////////////////////////////////////////////////////////////////////////////
//                      Helper types used for passing
//                      response data between functions
////////////////////////////////////////////////////////////////////////////////
#[derive(Serialize)]
struct FileData
{
    file_path: String,
    thumbnail_path: String,
    tags: Vec<String>,
}

impl FileData
{
    fn from_database(source: file_database::File) -> FileData
    {
        FileData {
            file_path: source.filename,
            thumbnail_path: source.thumbnail_path,
            tags: source.tags
        }
    }

    fn from_path(source: PathBuf) -> FileData
    {
        FileData {
            file_path: String::from(source.to_string_lossy()),
            thumbnail_path: String::from(source.to_string_lossy()),
            tags: vec!()
        }
    }
}


/**
  Serializable list response that contains data about a file list
*/
#[derive(Serialize)]
struct ListResponse
{
    pub id: usize,
    pub length: Option<usize>
}

#[derive(Debug)]
enum FileSavingResult
{
    Success,
    Failure(io::Error)
}

#[derive(Debug)]
enum FileSaveRequestResult
{
    NewDatabaseEntry(FileLocation, Receiver<FileSavingResult>),
    UpdatedDatabaseEntry(FileLocation)
}

////////////////////////////////////////////////////////////////////////////////
//                      Public request handlers
////////////////////////////////////////////////////////////////////////////////

/**
  Handles requests for creating a filelist from a directory path
*/
pub fn directory_list_handler(request: &mut Request) -> IronResult<Response>
{
    let path = get_get_variable(request, "path")?;

    let file_list_list = request.get::<Write<FileListList>>().unwrap();

    // Check if path is a valid path
    let path = PathBuf::from(&path);

    // Lock the file list and insert a new list
    let file_list_id = {
        let mut file_list_list = file_list_list.lock().unwrap();

        match file_list_list.get_id_with_source(FileListSource::Folder(path.clone()))
        {
            Some(id) => id,
            None => file_list_list.add(FileList::from_directory(path))
        }
    };

    let list_response = reply_to_file_list_request(file_list_list, file_list_id);
    Ok(Response::with((status::Ok, serde_json::to_string(&list_response).unwrap())))
}

/**
  Handles requests for actions dealing with specific entries in file lists
*/
pub fn file_list_request_handler(request: &mut Request) -> IronResult<Response>
{
    let action = get_get_variable(request, "action")?;

    let (list_id, file_index) = read_request_list_id_index(request)?;

    let file_location = {
        let mutex = request.get::<Write<FileListList>>().unwrap();
        let file_list_list = mutex.lock().unwrap();

        get_file_list_object(&*file_list_list, list_id, file_index)?
    };

    match action.as_str() {
        "get_data" => {
            let file_data = file_data_from_file_location(&file_location);
            Ok(Response::with(
                    (status::Ok, serde_json::to_string(&file_data).unwrap())
                ))
        },
        "get_file" => {
            let path = get_file_list_file(&file_location);
            Ok(Response::with((status::Ok, path)))
        },
        "get_thumbnail" => {
            let path = get_file_list_thumbnail(&file_location);
            Ok(Response::with((status::Ok, path)))
        }
        "save" => {
            let db = request.get::<Write<FileDatabase>>().unwrap();
            let tags = get_tags_from_request(request)?;

            match handle_save_request(db, &file_location, &tags)? {
                FileSaveRequestResult::NewDatabaseEntry(new_location, _) => {
                    update_file_list(
                                &mut request.get::<Write<FileListList>>().unwrap(),
                                list_id,
                                file_index,
                                new_location
                            );
                    Ok(Response::with((status::Ok, "")))
                },
                FileSaveRequestResult::UpdatedDatabaseEntry(new_location) => {
                    update_file_list(
                                &mut request.get::<Write<FileListList>>().unwrap(),
                                list_id,
                                file_index,
                                new_location
                            );
                    Ok(Response::with((status::Ok, "")))
                }
            }
        }
        val => {
            let message = format!("Unrecognised `action`: {}", val);
            Ok(Response::with((status::NotFound, message)))
        }
    }
}


////////////////////////////////////////////////////////////////////////////////
///                     Private functions for getting data
///                     out of iron requests
////////////////////////////////////////////////////////////////////////////////
fn read_request_list_id_index(request: &mut Request) -> Result<(usize, usize), FileRequestError>
{
    let list_id = get_get_variable(request, "list_id")?;

    let list_id = match list_id.parse::<usize>() {
        Ok(val) => val,
        Err(_) => {
            return Err(err_invalid_variable_type("list_id", "usize"));
        }
    };

    let file_index = get_get_variable(request, "index")?;

    let file_index = match file_index.parse::<usize>() {
        Ok(val) => val,
        Err(_) => {
            return Err(err_invalid_variable_type("index", "usize"));
        }
    };

    Ok((list_id, file_index))
}

fn get_tags_from_request(request: &mut Request) -> Result<Vec<String>, FileRequestError>
{
    //Get the important information from the request.
    let tag_string = get_get_variable(request, "tags")?;

    match json::decode::<Vec<String>>(&tag_string){
        Ok(result) => Ok(sanitize_tag_names(&result).unwrap()),
        Err(e) => {
            Err(err_invalid_variable_type("tags", &format!("{:?}", e)))
        }
    }
}

fn get_get_variable(request: &mut Request, name: &str) -> Result<String, FileRequestError>
{
    match request.get_ref::<UrlEncodedQuery>()
    {
        Ok(hash_map) => {
            match hash_map.get(name)
            {
                Some(val) => Ok(val.first().unwrap().clone()),
                None => Err(FileRequestError::NoSuchVariable(name.to_owned()))
            }
        },
        _ => Err(FileRequestError::NoUrlEncodedQuery)
    }
}


////////////////////////////////////////////////////////////////////////////////
//                      Private functions for generating
//                      replies to file requests
////////////////////////////////////////////////////////////////////////////////

fn reply_to_file_list_request(file_list_list: Arc<Mutex<FileListList>>, id: usize)
        -> ListResponse
{
    // Fetch the file list
    let file_amount = {
        let file_list_list = file_list_list.lock().unwrap();

        match file_list_list.get(id)
        {
            Some(list) => Some(list.len()),
            None => None
        }
    };

    let result = ListResponse{ id, length: file_amount };

    result
}

/**
  Updates the specified file_list with a new FileLocation
*/
fn update_file_list(
        file_list_list: &mut Arc<Mutex<FileListList>>,
        list_id: usize,
        file_index: usize,
        new_location: FileLocation
    )
{
    let mut file_list_list = file_list_list.lock().unwrap();
    file_list_list.edit_file_list_entry(list_id, file_index, &new_location);
}

/**
  Saves the specified tags for the file. If a new `FileLocation` has been created,
  it is returned. Otherwise None. If saving failed an error is returned
*/
// TODO: Write tests for new db entries
fn handle_save_request(db: Arc<Mutex<FileDatabase>>, file_location: &FileLocation, tags: &[String])
        -> Result<FileSaveRequestResult, FileRequestError>
{
    match *file_location {
        FileLocation::Unsaved(ref path) => {
            match save_new_file(db, path, tags)
            {
                // TODO: Handle issues sent back through the channel
                Ok((db_entry, save_result_rx)) => {
                    Ok(FileSaveRequestResult::NewDatabaseEntry(
                        FileLocation::Database(db_entry),
                        save_result_rx
                    ))
                },
                Err(e) => Err(e)
            }
        },
        FileLocation::Database(ref old_file) => {
            match update_stored_file(db, old_file, tags)
            {
                Ok(db_entry) => {
                    Ok(FileSaveRequestResult::UpdatedDatabaseEntry(
                        FileLocation::Database(db_entry)
                    ))
                },
                Err(e) => Err(e)
            }
        }
    }
}

/**
  Saves a specified file in the `Filedatabase`
*/
fn save_new_file(db: Arc<Mutex<FileDatabase>>, original_path: &PathBuf, tags: &[String])
        -> Result<(file_database::File, Receiver<FileSavingResult>), FileRequestError>
{
    let file_extension = match (*original_path).extension()
    {
        Some(val) => val,
        None => return Err(FileRequestError::NoFileExtension(original_path.clone()))
    };

    //Get the folder where we want to place the stored file
    let destination_dir = {
        let db = db.lock().unwrap();

        db.get_file_save_path()
    };

    let file_identifier = get_semi_unique_identifier();

    let thumbnail_path_without_extension = format!("{}/thumb_{}", destination_dir.clone(), &file_identifier);


    //Generate the thumbnail
    let original_path_string = original_path.to_string_lossy();
    let thumbnail_info = generate_thumbnail(
                &original_path_string,
                &thumbnail_path_without_extension,
                300
            )?;

    //Copy the file to the destination
    //Get the name and path of the new file
    let new_file_path =
            destination_dir + "/" + &file_identifier + "." + &file_extension.to_string_lossy();


    let thumbnail_filename = 
            Path::new(&thumbnail_info.path).file_name().unwrap().to_str().unwrap();
    let new_filename = 
    {
        let filename = Path::new(&*new_file_path).file_name().unwrap();

        String::from(filename.to_str().unwrap())
    };


    let timestamp = get_file_timestamp(&PathBuf::from((*original_path).clone()));

    //Store the file in the database
    let saved_id = {
        let mut db = db.lock().unwrap();

        db.add_new_file(
                &new_filename.to_string(),
                &thumbnail_filename.to_string(),
                tags,
                timestamp
            )
    };

    let save_result_rx = {
        let original_path = original_path.clone();
        let new_file_path = new_file_path.clone();

        let (tx, rx) = channel();

        thread::spawn(move || {
            let save_result = match fs::copy(original_path, new_file_path)
            {
                Ok(_) => FileSavingResult::Success,
                Err(e) => FileSavingResult::Failure(e)
            };

            // We ignore any failures to send the file save result since
            // it most likely means that the caller of the save function
            // does not care about the result
            match tx.send(save_result) {
                _ => {}
            }
        });

        rx
    };

    Ok((saved_id, save_result_rx))
}


/**
  Updates a specified file in the database with new tags
*/
fn update_stored_file(db: Arc<Mutex<FileDatabase>>, old_entry: &file_database::File, tags: &[String])
    -> Result<file_database::File, FileRequestError>
{
    let db = db.lock().unwrap();
    match db.change_file_tags(old_entry, tags)
    {
        Ok(result) => Ok(result),
        Err(e) => Err(FileRequestError::DatabaseSaveError(e))
    }
}


fn file_data_from_file_location(file: &FileLocation)
        -> FileData
{
    // Lock the file list and try to fetch the file
    match *file {
        FileLocation::Unsaved(ref path) => FileData::from_path(path.clone()),
        FileLocation::Database(ref db_entry) => {
            //TODO: Make sure this data is not outdated
            FileData::from_database(db_entry.clone())
        }
    }
}

fn get_file_list_file(file: &FileLocation)
        -> PathBuf
{
    match *file {
        FileLocation::Unsaved(ref path) => path.clone(),
        FileLocation::Database(ref db_entry) => {
            PathBuf::from(db_entry.filename.clone())
        }
    }
}

fn get_file_list_thumbnail(file: &FileLocation) -> PathBuf
{
    match *file {
        FileLocation::Unsaved(ref path) => path.clone(),
        FileLocation::Database(ref db_entry) => {
            PathBuf::from(db_entry.thumbnail_path.clone())
        }
    }
}

/**
  Returns a `FileLocation` from a `FileListList`, a list id and a file id
*/
fn get_file_list_object(file_list_list: &FileListList, list_id: usize, file_index: usize)
    -> Result<FileLocation, FileRequestError>
{
    let file_list = match file_list_list.get(list_id)
    {
        Some(list) => list,
        None => {
            return Err(FileRequestError::NoSuchList(list_id));
        }
    };

    match file_list.get(file_index)
    {
        Some(file) => Ok(file.clone()),
        None => {
            Err(FileRequestError::NoSuchFile(list_id, file_index))
        }
    }
}



/*
  Database tests are not currently run because the database can't be shared
  between test threads
*/
#[cfg(test)]
mod file_request_tests
{
    use super::*;


    fn dummy_database_entry(file_path: &str, thumbnail_path: &str)
            -> file_database::File
    {
        file_database::File {
            id: 0,
            filename: file_path.to_owned(),
            thumbnail_path: thumbnail_path.to_owned(),
            creation_date: None,
            is_uploaded: true,
            tags: vec!()
        }
    }

    fn make_dummy_file_list_list() -> FileListList
    {
        let mut fll = FileListList::new();

        let flist1 = FileList::from_locations(
                vec!( FileLocation::Unsaved(PathBuf::from("l0f0"))
                    , FileLocation::Unsaved(PathBuf::from("l0f1"))
                    , FileLocation::Unsaved(PathBuf::from("l0f2"))
                    ),
                FileListSource::Search
            );

        let flist2 = FileList::from_locations(
                vec!( FileLocation::Unsaved(PathBuf::from("l1f0"))
                    , FileLocation::Unsaved(PathBuf::from("l1f1"))
                    ),
                FileListSource::Search
            );

        let flist3 = FileList::from_locations(
                vec!( FileLocation::Database(dummy_database_entry("test1", "thumb1"))
                    , FileLocation::Database(dummy_database_entry("test2", "thumb2"))
                    ),
                FileListSource::Search
            );

        fll.add(flist1);
        fll.add(flist2);
        fll.add(flist3);

        fll
    }

    #[test]
    fn file_list_object_test()
    {
        let fll = make_dummy_file_list_list();

        // Getting files from the first list works
        assert_eq!(get_file_list_object(&fll, 0, 0).unwrap(), FileLocation::Unsaved(PathBuf::from("l0f0")));
        assert_eq!(get_file_list_object(&fll, 0, 2).unwrap(), FileLocation::Unsaved(PathBuf::from("l0f2")));
        // Getting files from the second list works
        assert_eq!(get_file_list_object(&fll, 1, 1).unwrap(), FileLocation::Unsaved(PathBuf::from("l1f1")));

        //Out of bounds 
        assert!(get_file_list_object(&fll, 0, 3).is_err());
        assert!(get_file_list_object(&fll, 1, 2).is_err());
        assert!(get_file_list_object(&fll, 3, 2).is_err());
    }

    #[test]
    fn database_related_tests()
    {
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

    fn saving_a_file_without_extension_fails(fdb: Arc<Mutex<FileDatabase>>)
    {
        let tags = vec!("test1".to_owned(), "test2".to_owned());
        assert_matches!(
                save_new_file(fdb.clone(), &PathBuf::from("test"), &tags),
                Err(FileRequestError::NoFileExtension(_))
            );
    }

    fn file_list_saving_works(fdb: Arc<Mutex<FileDatabase>>)
    {
        let tags = vec!("test1".to_owned(), "test2".to_owned());

        let src_path = PathBuf::from("test/media/DSC_0001.JPG");
        let (result, save_result_rx) =
                save_new_file(fdb.clone(), &src_path, &tags).unwrap();


        let save_result = save_result_rx.recv().unwrap();
        assert_matches!(save_result, FileSavingResult::Success);

        let full_path = {
            let fdb = fdb.lock().unwrap();
            PathBuf::from(fdb.get_file_save_path())
                .join(PathBuf::from(&result.filename))
        };

        // Make sure that the saved file exists
        assert!(full_path.exists());

        //Make sure that the file was actually added to the database
        match fdb.lock()
        {
            Ok(fdb) => {
                assert!(
                    fdb.get_files_with_tags(&tags)
                        .iter()
                        .fold(false, |acc, file| { acc || file.id == result.id })
                    )
            },
            Err(e) => {
                panic!("{:?}", e)
            }
        }
    }

    fn file_list_save_requests_work(fdb: Arc<Mutex<FileDatabase>>)
    {
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
                        _ => {panic!("Unreachable branch")}
                    }
                },
                _ => {panic!("Unreachable branch")}
            }
        };

        //Make sure that the file was actually added to the database
        assert!(
                fdb.lock().unwrap().get_files_with_tags(&tags)
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
            );
    }

    fn file_list_updates_work(fdb: Arc<Mutex<FileDatabase>>)
    {
        let old_tags = vec!(String::from("old"));
        let old_location = {
            let mut fdb = fdb.lock().unwrap();
            fdb.add_new_file("test", "thumb", &old_tags, 0)
        };

        let tags = vec!("new1".to_owned());

        let saved_entry = {
            let result = handle_save_request(fdb.clone(), &FileLocation::Database(old_location), &tags);

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
                        _ => {panic!("Unreachable branch")}
                    }
                },
                _ => {panic!("Unreachable branch")}
            }
        };

        // Make sure that the file was actually added to the database
        assert!(
                fdb.lock().unwrap().get_files_with_tags(&tags)
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
            );

        // Make sure the old entry was removed
        assert!(
                fdb.lock().unwrap().get_files_with_tags(&old_tags)
                    .iter()
                    .fold(false, |acc, file| { acc || file.id == saved_entry.id })
                ==
                false
            );
    }
}
