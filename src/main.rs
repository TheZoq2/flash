#![allow(dead_code)]

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

#[cfg(test)]
#[macro_use]
extern crate assert_matches;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[cfg(test)]
#[macro_use]
mod test_macros;

mod file_list;
mod file_database;
mod settings;
mod album_handler;
mod file_util;
mod file_request_handlers;
mod file_request_error;
mod exiftool;
mod search;
mod schema;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;


use iron::*;
use staticfile::Static;
use mount::Mount;
use std::path::{Path};

use file_database::FileDatabase;

use persistent::{Write};

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

//TODO: Rewrite this comment 
/**
    Process for saving an image:

    Reserve an ID in the database and store the tags of the image in it.

    Start a worker thread that generates the data that takes a lot of time and stores
    that data in the database
 */


fn main()
{
    //let target_dir = "/mnt/1TB-files/Pictures/Oneplus".to_string();
    //let target_dir = "/mnt/1TB-files/Pictures/dslr/apr13-2017".to_string();
    let target_dir = "/home/frans/Pictures/dslr/26-may".to_string();
    //let file_list = get_files_in_dir(&target_dir);

    let settings = settings::Settings::get_defaults();

    //Loading or creating the database
    let db = FileDatabase::new(establish_connection(), settings.get_file_storage_path());

    let mut mount = Mount::new();

    mount.mount("/list", file_request_handlers::file_list_request_handler);
    mount.mount("/", Static::new(Path::new("frontend/output")));
    mount.mount("/file", Static::new(Path::new(&target_dir)));
    mount.mount("/album/image", Static::new(Path::new(&settings.get_file_storage_path())));
    mount.mount("/file_list/from_path", file_request_handlers::directory_list_handler);

    let mut chain = Chain::new(mount);
    chain.link(Write::<file_list::FileListList>::both(file_list::FileListList::new()));
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

