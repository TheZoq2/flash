use std::path::PathBuf;

use urlencoded::UrlEncodedQuery;
use rustc_serialize::json;

use serde_json;

use iron::*;
use persistent::{Write};
use std::option::Option;
use std::fs;

use std::thread;
use std::sync::{Mutex};
use std::sync::Arc;

use std::path::Path;

use file_database;
use file_database::{FileDatabase};
use file_list::{FileList, FileListList, FileListSource, FileLocation};
use file_util::{sanitize_tag_names};
use file_util::{
    generate_thumbnail,
    get_file_extension,
    get_semi_unique_identifier,
    get_file_timestamp
};


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

pub fn reply_to_file_list_request(request: &mut Request, id: usize) -> IronResult<Response>
{
    #[derive(Serialize)]
    struct ListResponse
    {
        pub id: usize,
        pub length: Option<usize>
    }

    // Fetch the file list
    let file_amount = {
        let mutex = request.get::<Write<FileListList>>().unwrap();
        let file_list_list = mutex.lock().unwrap();

        match file_list_list.get(id)
        {
            Some(list) => Some(list.len()),
            None => None
        }
    };

    let result = ListResponse{ id, length: file_amount };

    Ok(Response::with((status::Ok, format!("{}", serde_json::to_string(&result).unwrap()))))
}

/**
  Handles requests for creating a filelist from a directory path
*/
pub fn directory_list_handler(request: &mut Request) -> IronResult<Response>
{
    let path = match get_get_variable(request, "path".to_string())
    {
        Some(val) => val,
        None => {
            println!("Directory list creation request did contain a path");
            //TODO: Find a better way to handle errors
            unimplemented!()
        }
    };

    // Check if path is a valid path
    let path = PathBuf::from(&path);

    // Lock the file list and insert a new list
    let file_list_id = {
        let mutex = request.get::<Write<FileListList>>().unwrap();
        let mut file_list_list = mutex.lock().unwrap();

        match file_list_list.get_id_with_source(FileListSource::Folder(path.clone()))
        {
            Some(id) => id,
            None => file_list_list.add(FileList::from_directory(path))
        }
    };

    reply_to_file_list_request(request, file_list_id)
}

pub fn reply_with_file_list_data(request: &mut Request, file: &FileLocation)
        -> IronResult<Response>
{
    // Lock the file list and try to fetch the file
    let file_data = match *file {
        FileLocation::Unsaved(path) => FileData::from_path(path),
        FileLocation::Database(id) => {
            // lock the database and fetch the file data
            let mutex = request.get::<Write<FileDatabase>>().unwrap();
            let db = mutex.lock().unwrap();

            let data = db.get_file_with_id(id);

            // TODO: Handle non-existent files
            FileData::from_database(data.unwrap())
        }
    };

    return Ok(Response::with((status::Ok, serde_json::to_string(&file_data).unwrap())))
}

fn reply_with_file_list_file(request: &mut Request, file: &FileLocation)
        -> IronResult<Response>
{
    let path = match *file {
        FileLocation::Unsaved(path) => path,
        FileLocation::Database(id) => {
            // lock the database and fetch the file data
            let mutex = request.get::<Write<FileDatabase>>().unwrap();
            let db = mutex.lock().unwrap();

            let data = db.get_file_with_id(id);

            // TODO: Handle non-existent files
            PathBuf::from(data.unwrap().filename)
        }
    };

    return Ok(Response::with((status::Ok, path)))
}

fn get_file_list_object(file_list_list: &FileListList, list_id: usize, file_index: usize)
    -> Result<FileLocation, String>
{
    let file_list = match file_list_list.get(list_id)
    {
        Some(list) => list,
        None => {
            let message = format!("No file list with id {}", list_id);
            return Err(message);
        }
    };

    match file_list.get(file_index)
    {
        Some(file) => Ok(file.clone()),
        None => {
            let message = format!("No file with index {} in file_list {}", file_index, list_id);
            Err(message)
        }
    }
}

/**
  Handles requests for actions dealing with specific entries in file lists
*/
pub fn file_list_request_handler(request: &mut Request) -> IronResult<Response>
{
    let action = match get_get_variable(request, "action".to_string()) {
        Some(val) => val,
        None => {
            return Ok(Response::with((status::NotFound, "Missing 'action' parameter")));
        }
    };

    let (list_id, file_index) = match read_request_list_id_index(request)
    {
        Ok(val) => val,
        Err(message) => {
            return Ok(Response::with((status::NotFound, message)));
        }
    };

    let file_location = {
        let mutex = request.get::<Write<FileListList>>().unwrap();
        let file_list_list = mutex.lock().unwrap();

        match get_file_list_object(&*file_list_list, list_id, file_index) {
            Ok(val) => val,
            Err(message) => {
                return Ok(Response::with((status:: NotFound, message)))
            }
        }
    };

    match action.as_str() {
        "get_data" => {
            reply_with_file_list_data(request, &file_location)
        },
        "get_file" => {
            reply_with_file_list_file(request, &file_location)
        },
        "save" => {
            match handle_save_request(request, &file_location) {
            }
        }
        val => {
            let message = format!("Unrecognised `action`: {}", val);
            Ok(Response::with((status::NotFound, message)))
        }
    }
}

