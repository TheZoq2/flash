use std::path::PathBuf;

use urlencoded::UrlEncodedQuery;
use rustc_serialize::json;

use iron::*;
use iron::typemap::Key;
use persistent::{Write};

use file_database::FileDatabaseContainer;

#[derive(Clone)]
pub struct FileList
{
    files: Vec<PathBuf>,
    current_index: usize,
}

impl FileList
{
    pub fn new(files: Vec<PathBuf>) -> FileList 
    {
        FileList {
            files: files,
            current_index: 0,
        }
    }

    pub fn get_current_file(&self) -> Option<PathBuf>
    {
        if self.current_index < self.files.len()
        {
            return Some(self.files[self.current_index].clone());
        }

        None
    }

    /**
      Returns the file after the current file without incrementing the current index. This can
      be used to preload the images in order to prevent the small lag when loading new images.
     */
    pub fn peak_next_file(&self) -> Option<PathBuf> 
    {
        if self.current_index + 1 < self.files.len()
        {
            return Some(self.files[self.current_index + 1].clone());
        }
        None
    }
    /**
      Increments current index by one while making sure it doesn't go too far out of bounds
     */
    pub fn select_next_file(&mut self)
    {
        self.current_index += 1;

        if self.current_index > self.files.len()
        {
            self.current_index = self.files.len();
        }
    }
    /**
      Decrements current index by one while making sure it doesn't go too far out of bounds
     */
    pub fn select_prev_file(&mut self)
    {
        if self.current_index > 1
        {
            self.current_index -= 1;
        }
    }
}

impl Key for FileList { type Value = FileList; }

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
    let mut file_list = mutex.lock().unwrap();
    match action.as_str()
    {
        "current" => {}
        "next" => file_list.select_next_file(),
        "prev" => file_list.select_prev_file(),
        "save" => handle_save_request(request, &file_list),
        other => println!("Unknown list action: {}", other),
    }

    let response = generate_file_list_response(file_list.get_current_file(), file_list.peak_next_file());

    Ok(Response::with((status::Ok, format!("{}", response))))
}

pub fn handle_save_request(request: &mut Request, file_list: &FileList)
{
    //Get the important information from the request.
    let tag_string = match request.get_ref::<UrlEncodedQuery>()
    {
        Ok(hash_map) => {
            match hash_map.get("tags")
            {
                Some(val) => val.first().unwrap().clone(), //The request contains a vec each occurence of the variable
                None => {
                    println!("Failed to save, tag list not included in the string");
                    return;
                }
            }
        },
        Err(e) => {println!("Failed to get GET variable: {:?}", e); return;}
    };

    let tags = match json::decode::<Vec<String>>(&tag_string){
        Ok(result) => result,
        Err(e) => {
            println!("Failed to decode tag list. Error: {}", e);
            return;
        }
    };

    //Get the original filename from the File list. 
    let original_filename = match file_list.get_current_file()
    {
        Some(name) => name.into_os_string().into_string().unwrap(),
        None => {
            println!("Failed to save file.Crrent file is None");
            return;
        }
    };
    //TODO: Copy the file to the propper destination and stuff

    //Store the file in the database
    let mutex = request.get::<Write<FileDatabaseContainer>>().unwrap();
    let mut db = mutex.lock().unwrap();

    db.add_file_to_db(original_filename, tags);
    db.save();
}

/**
    Generates a json string as a reply to a request for a file
 */
fn generate_file_list_response(path: Option<PathBuf>, next_path: Option<PathBuf>) -> String
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

        next_file: String,
        next_type: String,
    }

    let mut response = Response{
        status: "".to_string(),
        file_path: "".to_string(),
        file_type: "image".to_string(),

        next_file: "".to_string(),
        next_type: "image".to_string(),
    };

    match path
    {
        Some(path) => {
            let filename = path.file_name().unwrap().to_str().unwrap();
            
            response.status = "ok".to_string();
            response.file_path = "file/".to_string() + &filename;
            response.file_type = "image".to_string();
        },
        None => response.status = "no_file".to_string(),
    }

    match next_path
    {
        Some(path) =>
        {
            let filename = path.file_name().unwrap().to_str().unwrap();
            response.next_file = "file/".to_string() + &filename;
        },
        None => {}
    }

    json::encode(&response).unwrap()
}
