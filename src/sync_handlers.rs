use iron::*;
use persistent;

use file_database::FileDatabase;

use changelog::{Change, SyncPoint};
use error::{Result};
use request_helpers::{
    get_get_i64,
    to_json_with_result,
    from_json_with_result,
    get_get_variable,
    setup_db_connection
};
use settings::Settings;

use rand;
use serde_json;
use chrono::NaiveDateTime;

use std::fs::File;
use std::io::prelude::*;
use std::thread;

use foreign_server::{FileDetails, ChangeData, HttpForeignServer};
use sync::{apply_changes, sync_with_foreign};

use sync_progress as sp;


////////////////////////////////////////////////////////////////////////////////
//                  Request handlers
////////////////////////////////////////////////////////////////////////////////

pub fn sync_handler(own_port: u16, request: &mut Request, progress_tx: &sp::TxType) -> IronResult<Response> {
    let settings = request.get::<persistent::Read<Settings>>().unwrap();

    let foreign_url = get_get_variable(request, "foreign_url")?;

    let foreign_server = HttpForeignServer::new(foreign_url);
    let job_id = handle_sync_request((*settings).clone(), foreign_server, own_port, progress_tx)?;

    Ok(Response::with((status::Ok, to_json_with_result(job_id)?)))
}

pub fn syncpoint_request_handler(request: &mut Request) -> IronResult<Response> {
    let fdb = setup_db_connection(request)?;

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
    let fdb = setup_db_connection(request)?;

    let timestamp = get_get_i64(request, "starting_timestamp")?;

    let changes = handle_change_request(&fdb, timestamp)?;

    Ok(Response::with((status::Ok, to_json_with_result(changes)?)))
}

pub fn file_request_handler(request: &mut Request) -> IronResult<Response> {
    let fdb = setup_db_connection(request)?;

    let file_id = get_get_i64(request, "file_id")?;

    let file = handle_file_request(&fdb, file_id as i32)?;

    Ok(Response::with((status::Ok, file)))
}

pub fn thumbnail_request_handler(request: &mut Request) -> IronResult<Response> {
    let fdb = setup_db_connection(request)?;

    let file_id = get_get_i64(request, "file_id")?;

    let thumb = match handle_thumbnail_request(&fdb, file_id as i32)? {
        Some(bytes) => bytes,
        None => vec!()
    };

    Ok(Response::with((status::Ok, thumb)))
}

pub fn file_detail_handler(request: &mut Request) -> IronResult<Response> {

    let file_id = get_get_i64(request, "file_id")?;

    let fdb = setup_db_connection(request)?;
    let file_details = handle_file_detail_request(&fdb, file_id as i32)?;

    Ok(Response::with((status::Ok, to_json_with_result(&file_details)?)))
}

pub fn change_application_handler(request: &mut Request, progress_tx: &sp::TxType) -> IronResult<Response> {
    let settings = request.get::<persistent::Read<Settings>>().unwrap();

    let remote_ip = request.remote_addr.ip();
    let remote_port = get_get_i64(request, "port")? as u16;

    let foreign_server = HttpForeignServer::new(format!("{}:{}", remote_ip, remote_port));

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

    let job_id = handle_change_application(body, (*settings).clone(), foreign_server, progress_tx)?;
    Ok(Response::with((status::Ok, to_json_with_result(job_id)?)))
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


fn handle_change_application(
    body: String,
    settings: Settings,
    foreign: HttpForeignServer,
    progress_tx: &sp::TxType
) -> Result<usize> {
    let change_data = from_json_with_result::<ChangeData>(&body)?;


    let job_id = rand::random::<usize>();

    let progress_tx = progress_tx.clone();

    thread::spawn(move || {
        let fdb = FileDatabase::new(&settings.database_url, settings.get_file_storage_path());

        let result = fdb.and_then(|fdb| {
            apply_changes(
                &fdb,
                &foreign,
                &change_data.changes,
                &change_data.removed_files,
                &(job_id, progress_tx.clone())
            )
        })
        .and_then(|_| {
            progress_tx.send((job_id, sp::SyncUpdate::Done))
                .unwrap_or_else(|_e| println!("Failed to send done for sync job {}", job_id));
            Ok(())
        });

        if let Err(e) = result {
            progress_tx.send((job_id, sp::SyncUpdate::Error(format!("{:?}", e))))
                .expect("Failed to send error from handle_change_application worker 
                        to sync progress manager. Did it crash?");
        }
    });

    Ok(job_id)
}


fn handle_sync_request(
    settings: Settings,
    mut foreign: HttpForeignServer,
    own_port: u16,
    progress_tx: &sp::TxType
) -> Result<usize> {
    let job_id = rand::random::<usize>();

    let progress_tx = progress_tx.clone();
    thread::spawn(move || {
        let fdb = FileDatabase::new(&settings.database_url, settings.get_file_storage_path());

        let result = fdb.and_then(|fdb| {
            sync_with_foreign(
                    &fdb,
                    &mut foreign,
                    own_port,
                    &(job_id, progress_tx.clone())
                )
        });

        if let Err(e) = result {
            progress_tx.send((job_id, sp::SyncUpdate::Error(format!("{:#?}", e))))
                .expect("Failed to send error from handle_sync_request worker
                        to sync progress manager. Did it crash?");
        }
    });

    Ok(job_id)
}
