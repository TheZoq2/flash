use std::path::PathBuf;

use urlencoded::UrlEncodedQuery;
use rustc_serialize::json;

use std::thread;
use iron::*;
use persistent::{Write};
use std::option::Option;

use file_database::{FileDatabase};

use file_util::{
    generate_thumbnail,
    get_file_extension,
    get_semi_unique_identifier,
    get_file_timestamp,
};

use std::sync::{Mutex};

use std::fs;
use std::path::Path;

use std::ops::Deref;

use file_list::{FileList, FileListList};
use file_util::{sanitize_tag_names};

pub fn reply_to_file_list_request(id: usize) -> IronResult<Response>
{

    #[derive(Serialize)]
    struct ListResponse
    {
        pub id: usize,
        pub file_list: Option<FileList>
    }

    // Fetch the file list
    let mutex = request.get::<Wirte<FileListList>>().unwrap();
    let file_list_list = mutex.lock().unwrap();

    let result = ListResponse{ id, file_list: file_list_list.get(id)};

    Ok(Response::with((status::Ok, format!("{}", serde_json::to_string(result)))))
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
            unimplemented!()
        }
    };

    // Check if path is a valid path
    let path = PathBuf::from(&path);

    // Lock the file list and insert a new list
    let file_list_id = {
        let mutex = request.get::<Wirte<FileListList>>().unwrap();
        let file_list = mutex.lock().unwrap();

        file_list_list.add(FileList::from_directory(path))
    };

    reply_to_file_list_request(file_list_id)
}

/**
 Handler for requests for new files in the list
*/
pub fn file_list_request_handler(request: &mut Request) -> IronResult<Response>
{
    let action = match get_get_variable(request, "action".to_string())
    {
        Some(val) => val,
        None => {
            println!("Action not part of GET for request. Assuming 'current'");
            "current".to_string()
        }
    };

    //Get the current file list
    let mutex = request.get::<Write<FileList>>().unwrap();
    match action.as_str()
    {
        "current" => {}
        "next" => mutex.lock().unwrap().select_next_file(),
        "prev" => mutex.lock().unwrap().select_prev_file(),
        "save" => handle_save_request(request, &mutex),
        other => println!("Unknown list action: {}", other),
    }

    let filename = {
        let file_list = mutex.lock().unwrap();
        file_list.get_current_file()
    };

    let response = {
        let mutex = request.get::<Write<FileDatabase>>().unwrap();
        let db = mutex.lock().unwrap();
        generate_file_list_response(filename, &db.deref())
    };

    Ok(Response::with((status::Ok, format!("{}", response))))
}

pub fn handle_save_request(request: &mut Request, file_list_mutex: &Mutex<FileList>)
{
    //Get the original filename from the File list. 
    let (original_filename, old_id) = {
        let file_list = file_list_mutex.lock().unwrap();

        let filename = match file_list.get_current_file()
        {
            Some(file) => file.path.into_os_string().into_string().unwrap(),
            None => {
                println!("Failed to save file.Crrent file is None");
                return;
            }
        };

        (filename, file_list.get_current_file_save_id())
    };

    let file_extension = get_file_extension(&original_filename);

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
    let thumbnail_info = match generate_thumbnail(&original_filename, &thumbnail_path_without_extension, 300) {
        Ok(val) => val,
        Err(e) => {
            //TODO: The user needs to be alerted when this happens
            //TODO: Also, test this
            println!("Failed to generate thumbnail: {}", e); 
            return;
        }
    };

    //Copy the file to the destination
    //Get the name and path of the new file
    let new_file_path = destination_dir + "/" + &file_identifier + &file_extension;


    let thumbnail_filename = 
            Path::new(&thumbnail_info.path).file_name().unwrap().to_str().unwrap();
    let new_filename = 
    {
        let filename = Path::new(&new_file_path).file_name().unwrap();

        String::from(filename.to_str().unwrap())
    };


    let timestamp = get_file_timestamp(&PathBuf::from(original_filename.clone()));

    match old_id
    {
        Some(id) =>
        {
            //Modify the old image
            let mutex = request.get::<Write<FileDatabase>>().unwrap();
            let mut db = mutex.lock().unwrap();

            let file = db.get_file_with_id(id);

            if file.is_some()
            {
                db.change_file_tags(file.unwrap(), &tags);
            }
        }
        None =>
        {
            let saved_id;
            //Store the file in the database
            {
                let mutex = request.get::<Write<FileDatabase>>().unwrap();
                let mut db_container = mutex.lock().unwrap();

                saved_id = db_container.add_new_file(
                        &new_filename.to_string(),
                        &thumbnail_filename.to_string(),
                        &tags,
                        timestamp
                    ).id;
            }

            //Remember that the current image has been saved
            {
                let mut file_list = file_list_mutex.lock().unwrap();
                file_list.mark_current_file_as_saved(saved_id);
            }
        }
    }

    thread::spawn(move ||{
        match fs::copy(original_filename, new_file_path)
        {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to copy file to destination: {}", e);
                //TODO: Probably remove the thumbnail here
                return
            }
        };
    });
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
/**
    Generates a json string as a reply to a request for a file
 */
fn generate_file_list_response(file: Option<File>, db: &FileDatabase) -> String
{
    /**
      Helper class for generating json data about the current files
     */
    #[derive(RustcDecodable, RustcEncodable)]
    struct Response
    {
        status: String,
        file_path: String,
        file_type: String,

        tags: Vec<String>,
        old_id: Option<i32>
    }

    let mut response = Response{
        status: "".to_string(),
        file_path: "".to_string(),
        file_type: "image".to_string(),

        tags: vec!(),
        old_id: None
    };

    match file
    {
        Some(file_obj) => {
            let path = file_obj.path;
            let filename = path.file_name().unwrap().to_str().unwrap();
            
            response.status = "ok".to_string();
            response.file_path = "file/".to_string() + &filename;
            response.file_type = "image".to_string();

            match file_obj.saved_id
            {
                Some(id) => {
                    //Fetch the data about the image in the database
                    response.tags = db.get_file_with_id(id).unwrap().tags.clone();
                    response.old_id = Some(id);
                },
                None => {}
            }
        },
        None => response.status = "no_file".to_string(),
    }

    let result = json::encode(&response).unwrap();

    result
}
