#![allow(dead_code)]
//#![feature(btree_range, collections_bound)]
extern crate iron;
extern crate staticfile;
extern crate persistent;
extern crate mount;
extern crate urlencoded;
extern crate image;
#[macro_use]
extern crate lazy_static;
extern crate regex;

#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate diesel_codegen;

extern crate glob;
extern crate rustc_serialize;
extern crate chrono;

mod file_list;
mod file_database;
mod settings;
mod album_handler;
mod file_util;
mod file_request_handlers;
mod exiftool;
//mod search;
mod schema;


use iron::*;
use staticfile::Static;
use mount::Mount;
use std::path::{Path, PathBuf};

use file_database::FileDatabase;

use glob::glob;

use persistent::{Write};

use std::vec::Vec;

use diesel::prelude::*;
use diesel::pg::PgConnection;

use dotenv::dotenv;
use std::env;

//Establish a connection to the postgres database
pub fn establish_connection() -> PgConnection
{
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set. Perhaps .env is missing?");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}


/**
    Process for saving an image:

    Reserve an ID in the database and store the tags of the image in it.

    Start a worker thread that generates the data that takes a lot of time and stores
    that data in the database 
 */

/**
  Returns a list of all the files in a directory
*/
fn get_files_in_dir(dir: &String) -> Vec<PathBuf> 
{
    let mut result = Vec::<PathBuf>::new();

    let full_path = dir.clone() + "/*";

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


fn main() 
{
    //let target_dir = "/mnt/1TB-files/Pictures/Oneplus".to_string();
    let target_dir = "/mnt/1TB-files/Pictures/dslr/apr13-2017".to_string();
    //let target_dir = "/home/frans/Pictures/imgtest".to_string();
    let file_list = get_files_in_dir(&target_dir);

    let settings = settings::Settings::get_defaults();

    //Loading or creating the database
    let db = FileDatabase::new(establish_connection(), settings.get_file_storage_path());

    let mut mount = Mount::new();

    mount.mount("/list", file_request_handlers::file_list_request_handler);
    mount.mount("/", Static::new(Path::new("frontend/output")));
    mount.mount("/file", Static::new(Path::new(&target_dir)));
    mount.mount("/album/image", Static::new(Path::new(&settings.get_file_storage_path())));
    mount.mount("/album", album_handler::handle_album_list_request);
    mount.mount("/album/file", album_handler::handle_album_image_request);

    let mut chain = Chain::new(mount);
    chain.link(Write::<file_list::FileList>::both(file_list::FileList::new(file_list)));
    chain.link(Write::<FileDatabase>::both(db));
    match Iron::new(chain).http("localhost:3000")
    {
        Ok(_) => {
            println!("Server running on port 3000");
            println!("Open localhost/tag_editor.html or album.html")
        },
        Err(e) => println!("Failed to start iron: {}", e)
    }
}

