
use iron::*;

use std::path::{Path};

use serde_json;

use file_util::subdirs_in_directory;



pub fn subdirectory_request_handler(_request: &mut Request, dir: &Path) -> IronResult<Response> {
    let result = subdirs_in_directory(dir)?;

    Ok(Response::with(
        (status::Ok, serde_json::to_string(&result).unwrap()),
    ))
}
