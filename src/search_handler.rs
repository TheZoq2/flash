extern crate iron;

use std::path::PathBuf;

use iron::*;
use persistent::{Write, Read};

use file_database::FileDatabase;
use request_helpers::get_get_variable;

use file_list_response::list_info_request_handler;
use file_list::{FileLocation, FileList, FileListList, FileListSource};
use settings::Settings;
use search::{SearchType, parse_search_query, SavedSearchQuery};
use request_helpers::setup_db_connection;


pub fn handle_file_search(request: &mut Request) -> IronResult<Response> {
    // Get the important information from the request.
    let query = get_get_variable(request, "query")?;

    match parse_search_query(&query) {
        SearchType::Path(path) => handle_directory_search(request, &path),
        SearchType::Saved(query) => handle_search_for_saved_files(request, query),
    }
}


fn handle_search_for_saved_files(
    request: &mut Request,
    query: SavedSearchQuery,
) -> IronResult<Response> {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();
    let fdb = setup_db_connection(request)?;

    // Fetch the files in the database
    let files = fdb.search_files(query);

    // Build a file_list from the tags
    let file_locations = files.into_iter().map(FileLocation::Database).collect();

    let file_list_id = {
        let mut file_list_list = file_list_list.lock().unwrap();

        file_list_list.add(FileList::from_locations(
            file_locations,
            FileListSource::Search,
        ))
    };

    list_info_request_handler(file_list_list, file_list_id)
}

fn handle_directory_search(request: &mut Request, path_str: &str) -> IronResult<Response> {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();

    let file_read_path = {
        let settings = request.get::<Read<Settings>>().unwrap();

        settings.get_file_read_path()
    };

    let path = PathBuf::from(&path_str);

    // Lock the file list and insert a new list
    let file_list_id = {
        let mut file_list_list = file_list_list.lock().unwrap();

        match file_list_list.get_id_with_source(FileListSource::Folder(path.clone())) {
            Some(id) => id,
            None => file_list_list.add(FileList::from_directory(path, &file_read_path)),
        }
    };

    list_info_request_handler(file_list_list, file_list_id)
}
