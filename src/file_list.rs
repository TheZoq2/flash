use std::path::PathBuf;

use urlencoded::UrlEncodedQuery;
use rustc_serialize::json;

use std::thread;
use iron::*;
use iron::typemap::Key;
use persistent::{Write};
use std::option::Option;

use file_database::{FileDatabaseContainer, FileDatabase};
use file_util::{
    generate_thumbnail,
    get_file_extention,
    get_semi_unique_identifier,
    get_image_dimensions,
    get_file_timestamp,
};

use std::sync::Mutex;

use std::fs;
use std::path::Path;

use std::ops::Deref;

#[derive(Clone)]
pub struct File
{
    pub path: PathBuf,
    pub saved_id: Option<usize>
}

#[derive(Clone)]
pub struct FileList
{
    files: Vec<File>,
    current_index: usize,
}

impl FileList
{
    pub fn new(file_paths: Vec<PathBuf>) -> FileList 
    {
        let mut files = vec!();
        for path in file_paths
        {
            files.push(File{path:path, saved_id: None})
        }
        
        FileList {
            files: files,
            current_index: 0,
        }
    }

    pub fn get_current_file(&self) -> Option<File>
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
    pub fn peak_next_file(&self) -> Option<File> 
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

    pub fn mark_current_file_as_saved(&mut self, db_id: usize)
    {
        if self.current_index < self.files.len()
        {
            self.files[self.current_index].saved_id = Some(db_id);
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

    let filename = {
        let file_list = mutex.lock().unwrap();
        file_list.get_current_file()
    };

    let response = {
        let mutex = request.get::<Write<FileDatabaseContainer>>().unwrap();
        let db = mutex.lock().unwrap();
        generate_file_list_response(filename, &db.deref().get_db())
    };

    Ok(Response::with((status::Ok, format!("{}", response))))
}

pub fn handle_save_request(request: &mut Request, file_list_mutex: &Mutex<FileList>)
{
    //Get the original filename from the File list. 
    let original_filename = {
        let file_list = file_list_mutex.lock().unwrap();

        match file_list.get_current_file()
        {
            Some(file) => file.path.into_os_string().into_string().unwrap(),
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

    let tags = get_tags_from_request(request).unwrap();

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

    let file_path_clone = new_file_path.clone();
    let original_filename_clone = original_filename.clone();
    thread::spawn(move ||{
        match fs::copy(original_filename_clone, &file_path_clone)
        {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to copy file to destination: {}", e);
                //TODO: Probably remove the thumbnail here
                return
            }
        };
    });

    let thumbnail_filename = Path::new(&thumbnail_file_path.path).file_name().unwrap().to_str().unwrap();
    let new_filename = Path::new(&new_file_path).file_name().unwrap().to_str().unwrap();

    let timestamp = get_file_timestamp(&PathBuf::from(&original_filename));


    let saved_id;
    //Store the file in the database
    {
        let mutex = request.get::<Write<FileDatabaseContainer>>().unwrap();
        let mut db = mutex.lock().unwrap();

        saved_id = db.add_file_to_db(&new_filename.to_string(), &thumbnail_filename.to_string(), &tags, timestamp);
        db.save();
    }

    //Remember that the current image has been saved
    {
        let mut file_list = file_list_mutex.lock().unwrap();
        file_list.mark_current_file_as_saved(saved_id);
    }
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

        dimensions: (u32, u32),

        tags: Vec<String>,
        old_id: Option<usize>
    }

    let mut response = Response{
        status: "".to_string(),
        file_path: "".to_string(),
        file_type: "image".to_string(),

        dimensions: (0, 0),
        
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
            response.dimensions = get_image_dimensions(&path);

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
