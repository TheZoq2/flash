#![allow(dead_code)]
extern crate iron;
extern crate staticfile;
extern crate persistent;
extern crate mount;
extern crate urlencoded;
extern crate image;

extern crate glob;
extern crate rustc_serialize;

mod file_list;
mod file_database;
mod settings;
mod album_handler;
mod file_util;

//use std::env::args;

use iron::*;
use staticfile::Static;
use mount::Mount;
use std::path::{Path, PathBuf};

use glob::glob;

use persistent::{Write};

use std::vec::Vec;

use file_database::{FileDatabaseContainer};

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

fn hello_world(_: &mut Request) -> IronResult<Response>
{
    Ok(Response::with((status::Ok, "hello, world")))
}


fn main() {
    //let target_dir = "/mnt/1TB-files/Pictures/Oneplus".to_string();
    let target_dir = "/home/frans/Pictures/imgtest".to_string();
    let file_list = get_files_in_dir(target_dir.clone());

    let settings = settings::Settings::get_defaults();

    //Loading or creating the database
    //let database = FileDatabase::load_from_json(&settings);
    let db = FileDatabaseContainer::new(&settings);

    let mut mount = Mount::new();

    mount.mount("/hello", hello_world);
    mount.mount("/list", file_list::file_list_request_handler);
    mount.mount("/", Static::new(Path::new("files/")));
    mount.mount("/file", Static::new(Path::new(&target_dir)));
    mount.mount("/album/image", Static::new(Path::new(&settings.get_file_storage_path())));
    mount.mount("/album", album_handler::handle_album_list_request);

    let mut chain = Chain::new(mount);
    chain.link(Write::<file_list::FileList>::both(file_list::FileList::new(file_list)));
    chain.link(Write::<file_database::FileDatabaseContainer>::both(db));
    //mount.mount("/", Static::new(Path::new("files/index.html")));
    match Iron::new(chain).http("localhost:3000")
    {
        Ok(_) => println!("Server running on port 3000"),
        Err(e) => println!("Failed to start iron: {}", e)
    }
}

