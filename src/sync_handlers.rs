use iron::*;
use persistent::Write;

use file_database::FileDatabase;

use changelog::{Change, SyncPoint};
use error::{ErrorKind, Result};
use request_helpers::{get_get_variable, get_get_i64, to_json_with_result};

use serde::Serialize;
use serde_json;
use chrono::NaiveDateTime;


////////////////////////////////////////////////////////////////////////////////
//                  Request handlers
////////////////////////////////////////////////////////////////////////////////

pub fn syncpoint_request_handler(request: &mut Request) -> IronResult<Response> {
    let mutex = request.get::<Write<FileDatabase>>().unwrap();
    let fdb = mutex.lock().unwrap();

    match handle_syncpoint_request(&fdb) {
        Ok(syncpoints) => Ok(Response::with(
                (status::Ok, serde_json::to_string(&syncpoints).unwrap()),
            )),
        Err(e) => Ok(Response::with(
            (status::InternalServerError, format!("{:?}", e)),
        ))
    }
}

pub fn change_request_handler(request: &mut Request) -> IronResult<Response> {
    let mutex = request.get::<Write<FileDatabase>>().unwrap();
    let fdb = mutex.lock().unwrap();

    let timestamp = get_get_i64(request, "starting_timestamp")?;

    let changes = handle_change_request(&fdb, timestamp)?;

    Ok(Response::with((status::Ok, to_json_with_result(changes)?)))
}

////////////////////////////////////////////////////////////////////////////////
//                  Private functions for handling requests
////////////////////////////////////////////////////////////////////////////////

fn handle_syncpoint_request(fdb: &FileDatabase) -> Result<Vec<SyncPoint>> {
    fdb.get_syncpoints()
}

fn handle_change_request(fdb: &FileDatabase, starting_timestamp: i64) -> Result<Vec<Change>> {
    let starting_time = NaiveDateTime::from_timestamp(starting_timestamp, 0);

    fdb.get_changes_after_timestamp(&starting_time)
}







