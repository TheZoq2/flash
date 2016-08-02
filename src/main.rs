extern crate iron;
extern crate staticfile;
extern crate persistent;
extern crate mount;
extern crate urlencoded;

extern crate glob;

//use std::env::args;

use iron::*;
use iron::typemap::Key;
use staticfile::Static;
use mount::Mount;
use std::path::{Path, PathBuf};
use urlencoded::UrlEncodedQuery;

use glob::glob;

use persistent::Write;

use std::vec::Vec;

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

    //Returns the next file
    pub fn select_next_file(&mut self)
    {
        self.current_index += 1;

        if self.current_index > self.files.len()
        {
            self.current_index = self.files.len();
        }
    }
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
  Returns a list of all the files in a directory
 */
fn get_files_in_dir(dir: String) -> Vec<PathBuf> 
{
    let mut result = Vec::<PathBuf>::new();

    let full_path = dir + "/*";

    for entry in glob(&full_path).expect("Failed to read glob")
    {
        match entry
        {
            Ok(path) => result.push(path),
            Err(e) => println!("{}", e)
        }
    }

    result
}

/**
 Handler for requests for new files in the list
*/
fn file_list_request_handler(request: &mut Request) -> IronResult<Response>
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

    let response = "file/".to_string() + file_list.get_current_file().unwrap().file_name().unwrap().to_str().unwrap();

    Ok(Response::with((status::Ok, format!("{}", response))))
}

fn hello_world(_: &mut Request) -> IronResult<Response>
{
    Ok(Response::with((status::Ok, "hello, world")))
}

fn main() {
    let target_dir = "/mnt/1TB-files/Pictures/Oneplus".to_string();
    let file_list = get_files_in_dir(target_dir.clone());

    //let mut chain = Chain::new(hello_world);
    println!("Running server on port 3000");

    let mut mount = Mount::new();
    
    mount.mount("/hello", hello_world);
    mount.mount("/list", file_list_request_handler);
    mount.mount("/", Static::new(Path::new("files/")));
    mount.mount("/file", Static::new(Path::new(&target_dir)));

    let mut chain = Chain::new(mount);
    chain.link(Write::<FileList>::both(FileList::new(file_list)));
    //mount.mount("/", Static::new(Path::new("files/index.html")));
    Iron::new(chain).http("localhost:3000").unwrap();
}

