use iron::*;
use persistent::Write;

use file_database::FileDatabase;

use changelog::{Change, SyncPoint};
use error::{Result};
use request_helpers::{get_get_i64, to_json_with_result, from_json_with_result};

use serde_json;
use chrono::NaiveDateTime;

use std::fs::File;
use std::io::prelude::*;

use foreign_server::{FileDetails, ChangeData, HttpForeignServer};
use sync::apply_changes;


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

pub fn file_request_handler(request: &mut Request) -> IronResult<Response> {
    let mutex = request.get::<Write<FileDatabase>>().unwrap();
    let fdb = mutex.lock().unwrap();

    let file_id = get_get_i64(request, "file_id")?;

    let file = handle_file_request(&fdb, file_id as i32)?;

    Ok(Response::with((status::Ok, file)))
}

pub fn thumbnail_request_handler(request: &mut Request) -> IronResult<Response> {
    let mutex = request.get::<Write<FileDatabase>>().unwrap();
    let fdb = mutex.lock().unwrap();

    let file_id = get_get_i64(request, "file_id")?;

    let thumb = match handle_thumbnail_request(&fdb, file_id as i32)? {
        Some(bytes) => bytes,
        None => vec!()
    };

    Ok(Response::with((status::Ok, thumb)))
}

pub fn file_detail_handler(request: &mut Request) -> IronResult<Response> {

    let file_id = get_get_i64(request, "file_id")?;

    let mutex = request.get::<Write<FileDatabase>>().unwrap();
    let fdb = mutex.lock().unwrap();
    let file_details = handle_file_detail_request(&fdb, file_id as i32)?;

    Ok(Response::with((status::Ok, to_json_with_result(&file_details)?)))
}

pub fn change_application_handler(request: &mut Request) -> IronResult<Response> {
    let mutex = request.get::<Write<FileDatabase>>().unwrap();
    let fdb = mutex.lock().unwrap();

    let remote_addr = request.remote_addr;
    let remote_str = format!("{}", remote_addr);

    let mut body = String::new();
    match request.body.read_to_string(&mut body) {
        Ok(_) => {},
        Err(e) => {
            return Ok(Response::with((
                status::PreconditionFailed,
                format!("Failed to read body {:?}", e)
            )));
        }
    }

    handle_chage_application(body, &fdb, &HttpForeignServer::new(remote_str))?;
    Ok(Response::with((status::Ok, "")))
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


fn handle_file_request(fdb: &FileDatabase, id: i32) -> Result<Vec<u8>> {
    // Fetch the filename from the database
    let filename = fdb.get_file_with_id_result(id)?.filename;

    // Join the filename with the save path from the fdb
    let file_path = fdb.get_file_save_path().join(filename);

    // Open the file for reading
    let mut file = File::open(file_path)?;
    let mut content = vec!();
    file.read_to_end(&mut content)?;

    Ok(content)
}

fn handle_thumbnail_request(fdb: &FileDatabase, id: i32) -> Result<Option<Vec<u8>>> {
    let thumbnail_path = fdb.get_file_with_id_result(id)?.thumbnail_path;

    if let Some(thumbnail_path) = thumbnail_path {
        let mut file = File::open(fdb.get_file_save_path().join(thumbnail_path))?;
        let mut content = vec!();
        file.read_to_end(&mut content)?;

        Ok(Some(content))
    }
    else {
        Ok(None)
    }
}

fn handle_file_detail_request(fdb: &FileDatabase, id: i32) -> Result<FileDetails> {
    let file = fdb.get_file_with_id_result(id)?;

    Ok(FileDetails::from(&file))
}


fn handle_chage_application(body: String, fdb: &FileDatabase, foreign: &HttpForeignServer) -> Result<()> {
    let change_data = from_json_with_result::<ChangeData>(&body)?;

    apply_changes(&fdb, foreign, &change_data.changes, &change_data.removed_files)?;
    Ok(())

}
