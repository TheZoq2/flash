extern crate iron;

use std::path::PathBuf;

use iron::*;
use persistent::{Write, Read};

use file_database::FileDatabase;
use request_helpers::get_get_variable;

use file_list_response::reply_to_file_list_request;
use file_list::{FileLocation, FileList, FileListList, FileListSource};
use settings::Settings;
use search::{SearchType, parse_search_query};


pub fn handle_file_search(request: &mut Request) -> IronResult<Response> {
    // Get the important information from the request.
    let query = get_get_variable(request, "query")?;

    match parse_search_query(&query) {
        SearchType::Path(path) => handle_directory_search(request, &path),
        SearchType::Saved(tags) => handle_search_for_saved_files(request, tags),
    }
}


fn handle_search_for_saved_files(
    request: &mut Request,
    searched_tags: (Vec<String>, Vec<String>),
) -> IronResult<Response> {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();

    let (tags, negated_tags) = searched_tags;

    // Fetch the files in the database
    let files = {
        let mutex = request.get::<Write<FileDatabase>>().unwrap();
        let db = mutex.lock().unwrap();

        db.get_files_with_tags(&tags, &negated_tags)
    };

    // Build a file_list from the tags
    let file_locations = files.into_iter().map(FileLocation::Database).collect();

    let file_list_id = {
        let mut file_list_list = file_list_list.lock().unwrap();

        file_list_list.add(FileList::from_locations(
            file_locations,
            FileListSource::Search,
        ))
    };

    reply_to_file_list_request(file_list_list, file_list_id)
}

fn handle_directory_search(request: &mut Request, path_str: &str) -> IronResult<Response> {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();

    let starting_dir = {
        let settings = request.get::<Read<Settings>>().unwrap();

        settings.get_file_read_path()
    };

    let path = starting_dir.join(PathBuf::from(&path_str));

    // Lock the file list and insert a new list
    let file_list_id = {
        let mut file_list_list = file_list_list.lock().unwrap();

        match file_list_list.get_id_with_source(FileListSource::Folder(path.clone())) {
            Some(id) => id,
            None => file_list_list.add(FileList::from_directory(path)),
        }
    };

    reply_to_file_list_request(file_list_list, file_list_id)
}
