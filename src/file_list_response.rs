use iron::*;
use persistent::Write;

use file_list::{FileListList, FileList, FileListSource};

use std::sync::{Arc, Mutex};

use serde_json;

/**
  Serializable list response that contains data about a file list
*/
#[derive(Serialize)]
struct ListResponse {
    pub id: usize,
    pub length: usize,
    pub source: FileListSource
}

impl ListResponse
{
    pub fn from_file_list(id: usize, file_list: &FileList) -> ListResponse {
        ListResponse {
            id,
            length: file_list.len(),
            source: file_list.get_source().clone()
        }
    }
}

fn create_file_list_response(file_list_list: Arc<Mutex<FileListList>>, id: usize) -> Option<ListResponse> {
    // Fetch the file list
        let file_list_list = file_list_list.lock().unwrap();

        file_list_list.get(id).map(|list| {
            ListResponse::from_file_list(id, list)
        })
}

pub fn reply_to_file_list_request(
    file_list_list: Arc<Mutex<FileListList>>,
    file_list_id: usize,
) -> IronResult<Response> {
    let list_response = create_file_list_response(file_list_list, file_list_id);
    Ok(Response::with(
        (status::Ok, serde_json::to_string(&list_response).unwrap()),
    ))
}


// TODO: This should probably be moved to a more sensible file
pub fn file_list_listing_handler(request: &mut Request) -> IronResult<Response> {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();

    reply_to_list_listing_request(file_list_list)
}

pub fn reply_to_list_listing_request(file_list_list: Arc<Mutex<FileListList>>) -> IronResult<Response> {
    let file_list_list = file_list_list.lock().unwrap();

    let lists: Vec<ListResponse> = file_list_list.lists_with_ids().iter()
        .map(|id_list| {
            let (id, list) = *id_list;
            ListResponse::from_file_list(id, list)
        }).collect();

    Ok(Response::with(
        (status::Ok, serde_json::to_string(&lists).unwrap())
    ))
}
