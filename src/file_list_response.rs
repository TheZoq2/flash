use iron::*;
use persistent::Write;

use file_list::{FileListList, FileList, FileListSource, FileLocation};

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
            _ => None
        }
    }
}

pub enum ListAction {
    Info,
    LastSavedIndex
}
impl ListAction {
    pub fn try_parse(action_str: &str) -> Option<ListAction> {
        match action_str {
            "list_info" => Some(ListAction::Info),
            "list_last_saved_index" => Some(ListAction::LastSavedIndex),
            _ => None
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
    let file_list_list = file_list_list.lock().unwrap();

    let id = read_request_list_id(request)?;

    let file_list = match file_list_list.get(id) {
        Some(list) => Ok(list),
        None => Err(FileRequestError::NoSuchList(id))
    }?;

    match action {
        ListAction::Info => create_list_info_response(id, file_list),
        ListAction::LastSavedIndex => last_saved_request_handler(file_list)
    }
}

/**
  Handles `file_list` requests that are not concerned with specific file lists
*/
pub fn global_list_action_handler(request: &mut Request, action: GlobalAction) -> IronResult<Response> {
    let file_list_list = request.get::<Write<FileListList>>().unwrap();

    match action {
        GlobalAction::AllLists => reply_to_list_listing_request(file_list_list)
    }
}


////////////////////////////////////////////////////////////////////////////////
//                      Public request responders
////////////////////////////////////////////////////////////////////////////////

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
    id: usize,
) -> IronResult<Response> {
    let file_list_list = file_list_list.lock().unwrap();

    match file_list_list.get(id) {
        Some(file_list) => Ok(create_list_info_response(id, file_list)),
        None => Err(FileRequestError::NoSuchList(id))
    }?
}

pub fn create_list_info_response(id: usize, list: &FileList) -> IronResult<Response> {
    let list_response = ListResponse::from_file_list(id, list);

    Ok(Response::with(
        (status::Ok, serde_json::to_string(&list_response).unwrap()),
    ))
}


/**
  Returns the index of the last file that was saved to the database in a specific file
*/
fn last_saved_request_handler(file_list: &FileList) -> IronResult<Response> {
    let index = file_list.get_files().iter()
        .enumerate()
        .fold(0, |last, (id, file)| {
            match file {
                &FileLocation::Database(_) => id,
                _ => last
            }
        });

    Ok(Response::with(
        (status::Ok, serde_json::to_string(&index).unwrap())
    ))
}
