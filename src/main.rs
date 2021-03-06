//#![feature(associated_type_defaults)]

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
extern crate itertools;
extern crate rand;
extern crate reqwest;



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
mod sync_handlers;
mod sync_progress;
mod util;
mod file_handler;
mod byte_source;
mod foreign_server;
mod misc_handlers;

mod fix_timestamps;
mod db_fixes;

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
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}


fn perform_database_fixes(settings: &settings::Settings) {
    dotenv().ok();

    if env::var("FLASH_RUN_DB_FIXES").is_err() {
        println!("Database fixes are compiled but not enabled. run with FLASH_RUN_DB_FIXES=1 to enable");
        return;
    }
    else {
        println!("Running db fixes");
    }

    let fdb = FileDatabase::new(
            &settings.database_url,
            settings.get_file_storage_path()
        ).unwrap();
    println!("Deduplicating tags");
    db_fixes::deduplicate_tags(&fdb).expect("Failed to deduplicate tags");

    println!("creating changes for existing files");
    let current_time = chrono::NaiveDateTime::from_timestamp(chrono::offset::Utc::now().timestamp(), 0);
    db_fixes::create_changes_for_files(&fdb, &current_time).expect("Failed to create changes from files");
    println!("Done");
}


fn main() {
    let settings = settings::Settings::from_env();

    perform_database_fixes(&settings);

    //Loading or creating the database


    // Read the persistent file list if it exists
    let file_list_save_path = settings
        .get_file_storage_path()
        .join(&PathBuf::from("file_list_lists.json"));

    let file_list_list = {
        let db = FileDatabase::new(
            &settings.database_url,
            settings.get_file_storage_path()
        ).unwrap();
        persistent_file_list::read_file_list_list(&file_list_save_path, &db).unwrap()
    };

    let file_list_worker_commander = file_list_worker::start_worker(file_list_save_path);

    let file_read_path = settings.get_file_read_path();

    let (sync_tx, sync_rx, sync_storage) = sync_progress::setup_progress_datastructures();
    sync_progress::run_sync_tracking_thread(sync_rx, sync_storage.clone());

    let port = settings.get_port();

    let mut mount = Mount::new();

    let sync_tx1 = sync_tx.clone();
    let sync_handler = move |request: &mut Request| sync_handlers::sync_handler(port, request, &sync_tx1);

    mount.mount("/", Static::new(Path::new("frontend/output")));
    mount.mount("/list", file_request_handlers::file_list_request_handler);
    mount.mount("/search", search_handler::handle_file_search);
    mount.mount("sync/sync", sync_handler);
    mount.mount("sync/syncpoints", sync_handlers::syncpoint_request_handler);
    mount.mount("sync/syncpoints/add", sync_handlers::syncpoint_add_handler);
    mount.mount("sync/file_details", sync_handlers::file_detail_handler);
    mount.mount("sync/file", sync_handlers::file_request_handler);
    mount.mount("sync/thumbnail", sync_handlers::thumbnail_request_handler);
    mount.mount("sync/changes", sync_handlers::change_request_handler);
    mount.mount("sync/apply_changes", move |r: &mut Request| sync_handlers::change_application_handler(r, &sync_tx));
    mount.mount("sync/progress", move |r: &mut Request| sync_progress::progress_request_handler(r, &sync_storage));
    mount.mount("subdirectories", move |request: &mut Request| {
        misc_handlers::subdirectory_request_handler(request, &file_read_path)}
    );
    mount.mount("ping", misc_handlers::ping_handler);

    let mut chain = Chain::new(mount);
    chain.link(Write::<file_list::FileListList>::both(file_list_list));
    chain.link(Write::<file_list_worker::Commander>::both(file_list_worker_commander));
    chain.link(Read::<settings::Settings>::both(settings));

    let url = format!("0.0.0.0:{}", port);
    match Iron::new(chain).http(url) {
        Ok(_) => {
            println!("Server running on port {}", port);
            println!("Open http://localhost:{}/album.html", port);
        }
        Err(e) => println!("Failed to start iron: {}", e),
    }
}
