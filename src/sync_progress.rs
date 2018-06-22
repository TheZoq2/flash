use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;

use request_helpers::{to_json_with_result, get_get_usize};

use iron::prelude::*;
use iron::status;

use error::{Result, ErrorKind};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SyncUpdate {
    /// Done gathering data
    GatheredData,
    /// Done sending to the foreign server. The request has the specified ID
    SentToForeign(usize),
    /// Starting to apply a new change. There are usize changes left
    StartingToApplyChange(usize),
    /// Adding a change to the database. There are usize changes left
    AddingChangeToDb(usize),
    /// Removing the specified file from the database. There are usize files left to remove
    RemovingFile(usize),
    /// The new syncpoint is being added to the db
    AddingSyncpoint,
    /// Changes have been applied
    Done,
    /// An error occured while performing sync
    Error(String)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncStatus {
    pub last_update: SyncUpdate,
    pub foreign_job_id: Option<usize>
}

pub type RxType = Receiver<(usize, SyncUpdate)>;
pub type TxType = SyncSender<(usize, SyncUpdate)>;
pub type LocalTxType = (usize, TxType);
pub type StorageType = Arc<Mutex<HashMap<usize, SyncStatus>>>;

/**
  Creates a channel for sending updates and a hash map where they can be read from
*/
pub fn setup_progress_datastructures() -> (TxType, RxType, StorageType) {
    let (tx, rx) = sync_channel(32);
    let storage = Arc::new(Mutex::new(HashMap::new()));

    (tx, rx, storage)
}

/**
  Starts a tread that listens for updates on `update_rx` and inserts
  them into the corresponding spot in the hash map
*/
pub fn run_sync_tracking_thread(
    update_rx: Receiver<(usize, SyncUpdate)>,
    storage: StorageType
) {
    thread::spawn(move || {
        loop {
            let (id, update) = update_rx.recv()
                .expect("Failed to read sync update, sender disconnected");

            let mut storage = storage.lock().unwrap();

            println!("got_update: {:?}", update);

            // Fetch the old status from storage
            let mut new_status = storage
                .get(&id)
                .map(|x| (*x).clone())
                .unwrap_or(SyncStatus{last_update: update.clone(), foreign_job_id: None});


            // Update it to reflect what has changed
            if let SyncUpdate::SentToForeign(id) = update {
                new_status.foreign_job_id = Some(id);
            }
            new_status.last_update = update;

            storage.insert(id, new_status);
        }
    });
}

pub fn progress_request_handler(request: &mut Request, storage: &StorageType)
    -> IronResult<Response> 
{
    let job_id = get_get_usize(request, "job_id")?;

    let result = handle_progress_request(job_id, storage)?;

    Ok(Response::with((status::Ok, to_json_with_result(result)?)))
}

fn handle_progress_request(job_id: usize, storage: &StorageType) -> Result<SyncStatus> {
    let storage = storage.lock().unwrap();

    match storage.get(&job_id) {
        Some(val) => Ok((*val).clone()),
        None => bail!(ErrorKind::NoSuchJobId(job_id))
    }
}

