use std::path::PathBuf;

use urlencoded::UrlEncodedQuery;
use rustc_serialize::json;

use iron::*;
use iron::typemap::Key;
use persistent::Write;

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
        if(self.current_index + 1 < self.files.len())
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


/**
 Handler for requests for new files in the list
*/
pub fn file_list_request_handler(request: &mut Request) -> IronResult<Response>
{
    //Get the current file list
    let mutex = request.get::<Write<FileList>>().unwrap();
    let mut file_list = mutex.lock().unwrap();

    let mut action = "current";
    //Try to find the action GET variable
    match request.get_ref::<UrlEncodedQuery>()
    {
        Ok(hash_map) => {
            match hash_map.get("action")
            {
                Some(val) => action = val.first().unwrap(),
                None => println!("No action GET variable in list request")
            }
        },
        Err(e) => println!("Failed to get GET variable: {:?}", e),
    }

    match action
    {
        "current" => {}
        "next" => file_list.select_next_file(),
        "prev" => file_list.select_prev_file(),
        other => println!("Unknown list action: {}", other),
    }

    //let response = "file/".to_string() + file_list.get_current_file().unwrap().file_name().unwrap().to_str().unwrap();
    let response = generate_response_for_file(file_list.get_current_file());

    Ok(Response::with((status::Ok, format!("{}", response))))
}

/**
    Generates a json string as a reply to a request for a file
 */
fn generate_response_for_file(path: Option<PathBuf>) -> String
{
    #[derive(RustcDecodable, RustcEncodable)]
    struct Response
    {
        status: String,
        file_path: String,
        file_type: String,
    }

    let mut response = Response{
        status: "".to_string(),
        file_path: "".to_string(),
        file_type: "image".to_string(),
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

    json::encode(&response).unwrap()
}
