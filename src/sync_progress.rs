use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::thread;

#[derive(Debug, Serialize, Deserialize)]
pub enum SyncUpdate {
    /// Done gathering data
    GatheredData,
    /// Done sending to the foreign server. The request has the specified ID
    SentToForeign(usize),
    /// Starting to apply the change with the specified id. There are usize changes left
    StartingToApplyChange(i32, usize),
    /// Adding the change object to the database. There are usize changes left
    AddingChangeToDb(i32, usize),
    /// Removing the specified file from the database. There are usize files left to remove
    RemovingFile(i32, usize),
    /// Changes have been applied
    Done
}

pub type RxType = Receiver<(usize, SyncUpdate)>;
pub type TxType = Sender<(usize, SyncUpdate)>;
pub type StorageType = Arc<Mutex<HashMap<usize, SyncUpdate>>>;

/**
  Creates a channel for sending updates and a hash map where they can be read from
*/
pub fn setup_progress_datastructures() -> (TxType, RxType, StorageType) {
    let (tx, rx) = channel();
    let storage = Arc::new(Mutex::new(HashMap::new()));

    (tx, rx, storage)
}

/**
  Starts a tread that listens for updates on `update_rx` and inserts
  them into the corresponding spot in the hash map
*/
pub fn run_sync_tracking_thread(
    update_rx: Receiver<(usize, SyncUpdate)>,
    storage: Arc<Mutex<HashMap<usize, SyncUpdate>>>
) {
    thread::spawn(move || {
        loop {
            let (id, update) = update_rx.recv().unwrap();

            let mut storage = storage.lock().unwrap();

            storage.insert(id, update);
        }
    });
}