pub fn handle_save_request(request: &mut Request, file_location: &FileLocation)
        -> Result<Option<FileLocation>, String>
{
    let tags = get_tags_from_request(request)?;

    match file_location {
        FileLocation::Unsaved(path) => 
            Some(FileLocation::Database(save_new_file(request, path, tags))),
        FileLocation::Database(id) => None
    }
}

pub fn save_new_file(request: &mut Request, original_path: &PathBuf, tags: Vec<String>)
        -> Result<i32, String>
{
    let original_path = Arc::new(*original_path);

    let file_extension = (*original_path).extension().unwrap();

    //Get the folder where we want to place the stored file
    let destination_dir = {
        let mutex = request.get::<Write<FileDatabase>>().unwrap();
        let db = mutex.lock().unwrap();

        db.get_file_save_path()
    };

    let file_identifier = get_semi_unique_identifier();

    let tags = get_tags_from_request(request).unwrap();

    let thumbnail_path_without_extension = format!("{}/thumb_{}", destination_dir.clone(), &file_identifier);


    //Generate the thumbnail
    let original_path_string = (*original_path).to_string_lossy();
    let thumbnail_info = match generate_thumbnail(&original_path_string, &thumbnail_path_without_extension, 300) {
        Ok(val) => val,
        Err(e) => {
            return Err(format!("Failed to generate thumbnail: {}", e));
        }
    };

    //Copy the file to the destination
    //Get the name and path of the new file
    let new_file_path = Arc::new(
            destination_dir + "/" + &file_identifier + &file_extension.to_string_lossy()
        );


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
        let mutex = request.get::<Write<FileDatabase>>().unwrap();
        let mut db_container = mutex.lock().unwrap();

        db_container.add_new_file(
                &new_filename.to_string(),
                &thumbnail_filename.to_string(),
                &tags,
                timestamp
            ).id
    };

    thread::spawn(move ||{
        match fs::copy(*original_path, *new_file_path)
        {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to copy file to destination: {}", e);
                //TODO: Probably remove the thumbnail here
                return
            }
        };
    });

    Ok(saved_id)
}

pub fn update_stored_file(id: i32, tags: Vec<String>)
{
    
}


fn read_request_list_id_index(request: &mut Request) -> Result<(usize, usize), String>
{
    let list_id = match get_get_variable(request, "list_id".to_string()) {
        Some(val) => val,
        None => {
            return Err(format!("missing list_id variable"));
        }
    };

    let list_id = match list_id.parse::<usize>() {
        Ok(val) => val,
        Err(_) => {
            return Err(format!("{} is not a valid file list id", list_id));
        }
    };

    let file_index = match get_get_variable(request, "index".to_string()) {
        Some(val) => val,
        None => {
            return Err(format!("missing index variable"));
        }
    };

    let file_index = match file_index.parse::<usize>() {
        Ok(val) => val,
        Err(_) => {
            return Err(format!("{} is not a valid list index", file_index));
        }
    };

    Ok((list_id, file_index))
}


fn get_tags_from_request(request: &mut Request) -> Result<Vec<String>, String>
{
    //Get the important information from the request.
    let tag_string = match request.get_ref::<UrlEncodedQuery>()
    {
        Ok(hash_map) => {
            match hash_map.get("tags")
            {
                Some(val) => val.first().unwrap().clone(), //The request contains a vec each occurence of the variable
                None => {
                    return Err(String::from("Failed to decode tag list. 'tags' variable not
                                        in GET ilist"));
                }
            }
        },
        Err(e) => {
            return Err(String::from(format!("Failed to get GET variable: {:?}", e)));
        }
    };

    match json::decode::<Vec<String>>(&tag_string){
        Ok(result) => Ok(sanitize_tag_names(&result).unwrap()),
        Err(e) => {
            println!("Failed to decode tag list. Error: {}", e);
            return Err(format!("{}", e));
        }
    }
}

fn get_get_variable(request: &mut Request, name: String) -> Option<String>
{
    //return Some("".to_string());
    match request.get_ref::<UrlEncodedQuery>()
    {
        Ok(hash_map) => {
            match hash_map.get(&name)
            {
                Some(val) => Some(val.first().unwrap().clone()),
                None => None
            }
        },
        _ => None
    }
}
