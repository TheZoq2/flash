use iron::*;
use persistent::Write;

use file_list::{FileListList, FileList, FileListSource};

use std::sync::{Arc, Mutex};

use serde_json;

use file_request_error::{FileRequestError, err_invalid_variable_type};

use request_helpers::get_get_variable;

////////////////////////////////////////////////////////////////////////////////
//                      Request action types
////////////////////////////////////////////////////////////////////////////////
pub enum GlobalAction {
    AllLists,
}

impl GlobalAction {
    pub fn try_parse(action_str: &str) -> Option<GlobalAction> {
        match action_str {
            "lists" => Some(GlobalAction::AllLists),
            other => None
        }
    }
}

pub enum ListAction {
    ListInfo,
}
impl ListAction {
    pub fn try_parse(action_str: &str) -> Option<ListAction> {
        match action_str {
            "list_info" => Some(ListAction::ListInfo),
            other => None
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
//                      Internal types
////////////////////////////////////////////////////////////////////////////////

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

////////////////////////////////////////////////////////////////////////////////
//                      Public request handlers
////////////////////////////////////////////////////////////////////////////////

/**
  Handles requests for data about specific file lists
*/
pub fn list_action_handler(request: &mut Request, action: ListAction) -> IronResult<Response> {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();
    let id = read_request_list_id(request)?;

    match action {
        ListAction::ListInfo => list_info_request_handler(file_list_list, id)
    }
}

/**
  Handles file_list requests that are not concerned with specific file lists
*/
pub fn global_list_action_handler(request: &mut Request, action: GlobalAction) -> IronResult<Response> {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();

    match action {
        GlobalAction::AllLists => reply_to_list_listing_request(file_list_list)
    }
}

////////////////////////////////////////////////////////////////////////////////
//                      Private request parsers
////////////////////////////////////////////////////////////////////////////////

pub fn read_request_list_id(request: &mut Request) -> Result<usize, FileRequestError> {
    let list_id = get_get_variable(request, "list_id")?;

    match list_id.parse::<usize>() {
        Ok(val) => Ok(val),
        Err(_) => Err(err_invalid_variable_type("list_id", "usize")),
    }
}


////////////////////////////////////////////////////////////////////////////////
//                      Private helpers
////////////////////////////////////////////////////////////////////////////////

pub fn list_info_request_handler(
    file_list_list: Arc<Mutex<FileListList>>,
    file_list_id: usize,
) -> IronResult<Response> {
    let list_response = create_file_list_response(file_list_list, file_list_id);
    Ok(Response::with(
        (status::Ok, serde_json::to_string(&list_response).unwrap()),
    ))
}



fn create_file_list_response(file_list_list: Arc<Mutex<FileListList>>, id: usize) -> Option<ListResponse> {
    // Fetch the file list
        let file_list_list = file_list_list.lock().unwrap();

        file_list_list.get(id).map(|list| {
            ListResponse::from_file_list(id, list)
        })
}


pub fn reply_to_list_listing_request(file_list_list: Arc<Mutex<FileListList>>) -> IronResult<Response> {
    let file_list_list = file_list_list.lock().unwrap();

    let lists: Vec<ListResponse> = file_list_list.lists_with_ids().iter()
        .map(|&(id, list)| {
            ListResponse::from_file_list(id, list)
        }).collect();

    Ok(Response::with(
        (status::Ok, serde_json::to_string(&lists).unwrap())
    ))
}

