use std::path::PathBuf;

use urlencoded::UrlEncodedQuery;
use rustc_serialize::json;

use iron::*;
use iron::typemap::Key;
use persistent::{Write};

use file_database::{FileDatabaseContainer};
use file_util::{
    generate_thumbnail,
    get_file_extention,
    get_semi_unique_identifier,
    get_image_dimensions
};

use std::sync::Mutex;

use std::fs;
use std::path::Path;

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
    match action.as_str()
    {
        "current" => {}
        "next" => mutex.lock().unwrap().select_next_file(),
        "prev" => mutex.lock().unwrap().select_prev_file(),
        "save" => handle_save_request(request, &mutex),
        other => println!("Unknown list action: {}", other),
    }

    let file_list = mutex.lock().unwrap();
    let response = generate_file_list_response(file_list.get_current_file(), file_list.peak_next_file());

    Ok(Response::with((status::Ok, format!("{}", response))))
}

pub fn handle_save_request(request: &mut Request, file_list_mutex: &Mutex<FileList>)
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
        Ok(result) => sanitize_tag_names(&result).unwrap(),
        Err(e) => {
            println!("Failed to decode tag list. Error: {}", e);
            return;
        }
    };

    //Get the original filename from the File list. 
    let original_filename = {
        let file_list = file_list_mutex.lock().unwrap();

        match file_list.get_current_file()
        {
            Some(name) => name.into_os_string().into_string().unwrap(),
            None => {
                println!("Failed to save file.Crrent file is None");
                return;
            }
        }
    };
    let file_extention = get_file_extention(&original_filename);

    //Get the folder where we want to place the stored file
    let destination_dir = {
        let mutex = request.get::<Write<FileDatabaseContainer>>().unwrap();
        let db = mutex.lock().unwrap();

        db.get_saved_file_path()
    };

    let file_identifier = get_semi_unique_identifier();



    let thumbnail_path_without_extention = destination_dir.clone() + "/thumb_" + &file_identifier;


    //Generate the thumbnail
    let thumbnail_file_path = match generate_thumbnail(&original_filename, &thumbnail_path_without_extention, 300) {
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
    let new_file_path = destination_dir + "/" + &file_identifier + &file_extention;

    match fs::copy(original_filename, &new_file_path)
    {
        Ok(_) => {},
        Err(e) => {
            println!("Failed to copy file to destination: {}", e);
            //TODO: Probably remove the thumbnail here
            return
        }
    };
    

    let thumbnail_filename = Path::new(&thumbnail_file_path.path).file_name().unwrap().to_str().unwrap();
    let new_filename = Path::new(&new_file_path).file_name().unwrap().to_str().unwrap();


    //Store the file in the database
    let mutex = request.get::<Write<FileDatabaseContainer>>().unwrap();
    let mut db = mutex.lock().unwrap();

    db.add_file_to_db(&new_filename.to_string(), &thumbnail_filename.to_string(), &tags);
    db.save();
}

/**
  Checks a list of tags for unallowed characters and converts it into a storeable format,
  which at the moment is just removal of capital letters
 */
pub fn sanitize_tag_names(tag_list: &Vec<String>) -> Result<Vec<String>, String>
{
    let mut new_list = vec!();

    for tag in tag_list
    {
        if tag == ""
        {
            return Err(String::from("Tags can not be empty"));
        }

        new_list.push(tag.to_lowercase());
    }

    Ok(new_list)
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

        dimensions: (u32, u32)
    }

    let mut response = Response{
        status: "".to_string(),
        file_path: "".to_string(),
        file_type: "image".to_string(),

        next_file: "".to_string(),
        next_type: "image".to_string(),

        dimensions: (0, 0),
    };

    match path
    {
        Some(path) => {
            let filename = path.file_name().unwrap().to_str().unwrap();
            
            response.status = "ok".to_string();
            response.file_path = "file/".to_string() + &filename;
            response.file_type = "image".to_string();
            response.dimensions = get_image_dimensions(&path);
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


#[cfg(test)]
mod file_handler_tests
{
    use super::*;

    #[test]
    fn sanitize_tests()
    {
        {
            let vec = vec!(
                String::from("abCde"), 
                String::from("ABC"), 
                String::from("abc"));

            let expected = vec!(
                String::from("abcde"),
                String::from("abc"),
                String::from("abc")
                );

            assert_eq!(sanitize_tag_names(&vec), Ok(expected));
        }

        {
            assert_eq!(sanitize_tag_names(&vec!(String::from(""))), Err(String::from("Tags can not be empty")));
        }
    }
}
