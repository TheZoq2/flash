use iron::*;

use file_list::FileListList;

use std::sync::{
    Arc,
    Mutex
};

use serde_json;

/**
  Serializable list response that contains data about a file list
*/
#[derive(Serialize)]
struct ListResponse
{
    pub id: usize,
    pub length: Option<usize>
}


fn create_file_list_response(file_list_list: Arc<Mutex<FileListList>>, id: usize)
        -> ListResponse
{
    // Fetch the file list
    let file_amount = {
        let file_list_list = file_list_list.lock().unwrap();

        match file_list_list.get(id)
        {
            Some(list) => Some(list.len()),
            None => None
        }
    };

    ListResponse{ id, length: file_amount }
}

pub fn reply_to_file_list_request(file_list_list: Arc<Mutex<FileListList>>, file_list_id: usize)
    -> IronResult<Response>
{
    let list_response = create_file_list_response(file_list_list, file_list_id);
    Ok(Response::with((status::Ok, serde_json::to_string(&list_response).unwrap())))
}
