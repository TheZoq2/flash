extern crate iron;

use iron::*;
use persistent::{Write};

use file_database::FileDatabase;
use request_helpers::get_get_variable;

use file_list_response::{reply_to_file_list_request};

use file_list::{
    FileLocation,
    FileList,
    FileListList,
    FileListSource
};

use search;


pub fn handle_image_search(request: &mut Request) -> IronResult<Response>
{
    let file_list_list = request.get::<Write<FileListList>>().unwrap();
    // Get the important information from the request.
    let query = get_get_variable(request, "query")?;

    let (tags, negated_tags) = search::get_tags_from_query(&query);

    println!("tags: {:?}, {:?}", tags, negated_tags);

    // Fetch the files in the database
    let files = {
        let mutex = request.get::<Write<FileDatabase>>().unwrap();
        let db = mutex.lock().unwrap();

        db.get_files_with_tags(&tags, &negated_tags)
    };

    // Build a file_list from the tags
    let file_locations = files.into_iter()
        .map(FileLocation::Database)
        .collect();

    let file_list_id = {
        let mut file_list_list = file_list_list.lock().unwrap();

        file_list_list.add(FileList::from_locations(file_locations, FileListSource::Search))
    };

    reply_to_file_list_request(file_list_list, file_list_id)
}
