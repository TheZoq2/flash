#![allow(dead_code)]
#![allow(unused_doc_comment)]

#![recursion_limit="1024"]

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
extern crate error_chain;

#[macro_use]
extern crate diesel;
extern crate dotenv;
#[macro_use]
extern crate diesel_codegen;

extern crate glob;
extern crate chrono;

// Imports used by tests
#[cfg(test)]
#[macro_use]
extern crate assert_matches;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
#[cfg(test)]
// Modules used by tests
#[macro_use]
mod test_macros;


mod file_list;
mod persistent_file_list;
mod file_list_worker;
mod file_database;
mod settings;
mod search_handler;
mod file_util;
mod file_request_handlers;
mod exiftool;
mod search;
mod date_search;
mod schema;
mod request_helpers;
mod file_list_response;
mod error;
mod changelog;
mod sync;
mod util;
mod file_handler;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;


use iron::*;
use staticfile::Static;
use mount::Mount;
use std::path::{Path, PathBuf};

use file_database::FileDatabase;

use persistent::{Write, Read};

use diesel::prelude::*;
use diesel::pg::PgConnection;

use dotenv::dotenv;
use std::env;

//Establish a connection to the postgres database
pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url =
        env::var("DATABASE_URL").expect("DATABASE_URL must be set. Perhaps .env is missing?");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}


fn main() {
    //let target_dir = "/mnt/1TB-files/Pictures/Oneplus".to_string();
    //let target_dir = "/mnt/1TB-files/Pictures/dslr/apr13-2017".to_string();
    let target_dir = "/home/frans/Pictures/dslr/26-may".to_string();
    //let file_list = get_files_in_dir(&target_dir);

    let settings = settings::Settings::from_env();

    //Loading or creating the database
    let db = FileDatabase::new(establish_connection(), settings.get_file_storage_path());



    // Read the persistent file list if it exists
    let file_list_save_path = settings
        .get_file_storage_path()
        .join(&PathBuf::from("file_list_lists.json"));
    let file_list_list =
        persistent_file_list::read_file_list_list(&file_list_save_path, &db).unwrap();

    let file_list_worker_commander = file_list_worker::start_worker(file_list_save_path);

    let port = settings.get_port();

    let mut mount = Mount::new();

    mount.mount("/list", file_request_handlers::file_list_request_handler);
    mount.mount("/", Static::new(Path::new("frontend/output")));
    mount.mount("/file", Static::new(Path::new(&target_dir)));
    mount.mount("/album/image", Static::new(Path::new(&settings.get_file_storage_path())),);
    mount.mount("/search", search_handler::handle_file_search);
    mount.mount("file_list", file_request_handlers::file_list_request_handler);

    let mut chain = Chain::new(mount);
    chain.link(Write::<file_list::FileListList>::both(file_list_list));
    chain.link(Write::<file_list_worker::Commander>::both(file_list_worker_commander));
    chain.link(Write::<FileDatabase>::both(db));
    chain.link(Read::<settings::Settings>::both(settings));

    let url = format!("0.0.0.0:{}", port);
    match Iron::new(chain).http(url) {
        Ok(_) => {
            println!("Server running on port {}", port);
            println!("Open localhost/tag_editor.html or album.html");
        }
        Err(e) => println!("Failed to start iron: {}", e),
    }
}
